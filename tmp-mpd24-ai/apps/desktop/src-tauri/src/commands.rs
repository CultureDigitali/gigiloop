use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use mpd24_ai_audio_engine::{
    demo::demo_kit, load_wav_mono, AudioCommand, AudioDeviceInfo, AudioEngineHandle,
    PadPlaybackParams, PadSample, VelocityLayer,
};
use mpd24_ai_common::{
    ControllerProfile, MidiEvent, MidiEventKind, MidiPortInfo, MidiStatus, MidiValue,
};
use mpd24_ai_groove_engine::{
    active_hit_count, best_local_candidate, generate_candidates, GrooveOperation, GrooveParams,
};
use mpd24_ai_jev_engine::{
    DirectorIntensity, JevClient, JevConfig, JevDirectorDecision, JevRerankDecision, JevStatus,
    PerformanceDirection,
};
use mpd24_ai_midi_engine::profile::{profile_file_name, save_profile};
use mpd24_ai_project_store::{
    default_kit, AiFeedbackInput, KitPadSnapshot, PerformanceScene, ProjectSnapshot, ProjectStore,
    ProjectStoreError, ProjectSummary, StoredProject, VelocityLayerSnapshot,
    MAX_ROUND_ROBIN_VARIANTS, MAX_VELOCITY_LAYERS, PROJECT_SCHEMA_VERSION,
};
use mpd24_ai_sequencer::{LaunchQuantization, Pattern, PatternBank, PatternBankStatus};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::state::{AiPreviewFeedback, AppState};

const HYBRID_CANDIDATE_COUNT: usize = 6;
const JEV_MIN_CONFIDENCE: f32 = 0.55;
const JEV_MIN_SAFE_TO_PREVIEW: f32 = 0.50;
const JEV_DIRECTOR_MIN_CONFIDENCE: f32 = 0.50;
const JEV_DIRECTOR_MIN_WORTH_CHANGING: f32 = 0.50;
const MAX_IMPORTED_SAMPLE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HybridGroovePreview {
    pub pattern: Pattern,
    pub provider: String,
    pub selected_id: String,
    pub candidate_count: usize,
    pub local_score: f32,
    pub jev: Option<JevDecisionSummary>,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JevDecisionSummary {
    pub model: String,
    pub confidence: f32,
    pub safe_to_preview: f32,
    pub input_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectorPlanSummary {
    pub operation: GrooveOperation,
    pub direction: PerformanceDirection,
    pub intensity: DirectorIntensity,
    pub provider: String,
    pub confidence: Option<f32>,
    pub worth_changing: Option<f32>,
    pub model: Option<String>,
    pub memory_events: u64,
    pub memory_accepted: u64,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectedGroovePreview {
    pub preview: Option<HybridGroovePreview>,
    pub plan: DirectorPlanSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternLaunchResult {
    pub pattern: Pattern,
    pub status: PatternBankStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectInput {
    pub id: Option<i64>,
    pub name: String,
    pub launch_quantization: LaunchQuantization,
    pub master_gain: f32,
    pub groove_params: GrooveParams,
    pub scenes: Vec<Option<PerformanceScene>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedProject {
    pub stored: StoredProject,
    pub pattern: Pattern,
    pub bank_status: PatternBankStatus,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPadSample {
    pub pad: usize,
    pub state: KitPadSnapshot,
    pub sample_rate: u32,
    pub frame_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleLibraryItem {
    pub file: String,
    pub display_name: String,
    pub bytes: u64,
}

#[tauri::command]
pub fn list_midi_inputs(state: State<'_, AppState>) -> Result<Vec<MidiPortInfo>, String> {
    state
        .midi
        .lock()
        .map_err(|_| "MIDI state lock poisoned".to_string())?
        .list_inputs()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn connect_midi_input(
    app: AppHandle,
    state: State<'_, AppState>,
    port_id: String,
) -> Result<MidiPortInfo, String> {
    let event_app = app.clone();
    let mut midi = state
        .midi
        .lock()
        .map_err(|_| "MIDI state lock poisoned".to_string())?;

    let port = midi
        .connect(&port_id, move |event| {
            let _ = event_app.emit("midi://event", event);
        })
        .map_err(|error| error.to_string())?;

    let _ = app.emit(
        "midi://status",
        MidiStatus {
            connected: true,
            port: Some(port.clone()),
            message: "Connected".into(),
        },
    );

    Ok(port)
}

#[tauri::command]
pub fn disconnect_midi_input(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state
        .midi
        .lock()
        .map_err(|_| "MIDI state lock poisoned".to_string())?
        .disconnect();

    let _ = app.emit(
        "midi://status",
        MidiStatus {
            connected: false,
            port: None,
            message: "Disconnected".into(),
        },
    );

    Ok(())
}

#[tauri::command]
pub fn save_controller_profile(
    app: AppHandle,
    profile: ControllerProfile,
) -> Result<String, String> {
    let file_name = profile_file_name(&profile.id).map_err(|error| error.to_string())?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("controller-profiles");
    let path = directory.join(file_name);
    save_profile(&path, &profile).map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn ensure_audio_engine(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AudioDeviceInfo, String> {
    with_audio_engine(&app, &state, |audio| Ok(audio.device_info()))
}

#[tauri::command]
pub fn trigger_virtual_pad(
    app: AppHandle,
    state: State<'_, AppState>,
    pad: u8,
    velocity: u8,
) -> Result<AudioDeviceInfo, String> {
    if pad >= 16 {
        return Err(format!("invalid virtual pad index: {pad}"));
    }
    if velocity == 0 {
        return Err("velocity must be between 1 and 127".into());
    }

    let device = with_audio_engine(&app, &state, |audio| {
        audio
            .send(AudioCommand::Trigger { pad, velocity })
            .map_err(|error| error.to_string())?;
        Ok(audio.device_info())
    })?;

    let synthetic_event = MidiEvent {
        timestamp_micros: state.started.elapsed().as_micros() as u64,
        channel: 10,
        kind: MidiEventKind::NoteOn,
        data1: 36 + pad,
        value: MidiValue::SevenBit(velocity),
    };
    let _ = app.emit("midi://event", synthetic_event);

    if let Some(pattern) = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .record_hit(pad, velocity)?
    {
        clear_ai_preview(&state)?;
        let _ = app.emit("sequencer://pattern", pattern);
    }

    Ok(device)
}

#[tauri::command]
pub fn set_master_gain(state: State<'_, AppState>, gain: f32) -> Result<(), String> {
    let gain = gain.clamp(0.0, 1.5);
    *state
        .master_gain
        .lock()
        .map_err(|_| "master gain lock poisoned".to_string())? = gain;
    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        audio
            .send(AudioCommand::SetMasterGain(gain))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_kit_state(state: State<'_, AppState>) -> Result<Vec<KitPadSnapshot>, String> {
    state
        .kit
        .lock()
        .map(|kit| kit.clone())
        .map_err(|_| "kit state lock poisoned".to_string())
}

#[tauri::command]
pub fn list_sample_library(app: AppHandle) -> Result<Vec<SampleLibraryItem>, String> {
    let directory = sample_library_dir(&app)?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let mut items = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if !path.is_file()
            || path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_none_or(|extension| !extension.eq_ignore_ascii_case("wav"))
        {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        let file = entry.file_name().to_string_lossy().into_owned();
        let display_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Sample")
            .to_owned();
        items.push(SampleLibraryItem {
            file,
            display_name,
            bytes: metadata.len(),
        });
    }
    items.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    Ok(items)
}

#[tauri::command]
pub fn pick_and_import_pad_sample(
    app: AppHandle,
    state: State<'_, AppState>,
    pad: usize,
) -> Result<Option<ImportedPadSample>, String> {
    validate_pad_index(pad)?;
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("WAV audio", &["wav"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let source = file_path
        .into_path()
        .map_err(|error| format!("selected file is not a local path: {error}"))?;
    validate_wav_file(&source)?;

    let metadata = fs::metadata(&source).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_IMPORTED_SAMPLE_BYTES {
        return Err(format!(
            "sample is too large: {} MB (limit {} MB)",
            metadata.len() / (1024 * 1024),
            MAX_IMPORTED_SAMPLE_BYTES / (1024 * 1024)
        ));
    }
    let sample = load_wav_mono(&source).map_err(|error| error.to_string())?;
    let display_name = sample.name.clone();
    let directory = sample_library_dir(&app)?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .map(sanitize_file_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "sample".to_string());
    let file = format!("{}-{stem}.wav", unique_timestamp());
    let destination = directory.join(&file);
    fs::copy(&source, &destination).map_err(|error| error.to_string())?;
    Ok(Some(assign_loaded_sample(
        &state,
        pad,
        file,
        display_name,
        sample,
    )?))
}

#[tauri::command]
pub fn assign_library_sample(
    app: AppHandle,
    state: State<'_, AppState>,
    pad: usize,
    file: String,
) -> Result<ImportedPadSample, String> {
    validate_pad_index(pad)?;
    let path = resolve_library_file(&app, &file)?;
    validate_wav_file(&path)?;
    let sample = load_wav_mono(&path).map_err(|error| error.to_string())?;
    let display_name = library_display_name(&file);
    assign_loaded_sample(&state, pad, file, display_name, sample)
}

#[tauri::command]
pub fn set_pad_mixer(
    state: State<'_, AppState>,
    pad: usize,
    gain: f32,
    pan: f32,
) -> Result<KitPadSnapshot, String> {
    validate_pad_index(pad)?;
    let updated = {
        let mut kit = state
            .kit
            .lock()
            .map_err(|_| "kit state lock poisoned".to_string())?;
        let slot = kit
            .get_mut(pad)
            .ok_or_else(|| "kit pad is unavailable".to_string())?;
        slot.gain = gain.clamp(0.0, 2.0);
        slot.pan = pan.clamp(-1.0, 1.0);
        slot.clone()
    };
    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        audio
            .send(AudioCommand::SetPadGain {
                pad: pad as u8,
                gain: updated.gain,
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadPan {
                pad: pad as u8,
                pan: updated.pan,
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(updated)
}

#[tauri::command]
pub fn set_pad_playback(
    state: State<'_, AppState>,
    pad: usize,
    pitch_semitones: f32,
    reverse: bool,
    sample_start: f32,
    sample_end: f32,
    choke_group: Option<u8>,
) -> Result<KitPadSnapshot, String> {
    validate_pad_index(pad)?;
    if choke_group.is_some_and(|group| group == 0) {
        return Err("choke group must be between 1 and 127".to_string());
    }
    let start = sample_start.clamp(0.0, 0.999);
    let end = sample_end.clamp(start + 0.001, 1.0);
    let updated = {
        let mut kit = state
            .kit
            .lock()
            .map_err(|_| "kit state lock poisoned".to_string())?;
        let slot = kit
            .get_mut(pad)
            .ok_or_else(|| "kit pad is unavailable".to_string())?;
        slot.pitch_semitones = pitch_semitones.clamp(-24.0, 24.0);
        slot.reverse = reverse;
        slot.sample_start = start;
        slot.sample_end = end;
        slot.choke_group = choke_group;
        slot.clone()
    };
    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        audio
            .send(AudioCommand::SetPadPlayback {
                pad: pad as u8,
                params: playback_params(&updated),
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(updated)
}

#[tauri::command]
pub fn set_pad_velocity_layers(
    app: AppHandle,
    state: State<'_, AppState>,
    pad: usize,
    layers: Vec<VelocityLayerSnapshot>,
) -> Result<KitPadSnapshot, String> {
    validate_pad_index(pad)?;
    validate_velocity_layer_snapshots(&layers)?;
    let audio_layers = load_velocity_layers(&app, &layers)?;

    let updated = {
        let mut kit = state
            .kit
            .lock()
            .map_err(|_| "kit state lock poisoned".to_string())?;
        let slot = kit
            .get_mut(pad)
            .ok_or_else(|| "kit pad is unavailable".to_string())?;
        slot.velocity_layers = layers;
        slot.clone()
    };

    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        audio
            .send(AudioCommand::SetPadVelocityLayers {
                pad: pad as u8,
                layers: audio_layers,
            })
            .map_err(|error| error.to_string())?;
    }

    Ok(updated)
}

#[tauri::command]
pub fn reset_pad_to_demo(state: State<'_, AppState>, pad: usize) -> Result<KitPadSnapshot, String> {
    validate_pad_index(pad)?;
    let default = default_kit()
        .get(pad)
        .cloned()
        .ok_or_else(|| "default kit pad is unavailable".to_string())?;
    {
        let mut kit = state
            .kit
            .lock()
            .map_err(|_| "kit state lock poisoned".to_string())?;
        kit[pad] = default.clone();
    }
    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        let demo = demo_kit(audio.device_info().sample_rate);
        audio
            .send(AudioCommand::ReplacePad {
                pad: pad as u8,
                sample: demo[pad].clone(),
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadGain {
                pad: pad as u8,
                gain: default.gain,
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadPan {
                pad: pad as u8,
                pan: default.pan,
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadPlayback {
                pad: pad as u8,
                params: playback_params(&default),
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadVelocityLayers {
                pad: pad as u8,
                layers: Arc::from([]),
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(default)
}

#[tauri::command]
pub fn get_pattern(state: State<'_, AppState>) -> Result<Pattern, String> {
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .pattern()
}

#[tauri::command]
pub fn get_pattern_bank(state: State<'_, AppState>) -> Result<PatternBank, String> {
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .bank()
}

#[tauri::command]
pub fn get_pattern_bank_status(state: State<'_, AppState>) -> Result<PatternBankStatus, String> {
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .bank_status()
}

#[tauri::command]
pub fn launch_pattern_slot(
    state: State<'_, AppState>,
    slot: usize,
    quantization: LaunchQuantization,
) -> Result<PatternLaunchResult, String> {
    let (pattern, status) = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .queue_pattern(slot, quantization)?;
    clear_ai_preview(&state)?;
    Ok(PatternLaunchResult { pattern, status })
}

#[tauri::command]
pub fn copy_active_pattern_to_slot(
    state: State<'_, AppState>,
    slot: usize,
) -> Result<PatternBankStatus, String> {
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .copy_active_pattern_to(slot)
}

#[tauri::command]
pub fn list_projects(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ProjectSummary>, String> {
    with_project_store(&app, &state, |store| store.list())
}

#[tauri::command]
pub fn save_project(
    app: AppHandle,
    state: State<'_, AppState>,
    input: SaveProjectInput,
) -> Result<ProjectSummary, String> {
    let mut bank = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .bank()?;
    bank.queued = None;
    let snapshot = ProjectSnapshot {
        schema_version: PROJECT_SCHEMA_VERSION,
        name: input.name,
        pattern_bank: bank,
        launch_quantization: input.launch_quantization,
        master_gain: input.master_gain.clamp(0.0, 1.5),
        groove_params: input.groove_params,
        scenes: input.scenes,
        kit: state
            .kit
            .lock()
            .map_err(|_| "kit state lock poisoned".to_string())?
            .clone(),
    };
    with_project_store(&app, &state, |store| store.save(input.id, &snapshot))
}

#[tauri::command]
pub fn load_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<LoadedProject, String> {
    let mut stored = with_project_store(&app, &state, |store| store.load(id))?;
    stored.snapshot.pattern_bank.queued = None;
    *state
        .kit
        .lock()
        .map_err(|_| "kit state lock poisoned".to_string())? = stored.snapshot.kit.clone();
    *state
        .master_gain
        .lock()
        .map_err(|_| "master gain lock poisoned".to_string())? =
        stored.snapshot.master_gain.clamp(0.0, 1.5);
    let mut warnings = validate_kit_library_files(&app, &state)?;
    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        audio
            .send(AudioCommand::StopAll)
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetMasterGain(stored.snapshot.master_gain))
            .map_err(|error| error.to_string())?;
        warnings.extend(apply_kit_to_audio(&app, &state, audio));
    }

    let (pattern, bank_status) = {
        let mut sequencer = state
            .sequencer
            .lock()
            .map_err(|_| "sequencer state lock poisoned".to_string())?;
        sequencer.stop();
        let pattern = sequencer.replace_bank(stored.snapshot.pattern_bank.clone())?;
        let status = sequencer.bank_status()?;
        (pattern, status)
    };
    clear_ai_preview(&state)?;

    Ok(LoadedProject {
        stored,
        pattern,
        bank_status,
        warnings,
    })
}

#[tauri::command]
pub fn delete_project(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    with_project_store(&app, &state, |store| store.delete(id))
}

#[tauri::command]
pub fn update_pattern_step(
    state: State<'_, AppState>,
    track: usize,
    step: usize,
    active: bool,
    velocity: u8,
) -> Result<Pattern, String> {
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .set_step(track, step, active, velocity)?;
    clear_ai_preview(&state)?;
    Ok(pattern)
}

#[tauri::command]
pub fn set_pattern_length(
    state: State<'_, AppState>,
    total_steps: usize,
) -> Result<Pattern, String> {
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .set_step_count(total_steps)?;
    clear_ai_preview(&state)?;
    Ok(pattern)
}

#[tauri::command]
pub fn set_sequencer_bpm(state: State<'_, AppState>, bpm: f64) -> Result<Pattern, String> {
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .set_bpm(bpm)?;
    clear_ai_preview(&state)?;
    Ok(pattern)
}

#[tauri::command]
pub fn set_sequencer_swing(state: State<'_, AppState>, swing: f32) -> Result<Pattern, String> {
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .set_swing(swing)?;
    clear_ai_preview(&state)?;
    Ok(pattern)
}

#[tauri::command]
pub fn clear_pattern(state: State<'_, AppState>) -> Result<Pattern, String> {
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .clear()?;
    clear_ai_preview(&state)?;
    Ok(pattern)
}

#[tauri::command]
pub fn load_demo_pattern(state: State<'_, AppState>) -> Result<Pattern, String> {
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .load_demo()?;
    clear_ai_preview(&state)?;
    Ok(pattern)
}

#[tauri::command]
pub fn start_sequencer(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AudioDeviceInfo, String> {
    let audio = audio_handle(&app, &state)?;
    let device = audio.device_info();
    let event_app = app.clone();
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .start(audio, move |tick| {
            let _ = event_app.emit("sequencer://step", tick.position);
            let _ = event_app.emit("sequencer://bank", tick.bank_status);
            if let Some(pattern) = tick.activated_pattern {
                let _ = event_app.emit("sequencer://pattern", pattern);
            }
        })?;
    Ok(device)
}

#[tauri::command]
pub fn stop_sequencer(state: State<'_, AppState>) -> Result<(), String> {
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .stop();
    Ok(())
}

#[tauri::command]
pub fn set_sequencer_recording(
    state: State<'_, AppState>,
    recording: bool,
) -> Result<bool, String> {
    state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .set_recording(recording)
}

#[tauri::command]
pub fn generate_groove_candidate(
    state: State<'_, AppState>,
    operation: GrooveOperation,
    params: GrooveParams,
) -> Result<Pattern, String> {
    let source = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .pattern()?;
    let candidate = generate_candidates(&source, operation, &params, 1)
        .into_iter()
        .next()
        .ok_or_else(|| "groove engine produced no candidate".to_string())?;
    *state
        .ai_preview
        .lock()
        .map_err(|_| "AI preview lock poisoned".to_string())? = Some(candidate.pattern.clone());
    *state
        .ai_feedback_context
        .lock()
        .map_err(|_| "AI feedback context lock poisoned".to_string())? = Some(AiPreviewFeedback {
        operation: operation_name(operation).to_string(),
        provider: "local".to_string(),
        selected_id: candidate.id,
        local_score: candidate.metrics.local_score,
        jev_confidence: None,
    });
    Ok(candidate.pattern)
}

#[tauri::command]
pub fn get_jev_status() -> JevStatus {
    JevClient::status_from_env()
}

#[tauri::command]
pub async fn generate_hybrid_groove_candidate(
    state: State<'_, AppState>,
    operation: GrooveOperation,
    params: GrooveParams,
) -> Result<HybridGroovePreview, String> {
    let source = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .pattern()?;

    let candidates = generate_candidates(&source, operation, &params, HYBRID_CANDIDATE_COUNT);
    let local = best_local_candidate(&candidates)
        .cloned()
        .ok_or_else(|| "groove engine produced no candidates".to_string())?;

    let (selected, provider, jev, fallback_reason) = match JevConfig::from_env() {
        None => (
            local.clone(),
            "local".to_string(),
            None,
            Some("TYPESAFE_API_KEY is not configured".to_string()),
        ),
        Some(config) => {
            let jev_result = match JevClient::new(config) {
                Ok(client) => {
                    client
                        .rerank(&source, &candidates, operation, &params)
                        .await
                }
                Err(error) => Err(error),
            };
            match jev_result {
                Ok(decision)
                    if decision.confidence >= JEV_MIN_CONFIDENCE
                        && decision.safe_to_preview >= JEV_MIN_SAFE_TO_PREVIEW =>
                {
                    let chosen = candidates
                        .iter()
                        .find(|candidate| candidate.id == decision.selected_id)
                        .cloned()
                        .unwrap_or_else(|| local.clone());
                    (
                        chosen,
                        "jev".to_string(),
                        Some(summarize_jev(&decision)),
                        None,
                    )
                }
                Ok(decision) => (
                    local.clone(),
                    "localFallback".to_string(),
                    Some(summarize_jev(&decision)),
                    Some(format!(
                        "Jev confidence gate not met (confidence {:.2}, safe-to-preview {:.2})",
                        decision.confidence, decision.safe_to_preview
                    )),
                ),
                Err(error) => (
                    local.clone(),
                    "localFallback".to_string(),
                    None,
                    Some(format!("Jev unavailable: {error}")),
                ),
            }
        }
    };

    *state
        .ai_preview
        .lock()
        .map_err(|_| "AI preview lock poisoned".to_string())? = Some(selected.pattern.clone());
    *state
        .ai_feedback_context
        .lock()
        .map_err(|_| "AI feedback context lock poisoned".to_string())? = Some(AiPreviewFeedback {
        operation: operation_name(operation).to_string(),
        provider: provider.clone(),
        selected_id: selected.id.clone(),
        local_score: selected.metrics.local_score,
        jev_confidence: jev.as_ref().map(|decision| decision.confidence),
    });

    Ok(HybridGroovePreview {
        pattern: selected.pattern,
        provider,
        selected_id: selected.id,
        candidate_count: candidates.len(),
        local_score: selected.metrics.local_score,
        jev,
        fallback_reason,
    })
}

#[tauri::command]
pub async fn generate_directed_groove_candidate(
    app: AppHandle,
    state: State<'_, AppState>,
    params: GrooveParams,
    project_id: Option<i64>,
) -> Result<DirectedGroovePreview, String> {
    let source = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .pattern()?;

    let memory = with_project_store(&app, &state, |store| {
        store.ai_preference_summary(project_id)
    })
    .unwrap_or_default();
    let memory_json = serde_json::to_value(&memory).unwrap_or_else(|_| serde_json::json!({}));

    let (client, client_setup_reason) = match JevConfig::from_env() {
        None => (None, Some("TYPESAFE_API_KEY is not configured".to_string())),
        Some(config) => match JevClient::new(config) {
            Ok(client) => (Some(client), None),
            Err(error) => (None, Some(format!("Jev client unavailable: {error}"))),
        },
    };

    let local_plan = local_director_plan(&source, &params);
    let mut planning_fallback_reason = client_setup_reason;
    let (plan, plan_provider) = if let Some(client) = client.as_ref() {
        match client.direct(&source, &params, memory_json).await {
            Ok(decision) if decision.confidence >= JEV_DIRECTOR_MIN_CONFIDENCE => {
                planning_fallback_reason = None;
                (decision, "jev".to_string())
            }
            Ok(decision) => {
                planning_fallback_reason = Some(format!(
                    "Jev Director confidence gate not met ({:.2})",
                    decision.confidence
                ));
                (local_plan, "localFallback".to_string())
            }
            Err(error) => {
                planning_fallback_reason = Some(format!("Jev Director unavailable: {error}"));
                (local_plan, "localFallback".to_string())
            }
        }
    } else {
        (local_plan, "local".to_string())
    };

    let plan_summary = DirectorPlanSummary {
        operation: plan.operation,
        direction: plan.direction,
        intensity: plan.intensity,
        provider: plan_provider,
        confidence: (plan.model != "local").then_some(plan.confidence),
        worth_changing: (plan.model != "local").then_some(plan.worth_changing),
        model: (plan.model != "local").then_some(plan.model.clone()),
        memory_events: memory.total,
        memory_accepted: memory.accepted,
        fallback_reason: planning_fallback_reason,
    };

    if plan.model != "local" && plan.worth_changing < JEV_DIRECTOR_MIN_WORTH_CHANGING {
        clear_ai_preview(&state)?;
        return Ok(DirectedGroovePreview {
            preview: None,
            plan: plan_summary,
        });
    }

    let (operation, directed_params) = apply_director_plan(&params, &plan);
    let candidates =
        generate_candidates(&source, operation, &directed_params, HYBRID_CANDIDATE_COUNT);
    let local = best_local_candidate(&candidates)
        .cloned()
        .ok_or_else(|| "groove engine produced no candidates".to_string())?;

    let (selected, provider, jev, fallback_reason) = if let Some(client) = client.as_ref() {
        match client
            .rerank(&source, &candidates, operation, &directed_params)
            .await
        {
            Ok(decision)
                if decision.confidence >= JEV_MIN_CONFIDENCE
                    && decision.safe_to_preview >= JEV_MIN_SAFE_TO_PREVIEW =>
            {
                let chosen = candidates
                    .iter()
                    .find(|candidate| candidate.id == decision.selected_id)
                    .cloned()
                    .unwrap_or_else(|| local.clone());
                (
                    chosen,
                    "jev".to_string(),
                    Some(summarize_jev(&decision)),
                    None,
                )
            }
            Ok(decision) => (
                local.clone(),
                "localFallback".to_string(),
                Some(summarize_jev(&decision)),
                Some(format!(
                    "Jev rerank confidence gate not met (confidence {:.2}, safe-to-preview {:.2})",
                    decision.confidence, decision.safe_to_preview
                )),
            ),
            Err(error) => (
                local.clone(),
                "localFallback".to_string(),
                None,
                Some(format!("Jev rerank unavailable: {error}")),
            ),
        }
    } else {
        (
            local.clone(),
            "local".to_string(),
            None,
            Some("Jev rerank is not configured".to_string()),
        )
    };

    *state
        .ai_preview
        .lock()
        .map_err(|_| "AI preview lock poisoned".to_string())? = Some(selected.pattern.clone());
    *state
        .ai_feedback_context
        .lock()
        .map_err(|_| "AI feedback context lock poisoned".to_string())? = Some(AiPreviewFeedback {
        operation: operation_name(operation).to_string(),
        provider: format!("director:{provider}"),
        selected_id: selected.id.clone(),
        local_score: selected.metrics.local_score,
        jev_confidence: jev.as_ref().map(|decision| decision.confidence),
    });

    Ok(DirectedGroovePreview {
        preview: Some(HybridGroovePreview {
            pattern: selected.pattern,
            provider,
            selected_id: selected.id,
            candidate_count: candidates.len(),
            local_score: selected.metrics.local_score,
            jev,
            fallback_reason,
        }),
        plan: plan_summary,
    })
}

#[tauri::command]
pub fn accept_groove_candidate(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: Option<i64>,
) -> Result<Pattern, String> {
    let candidate = state
        .ai_preview
        .lock()
        .map_err(|_| "AI preview lock poisoned".to_string())?
        .take()
        .ok_or_else(|| "no AI candidate is available".to_string())?;
    let context = take_ai_feedback_context(&state)?;
    let pattern = state
        .sequencer
        .lock()
        .map_err(|_| "sequencer state lock poisoned".to_string())?
        .replace_pattern(candidate)?;
    record_ai_feedback_best_effort(&app, &state, project_id, context, true);
    Ok(pattern)
}

#[tauri::command]
pub fn discard_groove_candidate(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: Option<i64>,
) -> Result<(), String> {
    *state
        .ai_preview
        .lock()
        .map_err(|_| "AI preview lock poisoned".to_string())? = None;
    let context = take_ai_feedback_context(&state)?;
    record_ai_feedback_best_effort(&app, &state, project_id, context, false);
    Ok(())
}

fn with_audio_engine<T>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    action: impl FnOnce(&AudioEngineHandle) -> Result<T, String>,
) -> Result<T, String> {
    let audio = audio_handle(app, state)?;
    action(&audio)
}

fn audio_handle(app: &AppHandle, state: &State<'_, AppState>) -> Result<AudioEngineHandle, String> {
    let mut guard = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?;

    if guard.is_none() {
        let audio = AudioEngineHandle::start_default().map_err(|error| error.to_string())?;
        let master_gain = *state
            .master_gain
            .lock()
            .map_err(|_| "master gain lock poisoned".to_string())?;
        audio
            .send(AudioCommand::SetMasterGain(master_gain))
            .map_err(|error| error.to_string())?;
        let _ = apply_kit_to_audio(app, state, &audio);
        *guard = Some(audio);
    }

    let audio = guard
        .as_ref()
        .ok_or_else(|| "audio engine unavailable".to_string())?;
    Ok(audio.clone())
}

fn assign_loaded_sample(
    state: &State<'_, AppState>,
    pad: usize,
    file: String,
    display_name: String,
    sample: PadSample,
) -> Result<ImportedPadSample, String> {
    validate_pad_index(pad)?;
    let sample_rate = sample.sample_rate;
    let frame_count = sample.samples.len();
    let updated = {
        let mut kit = state
            .kit
            .lock()
            .map_err(|_| "kit state lock poisoned".to_string())?;
        let slot = kit
            .get_mut(pad)
            .ok_or_else(|| "kit pad is unavailable".to_string())?;
        slot.display_name = display_name;
        slot.sample_file = Some(file);
        slot.clone()
    };

    if let Some(audio) = state
        .audio
        .lock()
        .map_err(|_| "audio state lock poisoned".to_string())?
        .as_ref()
    {
        audio
            .send(AudioCommand::ReplacePad {
                pad: pad as u8,
                sample,
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadGain {
                pad: pad as u8,
                gain: updated.gain,
            })
            .map_err(|error| error.to_string())?;
        audio
            .send(AudioCommand::SetPadPan {
                pad: pad as u8,
                pan: updated.pan,
            })
            .map_err(|error| error.to_string())?;
    }

    Ok(ImportedPadSample {
        pad,
        state: updated,
        sample_rate,
        frame_count,
    })
}

fn apply_kit_to_audio(
    app: &AppHandle,
    state: &State<'_, AppState>,
    audio: &AudioEngineHandle,
) -> Vec<String> {
    let kit = match state.kit.lock() {
        Ok(kit) => kit.clone(),
        Err(_) => return vec!["kit state lock poisoned".to_string()],
    };
    let demos = demo_kit(audio.device_info().sample_rate);
    let mut warnings = Vec::new();

    for (pad, slot) in kit.iter().enumerate() {
        let sample = match slot.sample_file.as_deref() {
            Some(file) => match resolve_library_file(app, file)
                .and_then(|path| load_wav_mono(&path).map_err(|error| error.to_string()))
            {
                Ok(sample) => sample,
                Err(error) => {
                    warnings.push(format!(
                        "Pad {} sample '{}' unavailable: {}. Demo sound restored.",
                        pad + 1,
                        slot.display_name,
                        error
                    ));
                    demos[pad].clone()
                }
            },
            None => demos[pad].clone(),
        };

        for command in [
            AudioCommand::ReplacePad {
                pad: pad as u8,
                sample,
            },
            AudioCommand::SetPadGain {
                pad: pad as u8,
                gain: slot.gain,
            },
            AudioCommand::SetPadPan {
                pad: pad as u8,
                pan: slot.pan,
            },
            AudioCommand::SetPadPlayback {
                pad: pad as u8,
                params: playback_params(slot),
            },
        ] {
            if let Err(error) = audio.send(command) {
                warnings.push(format!("Pad {} audio update failed: {}", pad + 1, error));
                break;
            }
        }

        let (layers, layer_warnings) = load_velocity_layers_resilient(app, &slot.velocity_layers);
        warnings.extend(
            layer_warnings
                .into_iter()
                .map(|warning| format!("Pad {}: {}", pad + 1, warning)),
        );
        let layer_command = AudioCommand::SetPadVelocityLayers {
            pad: pad as u8,
            layers,
        };
        if let Err(error) = audio.send(layer_command) {
            warnings.push(format!(
                "Pad {} velocity-layer audio update failed: {}",
                pad + 1,
                error
            ));
        }
    }

    warnings
}

fn playback_params(slot: &KitPadSnapshot) -> PadPlaybackParams {
    PadPlaybackParams {
        pitch_semitones: slot.pitch_semitones,
        reverse: slot.reverse,
        start: slot.sample_start,
        end: slot.sample_end,
        choke_group: slot.choke_group,
    }
}

fn validate_velocity_layer_snapshots(layers: &[VelocityLayerSnapshot]) -> Result<(), String> {
    if layers.len() > MAX_VELOCITY_LAYERS {
        return Err(format!(
            "a pad supports at most {MAX_VELOCITY_LAYERS} velocity layers"
        ));
    }
    for (index, layer) in layers.iter().enumerate() {
        if layer.min_velocity == 0
            || layer.min_velocity > 127
            || layer.max_velocity > 127
            || layer.max_velocity < layer.min_velocity
        {
            return Err(format!("velocity layer {} has an invalid range", index + 1));
        }
        if layer.sample_files.is_empty() {
            return Err(format!("velocity layer {} has no samples", index + 1));
        }
        if layer.sample_files.len() > MAX_ROUND_ROBIN_VARIANTS {
            return Err(format!(
                "velocity layer {} has more than {MAX_ROUND_ROBIN_VARIANTS} variants",
                index + 1
            ));
        }
        if layer.sample_files.iter().any(|file| file.trim().is_empty()) {
            return Err(format!(
                "velocity layer {} has an empty sample reference",
                index + 1
            ));
        }
        if layer
            .sample_files
            .iter()
            .enumerate()
            .any(|(file_index, file)| layer.sample_files[..file_index].contains(file))
        {
            return Err(format!(
                "velocity layer {} contains the same sample more than once",
                index + 1
            ));
        }
        for other in layers.iter().skip(index + 1) {
            let overlaps = layer.min_velocity <= other.max_velocity
                && other.min_velocity <= layer.max_velocity;
            if overlaps {
                return Err("velocity layer ranges cannot overlap".to_string());
            }
        }
    }
    Ok(())
}

fn load_velocity_layers(
    app: &AppHandle,
    layers: &[VelocityLayerSnapshot],
) -> Result<Arc<[VelocityLayer]>, String> {
    validate_velocity_layer_snapshots(layers)?;
    let mut loaded = Vec::with_capacity(layers.len());
    for layer in layers {
        let mut samples = Vec::with_capacity(layer.sample_files.len());
        for file in &layer.sample_files {
            let path = resolve_library_file(app, file)?;
            validate_wav_file(&path)?;
            samples.push(load_wav_mono(&path).map_err(|error| error.to_string())?);
        }
        loaded.push(VelocityLayer::new(
            layer.min_velocity,
            layer.max_velocity,
            samples,
            layer.round_robin,
        ));
    }
    Ok(loaded.into())
}

fn load_velocity_layers_resilient(
    app: &AppHandle,
    layers: &[VelocityLayerSnapshot],
) -> (Arc<[VelocityLayer]>, Vec<String>) {
    if let Err(error) = validate_velocity_layer_snapshots(layers) {
        return (Arc::from([]), vec![error]);
    }

    let mut loaded = Vec::with_capacity(layers.len());
    let mut warnings = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        let mut samples = Vec::with_capacity(layer.sample_files.len());
        let mut failed = None;
        for file in &layer.sample_files {
            let result = resolve_library_file(app, file).and_then(|path| {
                validate_wav_file(&path)?;
                load_wav_mono(&path).map_err(|error| error.to_string())
            });
            match result {
                Ok(sample) => samples.push(sample),
                Err(error) => {
                    failed = Some(format!(
                        "velocity layer {} disabled because '{}' could not be loaded: {}",
                        index + 1,
                        file,
                        error
                    ));
                    break;
                }
            }
        }
        if let Some(warning) = failed {
            warnings.push(warning);
            continue;
        }
        loaded.push(VelocityLayer::new(
            layer.min_velocity,
            layer.max_velocity,
            samples,
            layer.round_robin,
        ));
    }
    (loaded.into(), warnings)
}

fn validate_kit_library_files(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let kit = state
        .kit
        .lock()
        .map_err(|_| "kit state lock poisoned".to_string())?
        .clone();
    let mut warnings = Vec::new();
    for (pad, slot) in kit.iter().enumerate() {
        if let Some(file) = slot.sample_file.as_deref() {
            match resolve_library_file(app, file) {
                Ok(path) if path.is_file() => {}
                Ok(_) => warnings.push(format!(
                    "Pad {} sample '{}' is missing; demo sound will be used.",
                    pad + 1,
                    slot.display_name
                )),
                Err(error) => warnings.push(format!(
                    "Pad {} sample reference rejected: {}",
                    pad + 1,
                    error
                )),
            }
        }
        for (layer_index, layer) in slot.velocity_layers.iter().enumerate() {
            for file in &layer.sample_files {
                match resolve_library_file(app, file) {
                    Ok(path) if path.is_file() => {}
                    Ok(_) => warnings.push(format!(
                        "Pad {} velocity layer {} sample '{}' is missing.",
                        pad + 1,
                        layer_index + 1,
                        file
                    )),
                    Err(error) => warnings.push(format!(
                        "Pad {} velocity layer {} sample reference rejected: {}",
                        pad + 1,
                        layer_index + 1,
                        error
                    )),
                }
            }
        }
    }
    Ok(warnings)
}

fn sample_library_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("sample-library"))
}

fn resolve_library_file(app: &AppHandle, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    let is_plain_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == file)
        && path.components().count() == 1;
    if !is_plain_name || file.starts_with('.') {
        return Err("invalid sample-library file reference".to_string());
    }
    let extension_ok = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"));
    if !extension_ok {
        return Err("sample-library reference must be a WAV file".to_string());
    }
    Ok(sample_library_dir(app)?.join(file))
}

fn validate_wav_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("selected WAV does not exist".to_string());
    }
    let extension_ok = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"));
    if !extension_ok {
        return Err("only WAV files are supported in this milestone".to_string());
    }
    Ok(())
}

fn validate_pad_index(pad: usize) -> Result<(), String> {
    if pad >= 16 {
        Err(format!("invalid pad index: {pad}"))
    } else {
        Ok(())
    }
}

fn sanitize_file_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(80)
        .collect()
}

fn unique_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn library_display_name(file: &str) -> String {
    let stem = Path::new(file)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Sample");
    stem.split_once('-')
        .filter(|(prefix, _)| prefix.chars().all(|character| character.is_ascii_digit()))
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| stem.to_string())
}

fn clear_ai_preview(state: &State<'_, AppState>) -> Result<(), String> {
    *state
        .ai_preview
        .lock()
        .map_err(|_| "AI preview lock poisoned".to_string())? = None;
    *state
        .ai_feedback_context
        .lock()
        .map_err(|_| "AI feedback context lock poisoned".to_string())? = None;
    Ok(())
}

fn take_ai_feedback_context(
    state: &State<'_, AppState>,
) -> Result<Option<AiPreviewFeedback>, String> {
    state
        .ai_feedback_context
        .lock()
        .map_err(|_| "AI feedback context lock poisoned".to_string())
        .map(|mut context| context.take())
}

fn record_ai_feedback_best_effort(
    app: &AppHandle,
    state: &State<'_, AppState>,
    project_id: Option<i64>,
    context: Option<AiPreviewFeedback>,
    accepted: bool,
) {
    let Some(context) = context else {
        return;
    };
    let _ = with_project_store(app, state, |store| {
        store.record_ai_feedback(&AiFeedbackInput {
            project_id,
            operation: context.operation,
            provider: context.provider,
            selected_id: context.selected_id,
            local_score: context.local_score,
            jev_confidence: context.jev_confidence,
            accepted,
        })
    });
}

fn operation_name(operation: GrooveOperation) -> &'static str {
    match operation {
        GrooveOperation::Complete => "complete",
        GrooveOperation::Variation => "variation",
        GrooveOperation::Fill => "fill",
        GrooveOperation::Humanize => "humanize",
        GrooveOperation::Simplify => "simplify",
        GrooveOperation::Complexify => "complexify",
    }
}

fn local_director_plan(source: &Pattern, params: &GrooveParams) -> JevDirectorDecision {
    let hits = active_hit_count(source);
    let steps = source.total_steps.max(1);
    let (operation, direction) = if hits == 0 {
        (GrooveOperation::Complete, PerformanceDirection::Hold)
    } else if hits < steps / 2 {
        (GrooveOperation::Complexify, PerformanceDirection::Build)
    } else if hits > steps.saturating_mul(2) {
        (GrooveOperation::Simplify, PerformanceDirection::Release)
    } else {
        (GrooveOperation::Variation, PerformanceDirection::Hold)
    };
    let intensity = if params.amount < 0.3 {
        DirectorIntensity::Subtle
    } else if params.amount > 0.7 {
        DirectorIntensity::Bold
    } else {
        DirectorIntensity::Balanced
    };

    JevDirectorDecision {
        operation,
        direction,
        intensity,
        confidence: 1.0,
        worth_changing: 1.0,
        model: "local".to_string(),
        input_tokens: 0,
    }
}

fn apply_director_plan(
    base: &GrooveParams,
    plan: &JevDirectorDecision,
) -> (GrooveOperation, GrooveParams) {
    let mut params = base.clone();
    let mut operation = plan.operation;

    match plan.intensity {
        DirectorIntensity::Subtle => {
            params.amount = params.amount.clamp(0.15, 0.35);
            params.complexity = params.complexity.min(0.55);
        }
        DirectorIntensity::Balanced => {}
        DirectorIntensity::Bold => {
            params.amount = params.amount.max(0.75);
            params.complexity = params.complexity.max(0.65);
        }
    }

    match plan.direction {
        PerformanceDirection::Hold => {}
        PerformanceDirection::Build => {
            params.density = (params.density + 0.15).min(1.0);
            params.complexity = (params.complexity + 0.1).min(1.0);
        }
        PerformanceDirection::Release => {
            params.density = (params.density - 0.15).max(0.0);
            params.amount = params.amount.min(0.65);
        }
        PerformanceDirection::Break => {
            operation = GrooveOperation::Fill;
            params.amount = params.amount.max(0.65);
        }
    }

    (operation, params)
}

fn with_project_store<T>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    action: impl FnOnce(&ProjectStore) -> Result<T, ProjectStoreError>,
) -> Result<T, String> {
    let mut projects = state
        .projects
        .lock()
        .map_err(|_| "project store lock poisoned".to_string())?;
    if projects.is_none() {
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        let path = directory.join("mpd24-ai.sqlite3");
        *projects = Some(ProjectStore::open(&path).map_err(|error| error.to_string())?);
    }
    let store = projects
        .as_ref()
        .ok_or_else(|| "project store unavailable".to_string())?;
    action(store).map_err(|error| error.to_string())
}

fn summarize_jev(decision: &JevRerankDecision) -> JevDecisionSummary {
    JevDecisionSummary {
        model: decision.model.clone(),
        confidence: decision.confidence,
        safe_to_preview: decision.safe_to_preview,
        input_tokens: decision.input_tokens,
    }
}

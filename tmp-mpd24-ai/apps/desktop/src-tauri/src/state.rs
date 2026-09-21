use std::{sync::Mutex, time::Instant};

use mpd24_ai_audio_engine::AudioEngineHandle;
use mpd24_ai_midi_engine::service::MidiService;
use mpd24_ai_project_store::{default_kit, KitPadSnapshot, ProjectStore};
use mpd24_ai_sequencer::Pattern;

use crate::sequencer_runtime::SequencerRuntime;

#[derive(Debug, Clone)]
pub struct AiPreviewFeedback {
    pub operation: String,
    pub provider: String,
    pub selected_id: String,
    pub local_score: f32,
    pub jev_confidence: Option<f32>,
}

pub struct AppState {
    pub midi: Mutex<MidiService>,
    pub audio: Mutex<Option<AudioEngineHandle>>,
    pub master_gain: Mutex<f32>,
    pub kit: Mutex<Vec<KitPadSnapshot>>,
    pub sequencer: Mutex<SequencerRuntime>,
    pub ai_preview: Mutex<Option<Pattern>>,
    pub ai_feedback_context: Mutex<Option<AiPreviewFeedback>>,
    pub projects: Mutex<Option<ProjectStore>>,
    pub started: Instant,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            midi: Mutex::new(MidiService::new()),
            audio: Mutex::new(None),
            master_gain: Mutex::new(0.86),
            kit: Mutex::new(default_kit()),
            sequencer: Mutex::new(SequencerRuntime::default()),
            ai_preview: Mutex::new(None),
            ai_feedback_context: Mutex::new(None),
            projects: Mutex::new(None),
            started: Instant::now(),
        }
    }
}

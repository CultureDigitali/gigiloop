import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AudioDeviceInfo,
  DirectedGroovePreview,
  MidiEvent,
  MidiPortInfo,
  MidiStatus,
  GrooveOperation,
  GrooveParams,
  HybridGroovePreview,
  ImportedPadSample,
  JevStatus,
  KitPadSnapshot,
  LaunchQuantization,
  PatternBank,
  PatternBankStatus,
  PatternLaunchResult,
  LoadedProject,
  ProjectSummary,
  SampleLibraryItem,
  SaveProjectInput,
  SequencerPattern,
  StepPosition,
  VelocityLayerSnapshot
} from "./types";

export type ControllerProfile = {
  id: string;
  displayName: string;
  nameMatches: string[];
  verification: "unverified" | "hardwareVerified";
  mappings: Array<{
    controlId: string;
    messageKind: string;
    channel: number;
    data1: number;
    label: string;
  }>;
};

export function listMidiInputs(): Promise<MidiPortInfo[]> {
  return invoke<MidiPortInfo[]>("list_midi_inputs");
}

export function connectMidiInput(portId: string): Promise<MidiPortInfo> {
  return invoke<MidiPortInfo>("connect_midi_input", { portId });
}

export function disconnectMidiInput(): Promise<void> {
  return invoke<void>("disconnect_midi_input");
}

export function onMidiEvent(handler: (event: MidiEvent) => void): Promise<UnlistenFn> {
  return listen<MidiEvent>("midi://event", ({ payload }) => handler(payload));
}

export function onMidiStatus(handler: (status: MidiStatus) => void): Promise<UnlistenFn> {
  return listen<MidiStatus>("midi://status", ({ payload }) => handler(payload));
}

export function saveControllerProfile(
  profile: ControllerProfile
): Promise<string> {
  return invoke<string>("save_controller_profile", { profile });
}

export function ensureAudioEngine(): Promise<AudioDeviceInfo> {
  return invoke<AudioDeviceInfo>("ensure_audio_engine");
}

export function triggerVirtualPad(pad: number, velocity: number): Promise<AudioDeviceInfo> {
  return invoke<AudioDeviceInfo>("trigger_virtual_pad", { pad, velocity });
}

export function setMasterGain(gain: number): Promise<void> {
  return invoke<void>("set_master_gain", { gain });
}

export function getKitState(): Promise<KitPadSnapshot[]> {
  return invoke<KitPadSnapshot[]>("get_kit_state");
}

export function listSampleLibrary(): Promise<SampleLibraryItem[]> {
  return invoke<SampleLibraryItem[]>("list_sample_library");
}

export function pickAndImportPadSample(
  pad: number
): Promise<ImportedPadSample | null> {
  return invoke<ImportedPadSample | null>("pick_and_import_pad_sample", { pad });
}

export function assignLibrarySample(
  pad: number,
  file: string
): Promise<ImportedPadSample> {
  return invoke<ImportedPadSample>("assign_library_sample", { pad, file });
}

export function setPadMixer(
  pad: number,
  gain: number,
  pan: number
): Promise<KitPadSnapshot> {
  return invoke<KitPadSnapshot>("set_pad_mixer", { pad, gain, pan });
}

export function setPadPlayback(
  pad: number,
  pitchSemitones: number,
  reverse: boolean,
  sampleStart: number,
  sampleEnd: number,
  chokeGroup: number | null
): Promise<KitPadSnapshot> {
  return invoke<KitPadSnapshot>("set_pad_playback", {
    pad,
    pitchSemitones,
    reverse,
    sampleStart,
    sampleEnd,
    chokeGroup
  });
}

export function setPadVelocityLayers(
  pad: number,
  layers: VelocityLayerSnapshot[]
): Promise<KitPadSnapshot> {
  return invoke<KitPadSnapshot>("set_pad_velocity_layers", { pad, layers });
}

export function resetPadToDemo(pad: number): Promise<KitPadSnapshot> {
  return invoke<KitPadSnapshot>("reset_pad_to_demo", { pad });
}

export function getPattern(): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("get_pattern");
}

export function getPatternBank(): Promise<PatternBank> {
  return invoke<PatternBank>("get_pattern_bank");
}

export function getPatternBankStatus(): Promise<PatternBankStatus> {
  return invoke<PatternBankStatus>("get_pattern_bank_status");
}

export function launchPatternSlot(
  slot: number,
  quantization: LaunchQuantization
): Promise<PatternLaunchResult> {
  return invoke<PatternLaunchResult>("launch_pattern_slot", {
    slot,
    quantization
  });
}

export function copyActivePatternToSlot(slot: number): Promise<PatternBankStatus> {
  return invoke<PatternBankStatus>("copy_active_pattern_to_slot", { slot });
}

export function listProjects(): Promise<ProjectSummary[]> {
  return invoke<ProjectSummary[]>("list_projects");
}

export function saveProject(input: SaveProjectInput): Promise<ProjectSummary> {
  return invoke<ProjectSummary>("save_project", { input });
}

export function loadProject(id: number): Promise<LoadedProject> {
  return invoke<LoadedProject>("load_project", { id });
}

export function deleteProject(id: number): Promise<void> {
  return invoke<void>("delete_project", { id });
}

export function updatePatternStep(
  track: number,
  step: number,
  active: boolean,
  velocity: number
): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("update_pattern_step", {
    track,
    step,
    active,
    velocity
  });
}

export function setPatternLength(totalSteps: number): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("set_pattern_length", { totalSteps });
}

export function setSequencerBpm(bpm: number): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("set_sequencer_bpm", { bpm });
}

export function setSequencerSwing(swing: number): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("set_sequencer_swing", { swing });
}

export function clearPattern(): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("clear_pattern");
}

export function loadDemoPattern(): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("load_demo_pattern");
}

export function startSequencer(): Promise<AudioDeviceInfo> {
  return invoke<AudioDeviceInfo>("start_sequencer");
}

export function stopSequencer(): Promise<void> {
  return invoke<void>("stop_sequencer");
}

export function setSequencerRecording(recording: boolean): Promise<boolean> {
  return invoke<boolean>("set_sequencer_recording", { recording });
}

export function onSequencerStep(
  handler: (position: StepPosition) => void
): Promise<UnlistenFn> {
  return listen<StepPosition>("sequencer://step", ({ payload }) => handler(payload));
}

export function onSequencerPattern(
  handler: (pattern: SequencerPattern) => void
): Promise<UnlistenFn> {
  return listen<SequencerPattern>("sequencer://pattern", ({ payload }) => handler(payload));
}

export function onPatternBankStatus(
  handler: (status: PatternBankStatus) => void
): Promise<UnlistenFn> {
  return listen<PatternBankStatus>("sequencer://bank", ({ payload }) => handler(payload));
}

export function generateGrooveCandidate(
  operation: GrooveOperation,
  params: GrooveParams
): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("generate_groove_candidate", {
    operation,
    params
  });
}

export function getJevStatus(): Promise<JevStatus> {
  return invoke<JevStatus>("get_jev_status");
}

export function generateHybridGrooveCandidate(
  operation: GrooveOperation,
  params: GrooveParams
): Promise<HybridGroovePreview> {
  return invoke<HybridGroovePreview>("generate_hybrid_groove_candidate", {
    operation,
    params
  });
}

export function generateDirectedGrooveCandidate(
  params: GrooveParams,
  projectId: number | null
): Promise<DirectedGroovePreview> {
  return invoke<DirectedGroovePreview>("generate_directed_groove_candidate", {
    params,
    projectId
  });
}

export function acceptGrooveCandidate(
  projectId: number | null
): Promise<SequencerPattern> {
  return invoke<SequencerPattern>("accept_groove_candidate", { projectId });
}

export function discardGrooveCandidate(projectId: number | null): Promise<void> {
  return invoke<void>("discard_groove_candidate", { projectId });
}

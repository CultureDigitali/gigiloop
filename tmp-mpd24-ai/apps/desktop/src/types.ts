export type MidiPortInfo = {
  id: string;
  index: number;
  name: string;
};

export type MidiEventKind =
  | "noteOn"
  | "noteOff"
  | "controlChange"
  | "polyAftertouch"
  | "channelAftertouch"
  | "machineControl";

export type MidiValue = {
  kind: "sevenBit";
  value: number;
};

export type MidiEvent = {
  timestampMicros: number;
  channel: number;
  kind: MidiEventKind;
  data1: number;
  value: MidiValue;
};

export type MidiStatus = {
  connected: boolean;
  port: MidiPortInfo | null;
  message: string;
};

export type AudioDeviceInfo = {
  name: string;
  sampleRate: number;
  channels: number;
};

export type SequencerStep = {
  active: boolean;
  velocity: number;
  probability: number;
  microOffsetMicros: number;
};

export type SequencerTrack = {
  pad: number;
  length: number;
  muted: boolean;
  steps: SequencerStep[];
};

export type SequencerPattern = {
  name: string;
  bpm: number;
  totalSteps: number;
  swing: number;
  tracks: SequencerTrack[];
};

export type StepPosition = {
  step: number;
  cycle: number;
  activeSlot: number;
  queuedSlot: number | null;
};

export type LaunchQuantization =
  | "immediate"
  | "nextBeat"
  | "nextBar"
  | "nextTwoBars";

export type QueuedPattern = {
  slot: number;
  quantization: LaunchQuantization;
};

export type PatternBankStatus = {
  activeSlot: number;
  queuedSlot: number | null;
  names: string[];
};

export type PatternBank = {
  slots: SequencerPattern[];
  activeSlot: number;
  queued: QueuedPattern | null;
};

export type PatternLaunchResult = {
  pattern: SequencerPattern;
  status: PatternBankStatus;
};

export type GrooveOperation =
  | "complete"
  | "variation"
  | "fill"
  | "humanize"
  | "simplify"
  | "complexify";

export type GrooveParams = {
  amount: number;
  density: number;
  complexity: number;
  humanize: number;
  seed: number;
  lockedTracks: number[];
};

export type JevStatus = {
  configured: boolean;
  model: string;
};

export type JevDecisionSummary = {
  model: string;
  confidence: number;
  safeToPreview: number;
  inputTokens: number;
};

export type HybridGroovePreview = {
  pattern: SequencerPattern;
  provider: "jev" | "local" | "localFallback";
  selectedId: string;
  candidateCount: number;
  localScore: number;
  jev: JevDecisionSummary | null;
  fallbackReason: string | null;
};

export type PerformanceDirection = "hold" | "build" | "release" | "break";
export type DirectorIntensity = "subtle" | "balanced" | "bold";

export type DirectorPlanSummary = {
  operation: GrooveOperation;
  direction: PerformanceDirection;
  intensity: DirectorIntensity;
  provider: string;
  confidence: number | null;
  worthChanging: number | null;
  model: string | null;
  memoryEvents: number;
  memoryAccepted: number;
  fallbackReason: string | null;
};

export type DirectedGroovePreview = {
  preview: HybridGroovePreview | null;
  plan: DirectorPlanSummary;
};

export type ProjectSummary = {
  id: number;
  name: string;
  createdAtMs: number;
  updatedAtMs: number;
};

export type ProjectSnapshot = {
  schemaVersion: number;
  name: string;
  patternBank: PatternBank;
  launchQuantization: LaunchQuantization;
  masterGain: number;
  grooveParams: GrooveParams;
  scenes: Array<PerformanceScene | null>;
  kit: KitPadSnapshot[];
};

export type StoredProject = {
  summary: ProjectSummary;
  snapshot: ProjectSnapshot;
};

export type LoadedProject = {
  stored: StoredProject;
  pattern: SequencerPattern;
  bankStatus: PatternBankStatus;
  warnings: string[];
};

export type SaveProjectInput = {
  id: number | null;
  name: string;
  launchQuantization: LaunchQuantization;
  masterGain: number;
  grooveParams: GrooveParams;
  scenes: Array<PerformanceScene | null>;
};

export type PerformanceScene = {
  name: string;
  patternSlot: number;
  masterGain: number;
  launchQuantization: LaunchQuantization;
  grooveParams: GrooveParams;
};

export type KitPadSnapshot = {
  displayName: string;
  sampleFile: string | null;
  gain: number;
  pan: number;
  pitchSemitones: number;
  reverse: boolean;
  sampleStart: number;
  sampleEnd: number;
  chokeGroup: number | null;
  velocityLayers: VelocityLayerSnapshot[];
};

export type VelocityLayerSnapshot = {
  minVelocity: number;
  maxVelocity: number;
  sampleFiles: string[];
  roundRobin: boolean;
};

export type ImportedPadSample = {
  pad: number;
  state: KitPadSnapshot;
  sampleRate: number;
  frameCount: number;
};

export type SampleLibraryItem = {
  file: string;
  displayName: string;
  bytes: number;
};

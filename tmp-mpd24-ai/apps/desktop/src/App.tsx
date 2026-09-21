import { useCallback, useEffect, useRef, useState } from "react";
import {
  acceptGrooveCandidate,
  assignLibrarySample,
  connectMidiInput,
  copyActivePatternToSlot,
  clearPattern,
  deleteProject,
  discardGrooveCandidate,
  disconnectMidiInput,
  ensureAudioEngine,
  getKitState,
  getPattern,
  getPatternBankStatus,
  generateHybridGrooveCandidate,
  generateDirectedGrooveCandidate,
  getJevStatus,
  loadDemoPattern,
  launchPatternSlot,
  listMidiInputs,
  listSampleLibrary,
  listProjects,
  loadProject,
  onMidiEvent,
  onMidiStatus,
  onPatternBankStatus,
  onSequencerPattern,
  onSequencerStep,
  pickAndImportPadSample,
  resetPadToDemo,
  saveControllerProfile,
  saveProject,
  setMasterGain,
  setPadMixer,
  setPadPlayback,
  setPadVelocityLayers,
  setPatternLength,
  setSequencerBpm,
  setSequencerRecording,
  setSequencerSwing,
  startSequencer,
  stopSequencer,
  triggerVirtualPad,
  updatePatternStep
} from "./api";
import { DevicePicker } from "./components/DevicePicker";
import { AIPanel } from "./components/AIPanel";
import { KitMixerPanel } from "./components/KitMixerPanel";
import { MidiLearnPanel } from "./components/MidiLearnPanel";
import { MidiEventTable } from "./components/MidiEventTable";
import { PatternBankPanel } from "./components/PatternBankPanel";
import { ProjectPanel } from "./components/ProjectPanel";
import { ScenePanel } from "./components/ScenePanel";
import { StepSequencer } from "./components/StepSequencer";
import { StatusStrip } from "./components/StatusStrip";
import { VelocityLayerPanel } from "./components/VelocityLayerPanel";
import { VirtualPadGrid } from "./components/VirtualPadGrid";
import { reconcileMidiSelection } from "./deviceSelection";
import { appendMidiEvent } from "./eventHistory";
import {
  learnMapping,
  type LearnedMapping,
  upsertLearnedMapping
} from "./midiLearn";
import type {
  AudioDeviceInfo,
  DirectedGroovePreview,
  GrooveOperation,
  GrooveParams,
  HybridGroovePreview,
  JevStatus,
  KitPadSnapshot,
  LaunchQuantization,
  MidiEvent,
  MidiPortInfo,
  MidiStatus,
  PatternBankStatus,
  PerformanceScene,
  ProjectSummary,
  SampleLibraryItem,
  SequencerPattern,
  VelocityLayerSnapshot
} from "./types";
import { padForKeyboardKey } from "./virtualPads";

export default function App() {
  const [ports, setPorts] = useState<MidiPortInfo[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [events, setEvents] = useState<MidiEvent[]>([]);
  const [busy, setBusy] = useState(false);
  const [audioBusy, setAudioBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [audioDevice, setAudioDevice] = useState<AudioDeviceInfo | null>(null);
  const [velocity, setVelocity] = useState(108);
  const [masterGain, setMasterGainState] = useState(0.86);
  const [learnTarget, setLearnTarget] = useState("");
  const [learnedMappings, setLearnedMappings] = useState<LearnedMapping[]>([]);
  const [savedProfilePath, setSavedProfilePath] = useState("");
  const [pattern, setPattern] = useState<SequencerPattern | null>(null);
  const [sequencerPlaying, setSequencerPlaying] = useState(false);
  const [sequencerRecording, setSequencerRecordingState] = useState(false);
  const [sequencerBusy, setSequencerBusy] = useState(false);
  const [playhead, setPlayhead] = useState<number | null>(null);
  const [patternBankStatus, setPatternBankStatus] = useState<PatternBankStatus>({
    activeSlot: 0,
    queuedSlot: null,
    names: [
      "Pattern A",
      "Pattern B",
      "Pattern C",
      "Pattern D",
      "Pattern E",
      "Pattern F",
      "Pattern G",
      "Pattern H"
    ]
  });
  const [launchQuantization, setLaunchQuantization] =
    useState<LaunchQuantization>("nextBar");
  const [projectName, setProjectName] = useState("Untitled Project");
  const [currentProjectId, setCurrentProjectId] = useState<number | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<number | null>(null);
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [projectBusy, setProjectBusy] = useState(false);
  const [scenes, setScenes] = useState<Array<PerformanceScene | null>>(
    Array.from({ length: 8 }, () => null)
  );
  const [kit, setKit] = useState<KitPadSnapshot[]>(
    Array.from({ length: 16 }, (_, index) => ({
      displayName: "Demo Pad " + (index + 1),
      sampleFile: null,
      gain: 1,
      pan: 0,
      pitchSemitones: 0,
      reverse: false,
      sampleStart: 0,
      sampleEnd: 1,
      chokeGroup: [2, 3, 12].includes(index) ? 1 : null,
      velocityLayers: []
    }))
  );
  const [sampleLibrary, setSampleLibrary] = useState<SampleLibraryItem[]>([]);
  const [sampleBusyPad, setSampleBusyPad] = useState<number | null>(null);
  const [velocityLayerResetRevision, setVelocityLayerResetRevision] = useState(0);
  const [aiCandidate, setAiCandidate] = useState<SequencerPattern | null>(null);
  const [aiDecision, setAiDecision] = useState<HybridGroovePreview | null>(null);
  const [directorResult, setDirectorResult] =
    useState<DirectedGroovePreview | null>(null);
  const [aiBusy, setAiBusy] = useState(false);
  const [jevStatus, setJevStatus] = useState<JevStatus>({
    configured: false,
    model: "jev-latest"
  });
  const [aiParams, setAiParams] = useState<GrooveParams>({
    amount: 0.35,
    density: 0.5,
    complexity: 0.5,
    humanize: 0.3,
    seed: 1,
    lockedTracks: []
  });
  const learnTargetRef = useRef("");
  const [status, setStatus] = useState<MidiStatus>({
    connected: false,
    port: null,
    message: "Ready"
  });

  const refresh = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      const discovered = await listMidiInputs();
      setPorts(discovered);
      setSelectedId(reconcileMidiSelection(selectedId, discovered));
      if (
        status.connected &&
        status.port &&
        !discovered.some((port) => port.id === status.port?.id)
      ) {
        await disconnectMidiInput();
        setStatus({
          connected: false,
          port: null,
          message: "Connected MIDI device is no longer available"
        });
      }
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }, [selectedId, status.connected, status.port]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let disposed = false;
    let offEvent: (() => void) | undefined;
    let offStatus: (() => void) | undefined;

    void onMidiEvent((event) => {
      setEvents((current) => appendMidiEvent(current, event));
      const learned = learnMapping(learnTargetRef.current, event);
      if (learned) {
        setLearnedMappings((current) => upsertLearnedMapping(current, learned));
        learnTargetRef.current = "";
        setLearnTarget("");
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        offEvent = unlisten;
      }
    });

    void onMidiStatus(setStatus).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        offStatus = unlisten;
      }
    });

    return () => {
      disposed = true;
      offEvent?.();
      offStatus?.();
    };
  }, []);

  useEffect(() => {
    void getPattern()
      .then(setPattern)
      .catch((reason) => setError(String(reason)));
    void getPatternBankStatus()
      .then(setPatternBankStatus)
      .catch((reason) => setError(String(reason)));
    void getJevStatus()
      .then(setJevStatus)
      .catch(() => undefined);
    void getKitState()
      .then(setKit)
      .catch((reason) => setError(String(reason)));
    void listSampleLibrary()
      .then(setSampleLibrary)
      .catch((reason) => setError(String(reason)));
    void listProjects()
      .then((savedProjects) => {
        setProjects(savedProjects);
        if (savedProjects.length > 0) {
          setSelectedProjectId(savedProjects[0].id);
        }
      })
      .catch((reason) => setError(String(reason)));

    let disposed = false;
    let offStep: (() => void) | undefined;
    let offPattern: (() => void) | undefined;
    let offBank: (() => void) | undefined;
    void onSequencerStep((position) => {
      setPlayhead(position.step);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        offStep = unlisten;
      }
    });
    void onSequencerPattern((updated) => {
      setPattern(updated);
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        offPattern = unlisten;
      }
    });
    void onPatternBankStatus(setPatternBankStatus).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        offBank = unlisten;
      }
    });

    return () => {
      disposed = true;
      offStep?.();
      offPattern?.();
      offBank?.();
    };
  }, []);

  async function ensureAudio() {
    if (audioDevice) {
      return audioDevice;
    }
    setAudioBusy(true);
    try {
      const device = await ensureAudioEngine();
      setAudioDevice(device);
      return device;
    } finally {
      setAudioBusy(false);
    }
  }

  async function triggerPad(pad: number) {
    setError("");
    try {
      const device = await triggerVirtualPad(pad, velocity);
      setAudioDevice(device);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changeMasterGain(value: number) {
    setMasterGainState(value);
    try {
      await ensureAudio();
      await setMasterGain(value);
    } catch (reason) {
      setError(String(reason));
    }
  }

  function armLearn(target: string) {
    learnTargetRef.current = target;
    setLearnTarget(target);
  }

  async function saveDevelopmentProfile() {
    setError("");
    try {
      const path = await saveControllerProfile({
        id: "development-controller",
        displayName: "MPD24-AI Development Controller",
        nameMatches: [],
        verification: "unverified",
        mappings: learnedMappings
      });
      setSavedProfilePath(path);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function togglePatternStep(track: number, step: number, active: boolean) {
    setError("");
    try {
      const updated = await updatePatternStep(track, step, active, velocity);
      setPattern(updated);
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changePatternLength(totalSteps: number) {
    try {
      setPattern(await setPatternLength(totalSteps));
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changeSequencerBpm(bpm: number) {
    try {
      setPattern(await setSequencerBpm(bpm));
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changeSequencerSwing(swing: number) {
    try {
      setPattern(await setSequencerSwing(swing));
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function startPattern() {
    setSequencerBusy(true);
    setError("");
    try {
      const device = await startSequencer();
      setAudioDevice(device);
      setSequencerPlaying(true);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSequencerBusy(false);
    }
  }

  async function stopPattern() {
    setSequencerBusy(true);
    try {
      await stopSequencer();
      setSequencerPlaying(false);
      setSequencerRecordingState(false);
      setPlayhead(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSequencerBusy(false);
    }
  }

  async function changeSequencerRecording(recording: boolean) {
    try {
      const enabled = await setSequencerRecording(recording);
      setSequencerRecordingState(enabled);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function clearCurrentPattern() {
    try {
      setPattern(await clearPattern());
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function loadDemo() {
    try {
      setPattern(await loadDemoPattern());
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function launchSlot(slot: number) {
    setError("");
    try {
      const result = await launchPatternSlot(slot, launchQuantization);
      setPatternBankStatus(result.status);
      setPattern(result.pattern);
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function copyPatternToSlot(slot: number) {
    setError("");
    try {
      setPatternBankStatus(await copyActivePatternToSlot(slot));
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function refreshProjects(preferredId?: number | null) {
    const savedProjects = await listProjects();
    setProjects(savedProjects);
    if (preferredId !== undefined) {
      setSelectedProjectId(preferredId);
    } else if (
      selectedProjectId !== null &&
      !savedProjects.some((project) => project.id === selectedProjectId)
    ) {
      setSelectedProjectId(savedProjects[0]?.id ?? null);
    }
  }

  async function saveCurrentProject(saveAs: boolean) {
    setProjectBusy(true);
    setError("");
    try {
      const summary = await saveProject({
        id: saveAs ? null : currentProjectId,
        name: projectName.trim(),
        launchQuantization,
        masterGain,
        grooveParams: aiParams,
        scenes
      });
      setCurrentProjectId(summary.id);
      setProjectName(summary.name);
      await refreshProjects(summary.id);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setProjectBusy(false);
    }
  }

  async function openSelectedProject() {
    if (selectedProjectId === null) {
      return;
    }
    if (blockRealtimeUnsafeKitEdit()) {
      return;
    }
    setProjectBusy(true);
    setError("");
    try {
      const loaded = await loadProject(selectedProjectId);
      setCurrentProjectId(loaded.stored.summary.id);
      setProjectName(loaded.stored.snapshot.name);
      setPattern(loaded.pattern);
      setPatternBankStatus(loaded.bankStatus);
      setLaunchQuantization(loaded.stored.snapshot.launchQuantization);
      setMasterGainState(loaded.stored.snapshot.masterGain);
      setAiParams(loaded.stored.snapshot.grooveParams);
      setScenes(loaded.stored.snapshot.scenes);
      setKit(loaded.stored.snapshot.kit);
      setVelocityLayerResetRevision((current) => current + 1);
      setNotice(loaded.warnings.join(" "));
      setSequencerPlaying(false);
      setSequencerRecordingState(false);
      setPlayhead(null);
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setProjectBusy(false);
    }
  }

  async function deleteSelectedProject() {
    if (selectedProjectId === null) {
      return;
    }
    setProjectBusy(true);
    setError("");
    const deletingId = selectedProjectId;
    try {
      await deleteProject(deletingId);
      if (currentProjectId === deletingId) {
        setCurrentProjectId(null);
      }
      await refreshProjects(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setProjectBusy(false);
    }
  }

  function captureScene(index: number, name: string) {
    const scene: PerformanceScene = {
      name,
      patternSlot: patternBankStatus.activeSlot,
      masterGain,
      launchQuantization,
      grooveParams: aiParams
    };
    setScenes((current) =>
      current.map((item, sceneIndex) => (sceneIndex === index ? scene : item))
    );
  }

  async function recallScene(index: number) {
    const scene = scenes[index];
    if (!scene) {
      return;
    }
    setSequencerBusy(true);
    setError("");
    try {
      const result = await launchPatternSlot(
        scene.patternSlot,
        scene.launchQuantization
      );
      await setMasterGain(scene.masterGain);
      setPatternBankStatus(result.status);
      setPattern(result.pattern);
      setLaunchQuantization(scene.launchQuantization);
      setMasterGainState(scene.masterGain);
      setAiParams(scene.grooveParams);
      setAiCandidate(null);
      setAiDecision(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSequencerBusy(false);
    }
  }

  function clearScene(index: number) {
    setScenes((current) =>
      current.map((item, sceneIndex) => (sceneIndex === index ? null : item))
    );
  }

  function updateKitPad(pad: number, updated: KitPadSnapshot) {
    setKit((current) =>
      current.map((slot, index) => (index === pad ? updated : slot))
    );
  }

  async function refreshSampleLibrary() {
    setSampleLibrary(await listSampleLibrary());
  }

  function blockRealtimeUnsafeKitEdit() {
    if (!sequencerPlaying) {
      return false;
    }
    setError("");
    setNotice(
      "Stop playback before importing, replacing, resetting samples, or applying velocity layers."
    );
    return true;
  }

  async function importPadSample(pad: number) {
    if (blockRealtimeUnsafeKitEdit()) {
      return;
    }
    setSampleBusyPad(pad);
    setError("");
    setNotice("");
    try {
      const imported = await pickAndImportPadSample(pad);
      if (imported) {
        updateKitPad(pad, imported.state);
        await refreshSampleLibrary();
        setNotice(
          imported.state.displayName +
            " imported on pad " +
            (pad + 1) +
            " · " +
            imported.sampleRate +
            " Hz"
        );
      }
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSampleBusyPad(null);
    }
  }

  async function assignPadSample(pad: number, file: string) {
    if (blockRealtimeUnsafeKitEdit()) {
      return;
    }
    setSampleBusyPad(pad);
    setError("");
    setNotice("");
    try {
      const assigned = await assignLibrarySample(pad, file);
      updateKitPad(pad, assigned.state);
      setNotice(
        assigned.state.displayName + " assigned to pad " + (pad + 1)
      );
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSampleBusyPad(null);
    }
  }

  async function changePadMixer(pad: number, gain: number, pan: number) {
    try {
      updateKitPad(pad, await setPadMixer(pad, gain, pan));
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changePadPlayback(
    pad: number,
    pitchSemitones: number,
    reverse: boolean,
    sampleStart: number,
    sampleEnd: number,
    chokeGroup: number | null
  ) {
    try {
      updateKitPad(
        pad,
        await setPadPlayback(
          pad,
          pitchSemitones,
          reverse,
          sampleStart,
          sampleEnd,
          chokeGroup
        )
      );
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changePadVelocityLayers(
    pad: number,
    layers: VelocityLayerSnapshot[]
  ) {
    if (blockRealtimeUnsafeKitEdit()) {
      return;
    }
    setSampleBusyPad(pad);
    setError("");
    setNotice("");
    try {
      const updated = await setPadVelocityLayers(pad, layers);
      updateKitPad(pad, updated);
      setNotice(
        layers.length === 0
          ? "Velocity layers cleared on pad " + (pad + 1)
          : layers.length +
              " velocity layer" +
              (layers.length === 1 ? "" : "s") +
              " applied to pad " +
              (pad + 1)
      );
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSampleBusyPad(null);
    }
  }

  async function resetPad(pad: number) {
    if (blockRealtimeUnsafeKitEdit()) {
      return;
    }
    setSampleBusyPad(pad);
    setError("");
    try {
      updateKitPad(pad, await resetPadToDemo(pad));
      setVelocityLayerResetRevision((current) => current + 1);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSampleBusyPad(null);
    }
  }

  async function generateAI(operation: GrooveOperation) {
    setAiBusy(true);
    setError("");
    const nextParams = { ...aiParams, seed: aiParams.seed + 1 };
    setAiParams(nextParams);
    try {
      const preview = await generateHybridGrooveCandidate(operation, nextParams);
      setAiCandidate(preview.pattern);
      setAiDecision(preview);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setAiBusy(false);
    }
  }

  async function generateDirector() {
    setAiBusy(true);
    setError("");
    const nextParams = { ...aiParams, seed: aiParams.seed + 1 };
    setAiParams(nextParams);
    try {
      const result = await generateDirectedGrooveCandidate(
        nextParams,
        currentProjectId
      );
      setDirectorResult(result);
      if (result.preview) {
        setAiCandidate(result.preview.pattern);
        setAiDecision(result.preview);
      } else {
        setAiCandidate(null);
        setAiDecision(null);
      }
    } catch (reason) {
      setError(String(reason));
    } finally {
      setAiBusy(false);
    }
  }

  function toggleAiLock(track: number) {
    setAiParams((current) => ({
      ...current,
      lockedTracks: current.lockedTracks.includes(track)
        ? current.lockedTracks.filter((item) => item !== track)
        : [...current.lockedTracks, track]
    }));
  }

  async function acceptAI() {
    setAiBusy(true);
    try {
      setPattern(await acceptGrooveCandidate(currentProjectId));
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setAiBusy(false);
    }
  }

  async function discardAI() {
    try {
      await discardGrooveCandidate(currentProjectId);
      setAiCandidate(null);
      setAiDecision(null);
      setDirectorResult(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.repeat) {
        return;
      }
      const target = event.target as HTMLElement | null;
      if (
        target?.tagName === "INPUT" ||
        target?.tagName === "SELECT" ||
        target?.tagName === "TEXTAREA"
      ) {
        return;
      }
      const pad = padForKeyboardKey(event.key);
      if (pad !== null) {
        event.preventDefault();
        void triggerPad(pad);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [velocity]);

  async function connect() {
    setBusy(true);
    setError("");
    try {
      const port = await connectMidiInput(selectedId);
      setStatus({ connected: true, port, message: "Connected" });
      setEvents([]);
    } catch (reason) {
      setError(String(reason));
      setStatus({ connected: false, port: null, message: "Connection failed" });
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    setBusy(true);
    setError("");
    try {
      await disconnectMidiInput();
      setStatus({ connected: false, port: null, message: "Disconnected" });
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="app-shell">
      <header>
        <div>
          <div className="eyebrow">MPD24-AI / AI PERFORMANCE LAB</div>
          <h1>AI Drum Machine</h1>
        </div>
        <StatusStrip status={status} />
      </header>

      <DevicePicker
        ports={ports}
        selectedId={selectedId}
        connected={status.connected}
        busy={busy}
        onSelect={setSelectedId}
        onRefresh={() => void refresh()}
        onConnect={() => void connect()}
        onDisconnect={() => void disconnect()}
      />

      <p className="hint">
        MPD24 mappings remain unverified until each physical control is captured from the real
        device.
      </p>

      {error && <div className="error-banner">{error}</div>}
      {notice && <div className="notice-banner">{notice}</div>}

      <ProjectPanel
        projectName={projectName}
        currentProjectId={currentProjectId}
        selectedProjectId={selectedProjectId}
        projects={projects}
        busy={projectBusy}
        loadLocked={sequencerPlaying}
        onNameChange={setProjectName}
        onSelectedProjectChange={setSelectedProjectId}
        onSave={() => void saveCurrentProject(false)}
        onSaveAs={() => void saveCurrentProject(true)}
        onLoad={() => void openSelectedProject()}
        onDelete={() => void deleteSelectedProject()}
      />

      <section className="audio-strip">
        <div>
          <span className="audio-label">AUDIO</span>
          <strong>{audioDevice ? audioDevice.name : "Not started"}</strong>
          {audioDevice && (
            <span>
              {audioDevice.sampleRate} Hz · {audioDevice.channels} ch
            </span>
          )}
        </div>
        <label>
          Master {Math.round(masterGain * 100)}%
          <input
            type="range"
            min="0"
            max="1.2"
            step="0.01"
            value={masterGain}
            onChange={(event) => void changeMasterGain(Number(event.target.value))}
          />
        </label>
      </section>

      <VirtualPadGrid
        velocity={velocity}
        padNames={kit.map((slot) => slot.displayName)}
        disabled={audioBusy}
        onVelocityChange={setVelocity}
        onTrigger={(pad) => void triggerPad(pad)}
      />

      <KitMixerPanel
        kit={kit}
        library={sampleLibrary}
        busyPad={sampleBusyPad}
        sampleEditingLocked={sequencerPlaying}
        onImport={(pad) => void importPadSample(pad)}
        onAssign={(pad, file) => void assignPadSample(pad, file)}
        onReset={(pad) => void resetPad(pad)}
        onMixerChange={(pad, gain, pan) =>
          void changePadMixer(pad, gain, pan)
        }
        onPlaybackChange={(
          pad,
          pitchSemitones,
          reverse,
          sampleStart,
          sampleEnd,
          chokeGroup
        ) =>
          void changePadPlayback(
            pad,
            pitchSemitones,
            reverse,
            sampleStart,
            sampleEnd,
            chokeGroup
          )
        }
      />

      <VelocityLayerPanel
        kit={kit}
        library={sampleLibrary}
        busyPad={sampleBusyPad}
        resetRevision={velocityLayerResetRevision}
        auditionVelocity={velocity}
        sampleEditingLocked={sequencerPlaying}
        onApply={(pad, layers) => void changePadVelocityLayers(pad, layers)}
      />

      <PatternBankPanel
        status={patternBankStatus}
        quantization={launchQuantization}
        busy={sequencerBusy}
        onQuantizationChange={setLaunchQuantization}
        onLaunch={(slot) => void launchSlot(slot)}
        onCopy={(slot) => void copyPatternToSlot(slot)}
      />

      <ScenePanel
        scenes={scenes}
        busy={sequencerBusy}
        onCapture={captureScene}
        onRecall={(index) => void recallScene(index)}
        onClear={clearScene}
      />

      {pattern && (
        <StepSequencer
          pattern={pattern}
          previewPattern={aiCandidate}
          playhead={playhead}
          playing={sequencerPlaying}
          recording={sequencerRecording}
          busy={sequencerBusy}
          defaultVelocity={velocity}
          onToggleStep={(track, step, active) =>
            void togglePatternStep(track, step, active)
          }
          onStart={() => void startPattern()}
          onStop={() => void stopPattern()}
          onRecordingChange={(recording) => void changeSequencerRecording(recording)}
          onBpmChange={(bpm) => void changeSequencerBpm(bpm)}
          onLengthChange={(steps) => void changePatternLength(steps)}
          onSwingChange={(swing) => void changeSequencerSwing(swing)}
          onClear={() => void clearCurrentPattern()}
          onLoadDemo={() => void loadDemo()}
        />
      )}

      {pattern && (
        <AIPanel
          source={pattern}
          candidate={aiCandidate}
          decision={aiDecision}
          director={directorResult}
          jevStatus={jevStatus}
          params={aiParams}
          busy={aiBusy}
          onParamsChange={setAiParams}
          onGenerate={(operation) => void generateAI(operation)}
          onDirector={() => void generateDirector()}
          onToggleLock={toggleAiLock}
          onAccept={() => void acceptAI()}
          onDiscard={() => void discardAI()}
        />
      )}

      <MidiLearnPanel
        armedTarget={learnTarget}
        mappings={learnedMappings}
        savedPath={savedProfilePath}
        busy={busy}
        onArm={armLearn}
        onSave={() => void saveDevelopmentProfile()}
      />

      <MidiEventTable events={events} />
    </main>
  );
}

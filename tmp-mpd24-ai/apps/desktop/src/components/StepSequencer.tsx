import type { SequencerPattern } from "../types";
import { PAD_LABELS } from "../virtualPads";

type Props = {
  pattern: SequencerPattern;
  previewPattern: SequencerPattern | null;
  playhead: number | null;
  playing: boolean;
  recording: boolean;
  busy: boolean;
  defaultVelocity: number;
  onToggleStep: (track: number, step: number, active: boolean) => void;
  onStart: () => void;
  onStop: () => void;
  onRecordingChange: (recording: boolean) => void;
  onBpmChange: (bpm: number) => void;
  onLengthChange: (steps: number) => void;
  onSwingChange: (swing: number) => void;
  onClear: () => void;
  onLoadDemo: () => void;
};

export function StepSequencer({
  pattern,
  previewPattern,
  playhead,
  playing,
  recording,
  busy,
  defaultVelocity,
  onToggleStep,
  onStart,
  onStop,
  onRecordingChange,
  onBpmChange,
  onLengthChange,
  onSwingChange,
  onClear,
  onLoadDemo
}: Props) {
  return (
    <section className="panel sequencer-panel">
      <div className="sequencer-toolbar">
        <div>
          <div className="panel-title">Step Sequencer</div>
          <p className="hint compact">
            Clock nativo Rust. I pad virtuali e il sequencer usano lo stesso sampler.
          </p>
        </div>

        <div className="transport-buttons">
          {playing ? (
            <button disabled={busy} onClick={onStop}>
              ■ Stop
            </button>
          ) : (
            <button disabled={busy} onClick={onStart}>
              ▶ Play
            </button>
          )}
          <button disabled={busy} onClick={onClear}>
            Clear
          </button>
          <button disabled={busy} onClick={onLoadDemo}>
            Demo
          </button>
          <button
            className={recording ? "recording-button" : ""}
            disabled={busy || !playing}
            onClick={() => onRecordingChange(!recording)}
          >
            {recording ? "● REC ON" : "○ REC"}
          </button>
        </div>

        <label className="compact-control">
          BPM
          <input
            type="number"
            min="20"
            max="320"
            step="1"
            value={Math.round(pattern.bpm)}
            onChange={(event) => {
              const value = Number(event.target.value);
              if (value >= 20 && value <= 320) {
                onBpmChange(value);
              }
            }}
          />
        </label>

        <label className="compact-control">
          Steps
          <select
            value={pattern.totalSteps}
            onChange={(event) => onLengthChange(Number(event.target.value))}
          >
            <option value={16}>16</option>
            <option value={32}>32</option>
            <option value={64}>64</option>
          </select>
        </label>

        <label className="swing-control">
          Swing {Math.round(pattern.swing * 100)}%
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={pattern.swing}
            onChange={(event) => onSwingChange(Number(event.target.value))}
          />
        </label>
      </div>

      <div className="sequencer-scroll">
        <table className="sequencer-table">
          <thead>
            <tr>
              <th className="track-name-cell">Track</th>
              {Array.from({ length: pattern.totalSteps }, (_, step) => (
                <th
                  className={playhead === step ? "playhead-heading" : ""}
                  key={step}
                >
                  {step + 1}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {pattern.tracks.map((track, trackIndex) => (
              <tr key={track.pad}>
                <th className="track-name-cell">
                  {PAD_LABELS[track.pad] ?? `Pad ${track.pad + 1}`}
                </th>
                {Array.from({ length: pattern.totalSteps }, (_, stepIndex) => {
                  const step = track.steps[stepIndex];
                  const active = Boolean(step?.active);
                  const previewStep =
                    previewPattern?.tracks[trackIndex]?.steps[stepIndex] ?? null;
                  const previewActive = Boolean(previewStep?.active);
                  const previewAdded = !active && previewActive;
                  const previewRemoved = active && previewStep !== null && !previewActive;
                  const previewChanged =
                    active &&
                    previewActive &&
                    previewStep !== null &&
                    (previewStep.velocity !== step.velocity ||
                      previewStep.microOffsetMicros !== step.microOffsetMicros);
                  return (
                    <td key={stepIndex}>
                      <button
                        aria-label={`${PAD_LABELS[track.pad] ?? "Pad"} step ${stepIndex + 1}`}
                        className={[
                          "step-cell",
                          active ? "active" : "",
                          playhead === stepIndex ? "playhead" : "",
                          previewAdded ? "preview-added" : "",
                          previewRemoved ? "preview-removed" : "",
                          previewChanged ? "preview-changed" : ""
                        ]
                          .filter(Boolean)
                          .join(" ")}
                        onPointerDown={() =>
                          onToggleStep(trackIndex, stepIndex, !active)
                        }
                        title={
                          active
                            ? `Velocity ${step.velocity}`
                            : `Add at velocity ${defaultVelocity}`
                        }
                      >
                        {previewAdded ? "+" : previewRemoved ? "×" : active ? "●" : ""}
                      </button>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

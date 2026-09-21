import { useState } from "react";
import type { KitPadSnapshot, SampleLibraryItem } from "../types";
import { PAD_LABELS } from "../virtualPads";

type Props = {
  kit: KitPadSnapshot[];
  library: SampleLibraryItem[];
  busyPad: number | null;
  sampleEditingLocked: boolean;
  onImport: (pad: number) => void;
  onAssign: (pad: number, file: string) => void;
  onReset: (pad: number) => void;
  onMixerChange: (pad: number, gain: number, pan: number) => void;
  onPlaybackChange: (
    pad: number,
    pitchSemitones: number,
    reverse: boolean,
    sampleStart: number,
    sampleEnd: number,
    chokeGroup: number | null
  ) => void;
};

export function KitMixerPanel({
  kit,
  library,
  busyPad,
  sampleEditingLocked,
  onImport,
  onAssign,
  onReset,
  onMixerChange,
  onPlaybackChange
}: Props) {
  const [selectedFiles, setSelectedFiles] = useState<Record<number, string>>({});

  return (
    <section className="panel kit-mixer-panel">
      <div className="panel-title">Sample Library & Pad Mixer</div>
      <p className="hint compact">
        I WAV importati vengono copiati nella libreria privata dell'app e restano riutilizzabili.
      </p>
      {sampleEditingLocked && (
        <p className="hint compact realtime-lock-note">
          Playback attivo: sostituzione sample e reset sono temporaneamente bloccati.
          Gain, pan, pitch, trim e choke restano modificabili.
        </p>
      )}

      <div className="kit-mixer-scroll">
        <table className="kit-mixer-table">
          <thead>
            <tr>
              <th>Pad</th>
              <th>Sample</th>
              <th>Library</th>
              <th>Gain</th>
              <th>Playback</th>
              <th>Pan</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {kit.map((slot, pad) => (
              <tr key={pad}>
                <th>
                  {pad + 1} · {PAD_LABELS[pad] ?? "Pad"}
                </th>
                <td>
                  <strong>{slot.displayName}</strong>
                  <span className="kit-source">
                    {slot.sampleFile ? "Library WAV" : "Built-in demo"}
                  </span>
                  {slot.velocityLayers.length > 0 && (
                    <span className="kit-source">
                      {slot.velocityLayers.length} velocity layer
                      {slot.velocityLayers.length === 1 ? "" : "s"}
                    </span>
                  )}
                </td>
                <td>
                  <div className="library-assign-row">
                    <select
                      value={selectedFiles[pad] ?? ""}
                      disabled={
                        library.length === 0 ||
                        busyPad === pad ||
                        sampleEditingLocked
                      }
                      onChange={(event) =>
                        setSelectedFiles((current) => ({
                          ...current,
                          [pad]: event.target.value
                        }))
                      }
                    >
                      <option value="">Choose imported WAV</option>
                      {library.map((sample) => (
                        <option key={sample.file} value={sample.file}>
                          {sample.displayName}
                        </option>
                      ))}
                    </select>
                    <button
                      disabled={
                        !selectedFiles[pad] ||
                        busyPad === pad ||
                        sampleEditingLocked
                      }
                      onClick={() => onAssign(pad, selectedFiles[pad])}
                    >
                      Assign
                    </button>
                  </div>
                </td>
                <td>
                  <label className="mixer-slider">
                    {Math.round(slot.gain * 100)}%
                    <input
                      type="range"
                      min="0"
                      max="2"
                      step="0.01"
                      value={slot.gain}
                      onChange={(event) =>
                        onMixerChange(pad, Number(event.target.value), slot.pan)
                      }
                    />
                  </label>
                </td>
                <td>
                  <div className="playback-controls">
                    <label>
                      Pitch {slot.pitchSemitones > 0 ? "+" : ""}
                      {slot.pitchSemitones.toFixed(0)} st
                      <input
                        type="range"
                        min="-24"
                        max="24"
                        step="1"
                        value={slot.pitchSemitones}
                        onChange={(event) =>
                          onPlaybackChange(
                            pad,
                            Number(event.target.value),
                            slot.reverse,
                            slot.sampleStart,
                            slot.sampleEnd,
                            slot.chokeGroup
                          )
                        }
                      />
                    </label>
                    <label className="reverse-toggle">
                      <input
                        type="checkbox"
                        checked={slot.reverse}
                        onChange={(event) =>
                          onPlaybackChange(
                            pad,
                            slot.pitchSemitones,
                            event.target.checked,
                            slot.sampleStart,
                            slot.sampleEnd,
                            slot.chokeGroup
                          )
                        }
                      />
                      Reverse
                    </label>
                    <label>
                      Start {Math.round(slot.sampleStart * 100)}%
                      <input
                        type="range"
                        min="0"
                        max="0.99"
                        step="0.01"
                        value={slot.sampleStart}
                        onChange={(event) =>
                          onPlaybackChange(
                            pad,
                            slot.pitchSemitones,
                            slot.reverse,
                            Math.min(Number(event.target.value), slot.sampleEnd - 0.01),
                            slot.sampleEnd,
                            slot.chokeGroup
                          )
                        }
                      />
                    </label>
                    <label>
                      End {Math.round(slot.sampleEnd * 100)}%
                      <input
                        type="range"
                        min="0.01"
                        max="1"
                        step="0.01"
                        value={slot.sampleEnd}
                        onChange={(event) =>
                          onPlaybackChange(
                            pad,
                            slot.pitchSemitones,
                            slot.reverse,
                            slot.sampleStart,
                            Math.max(Number(event.target.value), slot.sampleStart + 0.01),
                            slot.chokeGroup
                          )
                        }
                      />
                    </label>
                    <label>
                      Choke
                      <select
                        value={slot.chokeGroup ?? 0}
                        onChange={(event) => {
                          const group = Number(event.target.value);
                          onPlaybackChange(
                            pad,
                            slot.pitchSemitones,
                            slot.reverse,
                            slot.sampleStart,
                            slot.sampleEnd,
                            group === 0 ? null : group
                          );
                        }}
                      >
                        <option value={0}>Off</option>
                        {Array.from({ length: 8 }, (_, index) => index + 1).map(
                          (group) => (
                            <option key={group} value={group}>
                              {group}
                            </option>
                          )
                        )}
                      </select>
                    </label>
                  </div>
                </td>
                <td>
                  <label className="mixer-slider">
                    {slot.pan < -0.02
                      ? "L " + Math.round(Math.abs(slot.pan) * 100)
                      : slot.pan > 0.02
                        ? "R " + Math.round(slot.pan * 100)
                        : "C"}
                    <input
                      type="range"
                      min="-1"
                      max="1"
                      step="0.01"
                      value={slot.pan}
                      onChange={(event) =>
                        onMixerChange(pad, slot.gain, Number(event.target.value))
                      }
                    />
                  </label>
                </td>
                <td>
                  <div className="kit-actions">
                    <button
                      disabled={busyPad === pad || sampleEditingLocked}
                      onClick={() => onImport(pad)}
                    >
                      {busyPad === pad ? "Loading…" : "Import WAV"}
                    </button>
                    <button
                      disabled={
                        busyPad === pad ||
                        !slot.sampleFile ||
                        sampleEditingLocked
                      }
                      onClick={() => onReset(pad)}
                    >
                      Demo
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

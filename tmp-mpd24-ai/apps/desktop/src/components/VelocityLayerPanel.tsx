import { useEffect, useMemo, useRef, useState } from "react";
import type {
  KitPadSnapshot,
  SampleLibraryItem,
  VelocityLayerSnapshot
} from "../types";
import { PAD_LABELS } from "../virtualPads";
import {
  cloneVelocityLayers,
  createVelocityPreset,
  firstFreeVelocityRange,
  MAX_ROUND_ROBIN_VARIANTS,
  MAX_VELOCITY_LAYERS,
  validateVelocityLayers,
  velocityCoverageGaps,
  velocityLayerIndexFor
} from "../velocityLayers";

type Props = {
  kit: KitPadSnapshot[];
  library: SampleLibraryItem[];
  busyPad: number | null;
  resetRevision: number;
  auditionVelocity: number;
  sampleEditingLocked: boolean;
  onApply: (pad: number, layers: VelocityLayerSnapshot[]) => void;
};

export function VelocityLayerPanel({
  kit,
  library,
  busyPad,
  resetRevision,
  auditionVelocity,
  sampleEditingLocked,
  onApply
}: Props) {
  const [selectedPad, setSelectedPad] = useState(0);
  const [draft, setDraft] = useState<VelocityLayerSnapshot[]>([]);
  const [selectedFiles, setSelectedFiles] = useState<Record<number, string>>({});
  const slot = kit[selectedPad];
  const persistedSignature = JSON.stringify(slot?.velocityLayers ?? []);
  const lastPersistedSignature = useRef(persistedSignature);
  const draftCache = useRef<Record<number, VelocityLayerSnapshot[]>>({});

  useEffect(() => {
    const previousPersisted = lastPersistedSignature.current;
    lastPersistedSignature.current = persistedSignature;

    setDraft((current) => {
      const draftWasClean = JSON.stringify(current) === previousPersisted;
      if (!draftWasClean) {
        draftCache.current[selectedPad] = cloneVelocityLayers(current);
        return current;
      }
      delete draftCache.current[selectedPad];
      return cloneVelocityLayers(slot?.velocityLayers ?? []);
    });
  }, [persistedSignature, selectedPad, slot?.velocityLayers]);

  useEffect(() => {
    draftCache.current = {};
    setDraft(cloneVelocityLayers(kit[selectedPad]?.velocityLayers ?? []));
    setSelectedFiles({});
    lastPersistedSignature.current = JSON.stringify(
      kit[selectedPad]?.velocityLayers ?? []
    );
  }, [resetRevision]);

  const validationError = useMemo(
    () => validateVelocityLayers(draft),
    [draft]
  );
  const activeLayerIndex = useMemo(
    () => velocityLayerIndexFor(draft, auditionVelocity),
    [draft, auditionVelocity]
  );
  const coverageGaps = useMemo(() => velocityCoverageGaps(draft), [draft]);
  const dirty = JSON.stringify(draft) !== persistedSignature;

  function updateLayer(index: number, patch: Partial<VelocityLayerSnapshot>) {
    setDraft((current) =>
      current.map((layer, layerIndex) =>
        layerIndex === index ? { ...layer, ...patch } : layer
      )
    );
  }

  function changeSelectedPad(nextPad: number) {
    if (dirty) {
      draftCache.current[selectedPad] = cloneVelocityLayers(draft);
    } else {
      delete draftCache.current[selectedPad];
    }
    setSelectedPad(nextPad);
    setDraft(
      cloneVelocityLayers(
        draftCache.current[nextPad] ?? kit[nextPad]?.velocityLayers ?? []
      )
    );
    setSelectedFiles({});
    lastPersistedSignature.current = JSON.stringify(
      kit[nextPad]?.velocityLayers ?? []
    );
  }

  function addLayer() {
    const range = firstFreeVelocityRange(draft);
    if (!range || draft.length >= MAX_VELOCITY_LAYERS) {
      return;
    }
    setDraft((current) => [
      ...current,
      {
        ...range,
        sampleFiles: [],
        roundRobin: false
      }
    ]);
  }

  function addSample(layerIndex: number) {
    const file = selectedFiles[layerIndex];
    if (!file) {
      return;
    }
    setDraft((current) =>
      current.map((layer, index) =>
        index === layerIndex &&
        !layer.sampleFiles.includes(file) &&
        layer.sampleFiles.length < MAX_ROUND_ROBIN_VARIANTS
          ? { ...layer, sampleFiles: [...layer.sampleFiles, file] }
          : layer
      )
    );
    setSelectedFiles((current) => ({ ...current, [layerIndex]: "" }));
  }

  function applyPreset(zones: 2 | 3) {
    setDraft(createVelocityPreset(zones));
  }

  return (
    <section className="panel velocity-layer-panel">
      <div className="velocity-layer-header">
        <div>
          <div className="panel-title">Velocity Layers & Round Robin</div>
          <p className="hint compact">
            Velocity sceglie il layer; più WAV nello stesso layer possono alternarsi a ogni hit.
          </p>
        </div>
        <label className="compact-control">
          Pad
          <select
            value={selectedPad}
            onChange={(event) => changeSelectedPad(Number(event.target.value))}
          >
            {kit.map((pad, index) => (
              <option key={index} value={index}>
                {index + 1} · {PAD_LABELS[index] ?? "Pad"} · {pad.displayName}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="velocity-layer-toolbar">
        <button onClick={() => applyPreset(2)}>2-zone preset</button>
        <button onClick={() => applyPreset(3)}>3-zone preset</button>
        <button
          disabled={
            draft.length >= MAX_VELOCITY_LAYERS ||
            firstFreeVelocityRange(draft) === null
          }
          onClick={addLayer}
        >
          Add layer
        </button>
        <button disabled={draft.length === 0} onClick={() => setDraft([])}>
          Clear
        </button>
        <span className="velocity-audition">
          Test velocity {auditionVelocity}
          {" · "}
          {activeLayerIndex === null
            ? "primary sample fallback"
            : `layer ${activeLayerIndex + 1}`}
        </span>
      </div>

      {draft.length === 0 ? (
        <div className="velocity-empty">
          No velocity layers. The primary pad sample handles velocities 1–127.
        </div>
      ) : (
        <div className="velocity-layer-list">
          {draft.map((layer, layerIndex) => (
            <article
              className={
                "velocity-layer-card" +
                (layerIndex === activeLayerIndex
                  ? " velocity-layer-card-active"
                  : "")
              }
              key={layerIndex}
            >
              <div className="velocity-range-row">
                <strong>Layer {layerIndex + 1}</strong>
                <label>
                  Min
                  <input
                    type="number"
                    min="1"
                    max="127"
                    value={layer.minVelocity}
                    onChange={(event) =>
                      updateLayer(layerIndex, {
                        minVelocity: Number(event.target.value)
                      })
                    }
                  />
                </label>
                <label>
                  Max
                  <input
                    type="number"
                    min="1"
                    max="127"
                    value={layer.maxVelocity}
                    onChange={(event) =>
                      updateLayer(layerIndex, {
                        maxVelocity: Number(event.target.value)
                      })
                    }
                  />
                </label>
                <label className="reverse-toggle">
                  <input
                    type="checkbox"
                    checked={layer.roundRobin}
                    onChange={(event) =>
                      updateLayer(layerIndex, { roundRobin: event.target.checked })
                    }
                  />
                  Round robin
                </label>
                <button
                  onClick={() =>
                    setDraft((current) =>
                      current.filter((_, index) => index !== layerIndex)
                    )
                  }
                >
                  Remove layer
                </button>
              </div>

              <div className="velocity-sample-add">
                <select
                  value={selectedFiles[layerIndex] ?? ""}
                  disabled={
                    library.length === 0 ||
                    layer.sampleFiles.length >= MAX_ROUND_ROBIN_VARIANTS
                  }
                  onChange={(event) =>
                    setSelectedFiles((current) => ({
                      ...current,
                      [layerIndex]: event.target.value
                    }))
                  }
                >
                  <option value="">Choose library WAV</option>
                  {library.map((sample) => (
                    <option key={sample.file} value={sample.file}>
                      {sample.displayName}
                    </option>
                  ))}
                </select>
                <button
                  disabled={
                    !selectedFiles[layerIndex] ||
                    layer.sampleFiles.length >= MAX_ROUND_ROBIN_VARIANTS
                  }
                  onClick={() => addSample(layerIndex)}
                >
                  Add variant
                </button>
                <span>
                  {layer.sampleFiles.length} variant
                  {layer.sampleFiles.length === 1 ? "" : "s"}
                </span>
              </div>

              <div className="velocity-sample-chips">
                {layer.sampleFiles.map((file) => {
                  const sample = library.find((item) => item.file === file);
                  return (
                    <button
                      className="sample-chip"
                      key={file}
                      title="Remove this variant"
                      onClick={() =>
                        updateLayer(layerIndex, {
                          sampleFiles: layer.sampleFiles.filter((item) => item !== file)
                        })
                      }
                    >
                      {sample?.displayName ?? file} ×
                    </button>
                  );
                })}
              </div>
            </article>
          ))}
        </div>
      )}

      <div className="velocity-layer-footer">
        <div>
          {validationError ? (
            <span className="velocity-error">{validationError}</span>
          ) : (
            <span className="velocity-ok">
              {draft.length === 0
                ? "Primary sample fallback only."
                : "Layer map valid and ready to apply."}
            </span>
          )}
          {!validationError && coverageGaps.length > 0 && draft.length > 0 && (
            <span className="velocity-gap-note">
              Fallback ranges:{" "}
              {coverageGaps
                .map((gap) =>
                  gap.minVelocity === gap.maxVelocity
                    ? String(gap.minVelocity)
                    : `${gap.minVelocity}-${gap.maxVelocity}`
                )
                .join(", ")}
            </span>
          )}
          {sampleEditingLocked && (
            <span className="velocity-gap-note">
              Playback attivo: puoi preparare i layer, ma Apply Layers sarà disponibile
              dopo Stop.
            </span>
          )}
        </div>
        <button
          className="primary-action"
          disabled={
            !dirty ||
            validationError !== null ||
            busyPad === selectedPad ||
            sampleEditingLocked
          }
          onClick={() => onApply(selectedPad, cloneVelocityLayers(draft))}
        >
          {busyPad === selectedPad ? "Applying…" : "Apply Layers"}
        </button>
      </div>
    </section>
  );
}

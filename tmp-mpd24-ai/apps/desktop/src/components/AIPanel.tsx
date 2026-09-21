import { activeHitCount, changedStepCount } from "../groovePreview";
import type {
  DirectedGroovePreview,
  GrooveOperation,
  GrooveParams,
  HybridGroovePreview,
  JevStatus,
  SequencerPattern
} from "../types";
import { PAD_LABELS } from "../virtualPads";

const OPERATIONS: Array<{ id: GrooveOperation; label: string }> = [
  { id: "complete", label: "Complete" },
  { id: "variation", label: "Variation" },
  { id: "fill", label: "Fill" },
  { id: "humanize", label: "Humanize" },
  { id: "simplify", label: "Simplify" },
  { id: "complexify", label: "Complexify" }
];

type Props = {
  source: SequencerPattern;
  candidate: SequencerPattern | null;
  decision: HybridGroovePreview | null;
  director: DirectedGroovePreview | null;
  jevStatus: JevStatus;
  params: GrooveParams;
  busy: boolean;
  onParamsChange: (params: GrooveParams) => void;
  onGenerate: (operation: GrooveOperation) => void;
  onDirector: () => void;
  onToggleLock: (track: number) => void;
  onAccept: () => void;
  onDiscard: () => void;
};

export function AIPanel({
  source,
  candidate,
  decision,
  director,
  jevStatus,
  params,
  busy,
  onParamsChange,
  onGenerate,
  onDirector,
  onToggleLock,
  onAccept,
  onDiscard
}: Props) {
  const sourceHits = activeHitCount(source);
  const candidateHits = candidate ? activeHitCount(candidate) : 0;
  const changed = candidate ? changedStepCount(source, candidate) : 0;

  return (
    <section className="panel ai-panel">
      <div className="ai-header">
        <div>
          <div className="panel-title">Groove AI · Hybrid + Jev</div>
          <p className="hint compact">
            Fan-out locale di più groove, reranking decisionale con TypeSafe Jev quando configurato.
          </p>
        </div>
        <div className="ai-status-stack">
          <div className="seed-chip">Seed {params.seed}</div>
          <div className={`jev-chip ${jevStatus.configured ? "online" : "offline"}`}>
            Jev {jevStatus.configured ? "ready" : "not configured"} · {jevStatus.model}
          </div>
        </div>
      </div>

      <div className="ai-operation-grid">
        <button
          className="director-button"
          disabled={busy}
          onClick={onDirector}
        >
          ★ Jev Director
        </button>
        {OPERATIONS.map((operation) => (
          <button
            key={operation.id}
            disabled={busy}
            onClick={() => onGenerate(operation.id)}
          >
            {operation.label}
          </button>
        ))}
      </div>

      {director && (
        <div className="director-plan">
          <div>
            <strong>
              Director: {director.plan.direction} · {director.plan.operation} ·{" "}
              {director.plan.intensity}
            </strong>
            <span>
              planner {director.plan.provider}
              {director.plan.model ? " · " + director.plan.model : ""}
              {" · "}memory {director.plan.memoryAccepted}/{director.plan.memoryEvents} accepted
            </span>
          </div>
          <div>
            {director.plan.confidence !== null && (
              <span>
                plan confidence {Math.round(director.plan.confidence * 100)}%
              </span>
            )}
            {director.plan.worthChanging !== null && (
              <span>
                worth changing {Math.round(director.plan.worthChanging * 100)}%
              </span>
            )}
            {!director.preview && <strong className="hold-label">HOLD · keep current groove</strong>}
            {director.plan.fallbackReason && (
              <span className="fallback-line">{director.plan.fallbackReason}</span>
            )}
          </div>
        </div>
      )}

      <div className="ai-sliders">
        <label>
          AI Amount {Math.round(params.amount * 100)}%
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={params.amount}
            onChange={(event) =>
              onParamsChange({ ...params, amount: Number(event.target.value) })
            }
          />
        </label>
        <label>
          Density {Math.round(params.density * 100)}%
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={params.density}
            onChange={(event) =>
              onParamsChange({ ...params, density: Number(event.target.value) })
            }
          />
        </label>
        <label>
          Complexity {Math.round(params.complexity * 100)}%
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={params.complexity}
            onChange={(event) =>
              onParamsChange({ ...params, complexity: Number(event.target.value) })
            }
          />
        </label>
        <label>
          Humanize {Math.round(params.humanize * 100)}%
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={params.humanize}
            onChange={(event) =>
              onParamsChange({ ...params, humanize: Number(event.target.value) })
            }
          />
        </label>
      </div>

      <div className="panel-title ai-subtitle">Groove Lock</div>
      <div className="lock-grid">
        {PAD_LABELS.map((label, track) => {
          const locked = params.lockedTracks.includes(track);
          return (
            <button
              className={locked ? "locked" : ""}
              key={label}
              onClick={() => onToggleLock(track)}
            >
              {locked ? "🔒 " : ""}
              {label}
            </button>
          );
        })}
      </div>

      {candidate && (
        <div className="candidate-box">
          <div>
            <strong>Candidate ready</strong>
            <span>
              {sourceHits} → {candidateHits} hits · {changed} changed steps
            </span>
            {decision && (
              <span className="decision-line">
                {decision.provider === "jev"
                  ? `Jev selected ${decision.selectedId} from ${decision.candidateCount}`
                  : `Local reranker selected ${decision.selectedId} from ${decision.candidateCount}`}
                {" · "}local score {Math.round(decision.localScore * 100)}%
              </span>
            )}
            {decision?.jev && (
              <span className="decision-line">
                {decision.jev.model} · confidence {Math.round(decision.jev.confidence * 100)}%
                {" · "}safe preview {Math.round(decision.jev.safeToPreview * 100)}%
              </span>
            )}
            {decision?.fallbackReason && (
              <span className="fallback-line">{decision.fallbackReason}</span>
            )}
          </div>
          <div className="candidate-actions">
            <button disabled={busy} onClick={onDiscard}>
              Discard
            </button>
            <button className="primary-action" disabled={busy} onClick={onAccept}>
              Apply
            </button>
          </div>
        </div>
      )}
    </section>
  );
}

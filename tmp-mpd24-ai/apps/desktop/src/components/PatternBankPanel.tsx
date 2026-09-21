import type {
  LaunchQuantization,
  PatternBankStatus
} from "../types";

const SLOT_LABELS = ["A", "B", "C", "D", "E", "F", "G", "H"] as const;

type Props = {
  status: PatternBankStatus;
  quantization: LaunchQuantization;
  busy: boolean;
  onQuantizationChange: (value: LaunchQuantization) => void;
  onLaunch: (slot: number) => void;
  onCopy: (slot: number) => void;
};

export function PatternBankPanel({
  status,
  quantization,
  busy,
  onQuantizationChange,
  onLaunch,
  onCopy
}: Props) {
  return (
    <section className="panel pattern-bank-panel">
      <div className="pattern-bank-header">
        <div>
          <div className="panel-title">Patterns A-H</div>
          <p className="hint compact">
            Prepara variazioni in slot separati e lanciale sul confine musicale scelto.
          </p>
        </div>
        <label className="compact-control pattern-quantize-control">
          Launch
          <select
            value={quantization}
            onChange={(event) =>
              onQuantizationChange(event.target.value as LaunchQuantization)
            }
          >
            <option value="immediate">Immediate</option>
            <option value="nextBeat">Next Beat</option>
            <option value="nextBar">Next Bar</option>
            <option value="nextTwoBars">Next 2 Bars</option>
          </select>
        </label>
      </div>

      <div className="pattern-slot-grid">
        {SLOT_LABELS.map((label, slot) => {
          const active = status.activeSlot === slot;
          const queued = status.queuedSlot === slot;
          return (
            <div
              className={[
                "pattern-slot",
                active ? "active" : "",
                queued ? "queued" : ""
              ]
                .filter(Boolean)
                .join(" ")}
              key={label}
            >
              <button
                className="pattern-launch-button"
                disabled={busy}
                onClick={() => onLaunch(slot)}
              >
                <strong>{label}</strong>
                <span>
                  {active
                    ? "ACTIVE"
                    : queued
                      ? "QUEUED"
                      : status.names[slot] ?? "Pattern " + label}
                </span>
              </button>
              {!active && (
                <button
                  className="pattern-copy-button"
                  disabled={busy}
                  onClick={() => onCopy(slot)}
                  title={"Copy active pattern to " + label}
                >
                  Copy
                </button>
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}

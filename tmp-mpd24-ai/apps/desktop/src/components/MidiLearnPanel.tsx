import type { LearnedMapping } from "../midiLearn";
import { PAD_LABELS } from "../virtualPads";

type Props = {
  armedTarget: string;
  mappings: LearnedMapping[];
  savedPath: string;
  busy: boolean;
  onArm: (target: string) => void;
  onSave: () => void;
};

export function MidiLearnPanel({
  armedTarget,
  mappings,
  savedPath,
  busy,
  onArm,
  onSave
}: Props) {
  return (
    <section className="panel midi-learn-panel">
      <div className="learn-header">
        <div>
          <div className="panel-title">MIDI Learn</div>
          <p className="hint compact">
            Seleziona una destinazione, arma Learn e poi invia un evento reale o usa i pad virtuali.
          </p>
        </div>
        <button disabled={busy || mappings.length === 0} onClick={onSave}>
          Save development profile
        </button>
      </div>

      <div className="learn-grid">
        {PAD_LABELS.map((label, index) => {
          const target = `pad-${index + 1}`;
          const mapping = mappings.find((item) => item.controlId === target);
          return (
            <button
              className={`learn-target ${armedTarget === target ? "armed" : ""}`}
              key={target}
              onClick={() => onArm(armedTarget === target ? "" : target)}
            >
              <strong>{label}</strong>
              <span>
                {armedTarget === target
                  ? "Waiting for MIDI…"
                  : mapping
                    ? `${mapping.messageKind} ch ${mapping.channel || "—"} · ${mapping.data1}`
                    : "Not mapped"}
              </span>
            </button>
          );
        })}
      </div>

      {savedPath && <p className="hint">Saved: {savedPath}</p>}
    </section>
  );
}

import { PAD_KEYS, PAD_LABELS } from "../virtualPads";

type Props = {
  velocity: number;
  padNames?: string[];
  disabled: boolean;
  onVelocityChange: (velocity: number) => void;
  onTrigger: (pad: number) => void;
};

export function VirtualPadGrid({
  velocity,
  padNames,
  disabled,
  onVelocityChange,
  onTrigger
}: Props) {
  return (
    <section className="panel virtual-pad-panel">
      <div className="pad-panel-header">
        <div>
          <div className="panel-title">Virtual MPD / Demo Kit</div>
          <p className="hint compact">
            Mouse or keyboard. This is an internal simulation and does not define the real MPD24
            mapping.
          </p>
        </div>
        <label className="velocity-control">
          <span>Velocity {velocity}</span>
          <input
            type="range"
            min="1"
            max="127"
            value={velocity}
            onChange={(event) => onVelocityChange(Number(event.target.value))}
          />
        </label>
      </div>

      <div className="virtual-pad-grid">
        {PAD_LABELS.map((label, pad) => (
          <button
            className="virtual-pad"
            key={label}
            disabled={disabled}
            onPointerDown={() => onTrigger(pad)}
          >
            <strong>{padNames?.[pad] ?? label}</strong>
            <span>PAD {pad + 1} · {label}</span>
            <kbd>{PAD_KEYS[pad].toUpperCase()}</kbd>
          </button>
        ))}
      </div>
    </section>
  );
}

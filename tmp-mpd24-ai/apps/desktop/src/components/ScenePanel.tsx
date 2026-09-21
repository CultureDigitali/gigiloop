import type { PerformanceScene } from "../types";

const SCENE_NAMES = [
  "Intro",
  "Verse",
  "Build",
  "Drop",
  "Break",
  "Drop 2",
  "Outro",
  "Custom"
] as const;

type Props = {
  scenes: Array<PerformanceScene | null>;
  busy: boolean;
  onCapture: (index: number, name: string) => void;
  onRecall: (index: number) => void;
  onClear: (index: number) => void;
};

export function ScenePanel({
  scenes,
  busy,
  onCapture,
  onRecall,
  onClear
}: Props) {
  return (
    <section className="panel scene-panel">
      <div className="panel-title">Performance Scenes</div>
      <p className="hint compact">
        Ogni scena richiama pattern, master, quantizzazione e stato Groove AI.
      </p>

      <div className="scene-grid">
        {SCENE_NAMES.map((name, index) => {
          const scene = scenes[index];
          const slotLabel = scene
            ? String.fromCharCode("A".charCodeAt(0) + scene.patternSlot)
            : "—";
          return (
            <div className={scene ? "scene-card captured" : "scene-card"} key={name}>
              <button
                className="scene-recall"
                disabled={busy || !scene}
                onClick={() => onRecall(index)}
              >
                <strong>{name}</strong>
                <span>{scene ? "Pattern " + slotLabel : "Empty"}</span>
              </button>
              <div className="scene-actions">
                <button disabled={busy} onClick={() => onCapture(index, name)}>
                  {scene ? "Update" : "Capture"}
                </button>
                {scene && (
                  <button disabled={busy} onClick={() => onClear(index)}>
                    Clear
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

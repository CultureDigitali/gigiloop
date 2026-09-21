import type { ProjectSummary } from "../types";

type Props = {
  projectName: string;
  currentProjectId: number | null;
  selectedProjectId: number | null;
  projects: ProjectSummary[];
  busy: boolean;
  loadLocked: boolean;
  onNameChange: (name: string) => void;
  onSelectedProjectChange: (id: number | null) => void;
  onSave: () => void;
  onSaveAs: () => void;
  onLoad: () => void;
  onDelete: () => void;
};

export function ProjectPanel({
  projectName,
  currentProjectId,
  selectedProjectId,
  projects,
  busy,
  loadLocked,
  onNameChange,
  onSelectedProjectChange,
  onSave,
  onSaveAs,
  onLoad,
  onDelete
}: Props) {
  return (
    <section className="panel project-panel">
      <div>
        <div className="panel-title">Project Library · SQLite</div>
        <p className="hint compact">
          Salva bank A-H, impostazioni performance e parametri Groove AI.
        </p>
        {loadLocked && (
          <p className="hint compact realtime-lock-note">
            Playback attivo: Open è bloccato perché può sostituire il kit audio.
          </p>
        )}
      </div>

      <div className="project-controls">
        <label>
          Current project
          <input
            type="text"
            value={projectName}
            placeholder="Untitled Project"
            onChange={(event) => onNameChange(event.target.value)}
          />
        </label>
        <div className="project-actions">
          <button disabled={busy || !projectName.trim()} onClick={onSave}>
            {currentProjectId === null ? "Save" : "Update"}
          </button>
          <button disabled={busy || !projectName.trim()} onClick={onSaveAs}>
            Save As
          </button>
        </div>
      </div>

      <div className="project-library-row">
        <select
          value={selectedProjectId ?? ""}
          disabled={busy || projects.length === 0}
          onChange={(event) =>
            onSelectedProjectChange(
              event.target.value ? Number(event.target.value) : null
            )
          }
        >
          <option value="">Saved projects</option>
          {projects.map((project) => (
            <option key={project.id} value={project.id}>
              {project.name}
            </option>
          ))}
        </select>
        <button
          disabled={busy || selectedProjectId === null || loadLocked}
          onClick={onLoad}
        >
          Open
        </button>
        <button disabled={busy || selectedProjectId === null} onClick={onDelete}>
          Delete
        </button>
      </div>
    </section>
  );
}

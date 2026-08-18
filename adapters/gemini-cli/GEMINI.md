# GigiLoop adapter for Gemini CLI

Use the canonical `gigiloop/SKILL.md` workflow when the user requests autonomous implementation, testing, critique, and refinement across multiple cycles.

## Protocol

- select `strict`, `balanced`, or `fast`; default to `balanced` and never silently downgrade;
- baseline repository state, failures, and protected local edits before changing code;
- define measurable acceptance criteria and a verification contract;
- implement the highest-impact confirmed improvement;
- run targeted checks each iteration and broader milestone/final checks;
- tie every score to current evidence and current code state;
- adversarially review the diff, tests, assumptions, thresholds, and user-work preservation;
- reconcile scores after critique;
- reject deleted/skipped tests, lowered thresholds, disabled checks, blind snapshot approval, or narrowed scope used merely to obtain a pass;
- avoid destructive operations without explicit authorization and a recovery plan;
- checkpoint progress, detect stale state, and re-strategize after a plateau;
- exit only as `SUCCESS`, `BLOCKED`, `BUDGET EXHAUSTED`, or `STOPPED` with an evidence-backed report.

A plausible result is not a verified result. Read `gigiloop/references/integrity.md` before changing tests, snapshots, quality configuration, local work, data, deployments, or Git history.

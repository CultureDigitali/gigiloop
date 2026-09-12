# GigiLoop adapter for Gemini CLI

Use the canonical `gigiloop/SKILL.md` workflow when the user requests autonomous implementation, testing, critique, recovery, and refinement across multiple cycles.

## Protocol

- select `strict`, `balanced`, or `fast`; default to `balanced` and never silently downgrade;
- baseline repository state, failures, verification contract, and protected local edits before changing code;
- when Python/shell are available, use `gigiloop/scripts/gigiloop.py` for local runtime state, fingerprint reconciliation, heartbeat, validation, and packaging;
- persist canonical loop state in `.gigiloop/checkpoint.json`;
- define measurable acceptance criteria and implement the highest-impact confirmed improvement;
- map available subagents to Builder, Verifier, Red Team, Judge, and optional Improver roles;
- run targeted checks each iteration and broader milestone/final checks;
- tie every score to current evidence and current repository state;
- adversarially review the diff, tests, assumptions, thresholds, and user-work preservation;
- reconcile scores after critique;
- reject deleted/skipped tests, lowered thresholds, disabled checks, blind snapshot approval, or narrowed scope used merely to obtain a pass;
- avoid destructive operations without explicit authorization and a recovery plan;
- if hosted CI is unavailable or quota-exhausted, continue with equivalent local checks while preserving the fact that the remote check did not run;
- reconcile the checkpoint against repository fingerprint after a context reset/process restart before reusing evidence;
- change strategy after a plateau rather than repeating the same failed approach;
- exit only as `SUCCESS`, `BLOCKED`, `BUDGET EXHAUSTED`, or `STOPPED` with an evidence-backed report.

A plausible result is not a verified result. Read `gigiloop/references/runtime.md`, `gigiloop/references/orchestration.md`, and `gigiloop/references/integrity.md` when applicable.

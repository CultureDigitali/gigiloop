# GigiLoop adapter for Codex

Use the canonical workflow in `gigiloop/SKILL.md` when the user asks for repeated autonomous improvement rather than a single pass.

Typical triggers:

- keep iterating until it is done;
- do not stop at the first green test;
- run a Ralph-style or self-improving loop;
- harden this to production quality;
- keep fixing until every criterion is verified.

## Required behavior

1. Select `strict`, `balanced`, or `fast`; default to `balanced` and never silently downgrade.
2. Establish a baseline before editing, including branch/HEAD, pre-existing failures, verification contract, and uncommitted user work.
3. When Python/shell are available, use `gigiloop/scripts/gigiloop.py` for machine checkpointing, repository fingerprint reconciliation, heartbeat, local validation, and packaging.
4. Persist canonical state in `.gigiloop/checkpoint.json`; host task state is only a mirror.
5. Define measurable acceptance criteria and fix the highest-impact confirmed gap.
6. Prefer separate Builder, Verifier, Red Team, Judge, and optional Improver contexts when subagents are available.
7. Run targeted verification during iterations and broader checks at milestones/final gate.
8. Score only with current evidence tied to the current repository state.
9. Reconcile scores after critique; confirmed findings can invalidate a pass.
10. Reject verifier weakening: do not delete/skip tests, lower thresholds, disable checks, or blindly bless snapshots merely to turn output green.
11. Preserve unrelated user changes and avoid destructive operations without explicit authorization.
12. If GitHub Actions/hosted CI is unavailable or quota-exhausted, continue with equivalent local checks and independent review when possible; never invent the remote result.
13. On context/process resume, reconcile `.gigiloop/checkpoint.json` against the current repository fingerprint before trusting stored evidence.
14. Exit only as `SUCCESS`, `BLOCKED`, `BUDGET EXHAUSTED`, or `STOPPED`, using `gigiloop/references/reporting.md`.

Completion requires evidence, not optimism. Read `gigiloop/references/runtime.md`, `gigiloop/references/orchestration.md`, and `gigiloop/references/integrity.md` when their rules apply.

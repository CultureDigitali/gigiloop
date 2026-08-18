# GigiLoop adapter for Codex

Use the canonical workflow in `gigiloop/SKILL.md` when the user asks for repeated autonomous improvement rather than a single pass.

Typical triggers:

- keep iterating until it is done;
- do not stop at the first green test;
- run a Ralph-style or self-improving loop;
- harden this to production quality;
- keep fixing until every criterion is verified.

## Required behavior

1. Select the `strict`, `balanced`, or `fast` profile; default to `balanced` and never silently downgrade.
2. Establish a baseline before editing, including current branch/HEAD, pre-existing failures, and uncommitted user work.
3. Define measurable acceptance criteria and a verification contract.
4. Fix the highest-impact confirmed gap.
5. Run targeted verification during iterations and broader checks at milestones/final gate.
6. Score only with current evidence tied to the current code state.
7. Red-team the diff, tests, assumptions, and quality configuration.
8. Reconcile scores after critique; confirmed findings can invalidate a pass.
9. Reject verifier weakening: do not delete/skip tests, lower thresholds, disable checks, or blindly bless snapshots merely to turn output green.
10. Preserve unrelated user changes and avoid destructive operations without explicit authorization.
11. Checkpoint progress and change strategy after a plateau.
12. Exit only as `SUCCESS`, `BLOCKED`, `BUDGET EXHAUSTED`, or `STOPPED`, using the report contract in `gigiloop/references/reporting.md`.

Completion requires evidence, not optimism. Read `gigiloop/references/integrity.md` before changing tests, quality thresholds, snapshots, local work, migrations, or Git history.

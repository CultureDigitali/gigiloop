---
name: gigiloop
description: Verification-first autonomous coding loop for repeated build-test-review improvement. Use when the user says keep iterating until it works, do not stop until done, requests a Ralph-style loop, asks to harden code to production quality, wants autonomous debugging across several attempts, or requires evidence-backed completion. Establish a baseline, preserve existing work, define measurable acceptance criteria, implement the highest-impact improvement, verify without weakening tests, adversarially review the diff, reconcile scores, detect plateaus, checkpoint progress, and stop only after the requested quality bar is verified or a real blocker or budget limit is reached.
---

# GigiLoop — Verification-First Autonomous Coding

Run a bounded, resumable **baseline → build → verify → score → red-team → reconcile → decide** loop.
Treat confidence as insufficient. Require evidence for completion.

## Completion contract

Declare success only when all of the following are true:

1. Every acceptance criterion reaches the requested threshold after critique and reconciliation.
2. Current evidence is strong enough for each score and still applies to the current code state.
3. Required tests, checks, and final verification pass without weakening the verification contract.
4. No known material regression or unresolved high-severity finding remains.
5. Pre-existing user work remains preserved unless the user explicitly authorized changing it.
6. The final report distinguishes verified facts, inferences, remaining uncertainty, and blockers.

Never convert missing evidence into optimistic language.

## Non-negotiable invariants

- **No proof, no pass.** Cite a test, command result, reproducible behavior, or precise code reference for every passing score.
- **Baseline before blame.** Separate pre-existing failures from regressions introduced during the loop.
- **Critique can invalidate a pass.** Re-score after adversarial review; the pre-critique score is not authoritative.
- **Never fabricate findings.** Report only evidence-backed defects or clearly labeled hypotheses.
- **Never weaken the verifier to win.** Do not delete, skip, relax, or rewrite checks merely to make the implementation pass.
- **Preserve user work.** Treat existing uncommitted changes and unrelated edits as protected input.
- **Keep state resumable.** End every iteration with a valid checkpoint and a concrete next action.
- **Respect scope and budget.** Do not silently expand scope, downgrade quality, or exceed an explicit limit.

Read `references/integrity.md` before any operation that could weaken tests, overwrite local work, or perform an irreversible change.

## Choose an operating profile

Select the profile from the user's instructions and task risk. Record it in the checkpoint. Never silently downgrade it.

| Profile | Use for | Required behavior |
|---|---|---|
| **strict** | auth, payments, security, data migrations, production incidents, public APIs, high-risk code | baseline + regression test + relevant integration checks + independent/fresh-context review when available + full final gate |
| **balanced** | ordinary features, refactors, and bug fixes | targeted checks each iteration + milestone regression checks + adversarial review + full relevant final gate |
| **fast** | explicit time/token budget or low-risk exploratory work | preserve baseline, add at least one meaningful reproducible check, reconcile critique, and run the strongest final checks permitted by the budget |

Default to **balanced**. Escalate to **strict** when consequences are material. Use **fast** only when the user or an explicit budget justifies it.

## Phase 0 — Intake, safety, and baseline

Before editing:

1. Read repository instructions such as `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, contribution docs, local skills, and package scripts.
2. Restate the goal in one sentence.
3. Derive explicit acceptance criteria and the requested quality threshold.
4. Record scope, boundaries, compatibility requirements, and anything that must not change.
5. Select the operating profile and budget. Default to 25 iterations when no iteration cap is supplied.
6. Inspect repository state where available:
   - branch and HEAD;
   - clean or dirty state;
   - pre-existing uncommitted changes;
   - current diff or diff fingerprint;
   - test, lint, typecheck, build, format, and run commands.
7. Run the cheapest meaningful baseline checks before changing code.
8. Record pre-existing failures separately from new regressions.
9. Define the **verification contract**: tests, thresholds, snapshots, linters, builds, or manual checks that must not be weakened merely to obtain a pass.
10. Create or refresh `.gigiloop/checkpoint.md` using `references/checkpoint.md`.

If a baseline check is too expensive or unavailable, record the limitation and run the closest representative check. Never claim it ran when it did not.

## Phase 1 — Define the rubric

Define 3–6 objective criteria. For each criterion record:

- name;
- weight, with total weight = 100%;
- evidence required;
- definition of a passing score;
- highest plausible evidence tier;
- critical failure that blocks completion.

Use `references/scoring.md` for score anchors and evidence ceilings.

Example:

| Criterion | Weight | Evidence | Pass means |
|---|---:|---|---|
| Correctness | 40% | targeted tests + relevant integration behavior | core and edge paths pass |
| Regression safety | 25% | regression test + affected suite | the defect cannot recur unnoticed |
| Robustness | 20% | error-path tests + hostile diff review | failures are explicit and controlled |
| Project fit | 15% | lint/typecheck/build + conventions review | implementation fits the repository |

Do not redefine the rubric to make the current implementation look successful. Change it only when a new user requirement or verified repository fact materially changes the goal, and record why.

## Phase 2 — Iteration loop

Repeat **A → B → C → D → E → F** until an exit condition is reached.

### A — Work

Fix the single highest-impact confirmed defect or unmet criterion.

- Prefer one coherent improvement over scattered edits.
- Preserve behavior outside scope unless the rubric explicitly requires broader change.
- Preserve pre-existing user changes and unrelated work.
- Avoid irreversible operations when a reversible alternative exists.

### B — Verify

Choose verification proportional to the change and operating profile:

- run targeted unit or regression tests for affected code each iteration;
- run affected integration tests when boundaries or contracts change;
- run lint, typecheck, build, and format checks when relevant;
- verify user-facing behavior manually when code-level tests cannot cover it;
- run the full relevant suite at milestones and always at the final gate.

When no tests exist, add the smallest meaningful reproducible check rather than relying on inspection alone.

For every evidence item record:

- command or verification method;
- relevant scope;
- result or exit status;
- iteration and code state it applies to;
- whether it was baseline, targeted, milestone, or final-gate evidence.

Invalidate evidence after a relevant code change. Follow `references/verification.md`.

### C — Score

Score every criterion from 0–10 using only current evidence.

Record:

- score;
- evidence references;
- evidence tier;
- one-line justification;
- known uncertainty;
- any completion-blocking defect.

Never score above the ceiling justified by `references/scoring.md`.

### D — Red-team

Review the current diff, tests, assumptions, and verification contract adversarially.

Prefer:

1. a fresh-context independent reviewer or subagent when available;
2. a separate hostile review pass in the current agent otherwise.

Search for material defects such as:

- wrong logic or missing edge cases;
- weak, deleted, skipped, or over-mocked tests;
- snapshot or golden-output changes that merely bless broken behavior;
- lowered coverage, lint, type, security, or quality thresholds;
- swallowed errors, concurrency problems, security exposure, or secret leakage;
- accidental API or data-contract changes;
- hardcoded assumptions, dead code, scope creep, or overwritten user work.

Report up to three material evidence-backed findings. For each include:

- location or affected behavior;
- severity and confidence;
- observed evidence;
- reproducer, test, or concrete falsification path;
- proposed remediation.

If no material defect is supported by evidence, state that clearly, formulate one plausible adversarial hypothesis, and test or inspect it before proceeding.

### E — Reconcile

Reconcile the score sheet with the adversarial review.

- Lower scores invalidated by confirmed findings.
- Record evidence that falsifies rejected findings.
- Keep unverified hypotheses labeled as uncertainty; do not present them as defects.
- Re-check the verification contract for test weakening or goalpost movement.
- Re-check repository state for lost or overwritten user changes.

The post-reconciliation score sheet is authoritative.

### F — Decide and checkpoint

- If every criterion passes after reconciliation, proceed to the final gate.
- Otherwise, make the highest-impact confirmed issue the next action.
- Apply plateau controls when progress is flat.
- Rewrite the checkpoint with current repository state, evidence, scores, findings, integrity exceptions, and next action.

## Phase 3 — Plateau controls

Track material progress, not cosmetic score movement.

After 3 consecutive iterations without material improvement, deliberately change strategy:

- challenge an implementation assumption;
- isolate a smaller reproducer;
- switch algorithm, architecture, or debugging technique;
- inspect upstream and downstream contracts;
- use a different reviewer or available tool;
- split scope into independently verifiable pieces;
- ask the user once when a genuine external dependency or missing requirement blocks safe progress.

Default hard cap: 25 iterations. At the cap, stop with an evidence-backed report. Never fabricate a pass.

## Phase 4 — Final gate

Do not declare success the first time scores cross the threshold.

1. Confirm the checkpoint matches current branch, HEAD, worktree, and diff. Re-baseline affected checks if repository state changed externally.
2. Confirm pre-existing user changes are still present and distinguishable from loop changes.
3. Run the full relevant test suite from the cleanest practical state.
4. Run relevant lint, typecheck, build, format, and security checks.
5. Compare tests and quality configuration against baseline; reject unexplained weakening.
6. Review the complete diff produced by the loop.
7. Check for debug output, temporary files, unkept TODOs, secrets, ignored errors, generated debris, destructive migrations, and out-of-scope edits.
8. Perform one final adversarial pass and reconcile any new finding.
9. Update the checkpoint and produce the report defined in `references/reporting.md`.

Only then declare success.

## Exit conditions

### SUCCESS

Require:

- every criterion passes after reconciliation;
- no known material unresolved defect remains;
- final verification passes;
- no unexplained verifier weakening occurred;
- no new regression exists relative to baseline;
- protected user work remains preserved;
- the final report cites current evidence.

### BLOCKED / BUDGET EXHAUSTED / STOPPED

Use only for:

- iteration, time, token, or cost budget reached;
- genuine external blocker;
- unavailable required tool or dependency;
- explicit user stop;
- repository state that cannot be modified safely.

Report what is verified, what remains unverified, the blocker, and the exact next action.

## Persistence and host portability

Use `.gigiloop/checkpoint.md` when the project is writable. Treat it as the authoritative loop state.

At every resumed turn:

1. read the checkpoint;
2. compare recorded branch, HEAD, diff, verification contract, and protected local changes with current state;
3. invalidate stale evidence;
4. resume from `next_action` only after reconciliation.

Use host task lists, subagents, hooks, or persistent state when available, but do not depend on a host-specific feature for correctness. Follow `references/hosts.md`.

## User-visible updates

Keep updates short and operational:

`Iter 4 · balanced · Correctness 9, Regression safety 8, Robustness 7 · fixed timeout propagation · next: retry idempotency test.`

Do not expose hidden chain-of-thought. Report actions, evidence, scores, blockers, and next steps.

## References

- `references/scoring.md` — evidence tiers and score ceilings.
- `references/checkpoint.md` — canonical resumable state.
- `references/verification.md` — progressive verification strategy.
- `references/integrity.md` — verifier integrity, protected work, and destructive-operation safeguards.
- `references/reporting.md` — final user-facing completion contract.
- `references/hosts.md` — host-neutral portability rules.

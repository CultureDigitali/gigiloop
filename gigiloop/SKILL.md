---
name: gigiloop
description: Verification-first autonomous coding loop for repeated build-test-review improvement. Use when the user asks an agent to keep iterating until code works, says do not stop until done, requests a Ralph-style loop, wants repeated build-test-review cycles, asks to harden a feature to production quality, or wants evidence-backed autonomous improvement. Define measurable acceptance criteria, establish a baseline, implement and test changes, adversarially review them, reconcile scores after critique, checkpoint progress, recover from idle/crash/context resets, detect plateaus, optionally self-improve after correctness, and stop only when every criterion is verified or a real blocker or budget limit is reached.
---

# GigiLoop — Verification-First Autonomous Coding

Run a bounded, resumable **baseline → build → verify → score → red-team → reconcile → decide** loop.
Treat confidence as insufficient. Require current evidence for completion.

## Completion contract

Declare `SUCCESS` only when all of the following are true:

1. Every critical acceptance criterion reaches the requested threshold after critique and reconciliation.
2. Evidence is current for the repository state it claims to verify.
3. Required tests, checks, and final verification pass without weakening the verification contract.
4. No known material regression or unresolved high-severity finding remains.
5. Pre-existing user work remains preserved unless the user explicitly authorized changing it.
6. Repository/checkpoint state is resumable and the final report distinguishes verified, inferred, unverified, and blocked claims.

Never convert missing evidence into optimistic language.

## Non-negotiable invariants

- **No proof, no pass.** Cite tests, command results, reproducible behavior, or precise code evidence.
- **Baseline before blame.** Separate pre-existing failures from regressions introduced by the loop.
- **Critique can invalidate a pass.** Post-review reconciliation is authoritative.
- **Never fabricate findings.** Report evidence-backed defects or clearly labeled hypotheses only.
- **Never weaken the verifier to win.** Do not delete/skip tests, lower thresholds, disable checks, or blindly bless snapshots merely to get green output.
- **Preserve user work.** Treat existing uncommitted and unrelated edits as protected input.
- **Keep state resumable.** Checkpoint after every material iteration and before intentional stops.
- **Hosted CI is an accelerator, not memory.** The loop must preserve local state when GitHub Actions or another hosted service is unavailable.
- **Respect scope and budget.** Do not silently expand scope, downgrade quality, or exceed explicit limits.

Read `references/integrity.md` before changing tests, thresholds, snapshots, migrations, generated files, Git history, or pre-existing user work.

## Runtime preference

When Python 3 and shell execution are available, use the bundled standard-library runtime in `scripts/gigiloop.py` for checkpointing, repository fingerprints, resume reconciliation, self-tests, packaging, heartbeat, and optional supervision.

Run at intake when practical:

```bash
python <skill-path>/scripts/gigiloop.py doctor --root .
```

Create machine-readable state for a new loop:

```bash
python <skill-path>/scripts/gigiloop.py init --root . --goal "<goal>" --profile balanced
```

Run local verification through the runtime when command execution is available so evidence is recorded against the exact repository fingerprint:

```bash
python <skill-path>/scripts/gigiloop.py verify --root . --kind targeted --tier T3 --check "pytest -q"
```

If a verification command changes the repository, the runtime marks that evidence stale and requires re-baselining instead of accepting the result as current proof.

At a resumed session or process restart:

```bash
python <skill-path>/scripts/gigiloop.py resume --root .
```

A resume result that requires re-baselining means repository drift invalidated prior evidence. Re-run the smallest sufficient affected checks before scoring.

If code execution is unavailable, follow the same contract manually using the checkpoint template in `references/checkpoint.md` and report the lower automation guarantee.

Read `references/runtime.md` for supervisor, heartbeat, local-CI fallback, and recovery rules.

### Optional durable worker coordination

When the host supports subagents, persistent sessions, or long-running orchestration, initialize the provider-neutral coordination layer after the main checkpoint:

```bash
python <skill-path>/scripts/coordination.py init --root . --max-active-workers 2
```

Use it to preserve stable worker IDs, reuse sleeping workers, queue follow-up work with lease/receipt semantics, create deterministic worker finish boundaries, and write compact context handoffs. Coordination state lives in `.gigiloop/coordination.json` and is bound to the main checkpoint `run_id`.

Do not treat a message receipt as verification evidence. Code/test evidence remains in the main checkpoint. Read `references/coordination.md` before integrating host workers or persistent sessions.

## Choose an operating profile

Select the profile from user instructions and risk. Record it. Never silently downgrade it.

| Profile | Use for | Required behavior |
|---|---|---|
| **strict** | auth, payments, security, migrations, production incidents, public APIs, high-risk code | baseline + regression test + relevant integration checks + independent/fresh review when available + full final gate |
| **balanced** | ordinary features, refactors, bug fixes | targeted iteration checks + milestone regression checks + adversarial review + full relevant final gate |
| **fast** | explicit budget or low-risk exploration | baseline + meaningful reproducible check + reconciliation + strongest final checks allowed by budget |

Default to **balanced**. Escalate when consequences are material. Use **fast** only when an explicit budget or low-risk request justifies it.

## Phase 0 — Intake, safety, and baseline

Before editing:

1. Read repository instructions (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, contribution docs, local skills, package scripts).
2. Restate the goal in one sentence.
3. Derive explicit acceptance criteria and requested quality threshold.
4. Record scope, boundaries, compatibility requirements, and what must not change.
5. Select profile and budget. Default to 25 iterations when no cap is supplied.
6. Inspect branch, HEAD, dirty state, protected local changes, relevant diff, test/lint/typecheck/build/format/run commands.
7. Run the cheapest meaningful baseline checks before changing code.
8. Record pre-existing failures separately from loop regressions.
9. Define the **verification contract**: tests, thresholds, snapshots, linters, builds, security checks, or manual acceptance that must not be weakened merely to pass.
10. Create or refresh `.gigiloop/checkpoint.json` with the runtime when available.

If a baseline check is too expensive or unavailable, record the limitation and run the closest representative check. Never claim it ran when it did not.

## Phase 1 — Define the rubric

Define 3–6 objective criteria. For each record:

- name;
- weight, total 100%;
- evidence required;
- passing definition;
- highest plausible evidence tier;
- critical failure that blocks completion.

Use `references/scoring.md` for score anchors and evidence ceilings.

Do not redefine the rubric merely to make the current implementation pass. Change it only when a new user requirement or verified repository fact materially changes the goal, and record why.

## Phase 2 — Select orchestration

When the host provides subagents or fresh contexts, use the roles in `references/orchestration.md`:

1. **Builder** — implementation only.
2. **Verifier** — reproducible evidence only.
3. **Red Team** — hostile review and failure hypotheses.
4. **Judge** — reconciliation and next-action decision.
5. **Improver** — optional measurable self-improvement after correctness is stable.

Prefer real context separation. When unavailable, emulate roles sequentially and state that review was not independent. Same-context role-play cannot by itself justify T5 evidence.

## Phase 3 — Iteration loop

Repeat **A → B → C → D → E → F** until an exit condition is reached.

### A — Work

Fix the single highest-impact confirmed defect or unmet criterion.

- Prefer one coherent change over scattered edits.
- Preserve behavior outside scope unless the rubric requires broader change.
- Preserve pre-existing user changes and unrelated work.
- Avoid irreversible operations when a reversible alternative exists.
- Update heartbeat/phase when a supervisor is being used.

### B — Verify

Choose verification proportional to the change and profile:

- targeted unit/regression tests for affected code each iteration;
- affected integration tests when boundaries or contracts change;
- lint, typecheck, build, format, security checks when relevant;
- manual user-facing verification when code tests cannot cover behavior;
- full relevant suite at milestones and always at the final gate.

When no tests exist, add the smallest meaningful reproducible check rather than relying on inspection alone.

For every evidence item record method, scope, result/exit status, iteration/code state, classification, evidence tier, and limitations. Invalidate evidence after a relevant edit. Follow `references/verification.md`.

### C — Score

Score every criterion 0–10 using only current evidence. Record evidence references, tier, justification, uncertainty, and completion-blocking defects.

Never score above the ceiling justified by `references/scoring.md`.

### D — Red-team

Prefer a fresh-context independent reviewer/subagent. Otherwise perform a separate hostile pass.

Search for material defects including:

- wrong logic or missing edge cases;
- weak/deleted/skipped/over-mocked tests;
- snapshot/golden changes that merely bless broken behavior;
- lowered coverage/lint/type/security/performance thresholds;
- swallowed errors, concurrency faults, security exposure, secret leakage;
- accidental API/data-contract changes;
- hardcoded assumptions, dead code, scope creep, protected-work damage.

Report at most three material evidence-backed findings with location, severity, confidence, evidence, reproducer/falsification path, and remediation.

If none are supported, state so and test one plausible adversarial hypothesis before proceeding.

### E — Reconcile

Reconcile scores with review evidence.

- Lower scores invalidated by confirmed findings.
- Record evidence that falsifies rejected findings.
- Keep unverified hypotheses labeled as uncertainty.
- Re-check verifier integrity and protected work.

Post-reconciliation scores are authoritative.

### F — Decide and checkpoint

- If every criterion passes after reconciliation, proceed to the final gate.
- Otherwise choose the highest-impact confirmed issue as the next action.
- Apply plateau controls when progress is flat.
- Atomically checkpoint repository state, evidence freshness, scores, findings, integrity state, iteration, phase, and next action.

With the runtime:

```bash
python <skill-path>/scripts/gigiloop.py checkpoint --root . --increment --phase work --next-action "<next action>"
```

## Phase 4 — Plateau controls

Track material progress, not cosmetic score movement.

After 3 consecutive flat iterations, deliberately change strategy:

- challenge a root assumption;
- isolate a smaller discriminating reproducer;
- switch algorithm/architecture/debugging technique;
- inspect upstream/downstream contracts;
- use a different reviewer/tool;
- split scope into independently verifiable pieces;
- ask the user only when a genuine external dependency or missing requirement blocks safe progress.

Default hard cap: 25 iterations. At the cap, stop with an evidence-backed report. Never fabricate a pass.

## Phase 5 — Self-improvement pass

Run only when the primary correctness criteria already pass or the user explicitly requests automatic improvement.

Ask: **what material change could measurably improve the project without changing the agreed goal?**

For each candidate require a hypothesis, expected measurable benefit, risk/cost, and verification method. Choose at most one highest-value improvement per iteration. Re-run the full adversarial loop afterward.

Do not optimize indefinitely. Stop when no candidate has credible positive expected value, the user budget is reached, or an improvement threatens an already-passing critical criterion.

Follow `references/orchestration.md`.

## Phase 6 — Final gate

Do not declare success the first time scores cross the threshold.

1. Reconcile checkpoint with current branch, HEAD, worktree, diff, and fingerprint.
2. Confirm protected pre-existing changes remain present and distinguishable.
3. Run the full relevant test suite from the cleanest practical state.
4. Run relevant lint, typecheck, build, format, security, and compatibility checks.
5. Compare tests/quality configuration against baseline; reject unexplained weakening.
6. Review the complete diff produced by the loop.
7. Check for debug output, temporary files, unkept TODOs, secrets, ignored errors, generated debris, destructive migrations, and out-of-scope edits.
8. Perform a final adversarial review and reconcile new findings.
9. Update the checkpoint and produce the report in `references/reporting.md`.

Only then declare success.

## Hosted CI outage / quota policy

GitHub Actions or another hosted CI system must not be a single point of failure for ordinary loop progress.

If hosted CI is unavailable, exhausted, delayed, or infrastructure-failing:

1. record the remote limitation;
2. run equivalent deterministic local validation and project checks;
3. obtain independent local/subagent review when available;
4. continue iterating with evidence ceilings that reflect what actually ran;
5. retry remote CI only when useful or when merge/release policy explicitly requires it.

If branch protection or release policy requires remote CI, local checks support development but do not counterfeit the external gate. Report `BLOCKED` only when that required gate actually prevents the requested merge/release outcome.

## Recovery from idle, crash, or context reset

Treat `.gigiloop/checkpoint.json` as authoritative machine state.

On resume:

1. load and validate the checkpoint;
2. recompute repository fingerprint;
3. invalidate evidence if relevant state changed;
4. re-run the smallest sufficient baseline checks;
5. continue from `next_action` only after reconciliation.

When a restart-safe agent command is available, `scripts/gigiloop.py supervise` may restart it after unexpected exit or stale heartbeat. See `references/runtime.md`.

## Exit conditions

### SUCCESS

Require every criterion to pass after reconciliation, no material unresolved defect, current final verification, intact verifier contract, no new regression relative to baseline, preserved protected work, and current evidence in the final report.

### BLOCKED / BUDGET EXHAUSTED / STOPPED

Use only for a genuine external blocker, unavailable required gate/dependency, exhausted iteration/time/token/cost/restart budget, unsafe repository state, or explicit user stop.

Report verified state, unverified state, blocker/budget, and exact next action.

## User-visible updates

Keep iteration updates short and operational, for example:

`Iter 4 · balanced · Correctness 9, Regression 8, Robustness 7 · fixed timeout propagation · next: retry idempotency test.`

Do not expose hidden chain-of-thought. Report actions, evidence, scores, blockers, and next steps.

## References

- `references/scoring.md` — evidence tiers and score ceilings.
- `references/checkpoint.md` — canonical checkpoint schema and freshness rules.
- `references/verification.md` — progressive local verification strategy.
- `references/integrity.md` — verifier integrity, protected work, destructive-operation safeguards.
- `references/reporting.md` — final user-facing completion contract.
- `references/hosts.md` — host-neutral portability and fallback rules.
- `references/runtime.md` — deterministic runtime, heartbeat, supervision, CI fallback, recovery.
- `references/orchestration.md` — Builder/Verifier/Red Team/Judge/Improver role protocol.
- `references/coordination.md` — durable worker identity, leased inbox, finish boundary, sleeping reuse, and compact handoff protocol.

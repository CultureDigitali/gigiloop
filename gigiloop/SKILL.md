---
name: gigiloop
description: Verification-first autonomous coding loop. Use when the user asks an agent to keep iterating until code works, says "don't stop until it's done", requests a Ralph-style loop, wants repeated build-test-review cycles, asks to harden a feature to production quality, or wants evidence-backed autonomous improvement. Define measurable acceptance criteria, establish a baseline, implement and test changes, adversarially review them, reconcile scores after critique, checkpoint progress, detect plateaus, and stop only when every criterion is verified or a real blocker or budget limit is reached.
---

# GigiLoop — Verification-First Autonomous Coding

Run a bounded, resumable **build → test → score → red-team → reconcile → decide** loop.
Treat confidence as insufficient: completion requires evidence.

## Prime directives

1. Declare success only when every acceptance criterion is at least 9/10, all
   required verification is green, adversarial findings are reconciled, and the
   final gate passes.
2. Never award a score without concrete evidence such as a test result, command
   output, reproducible behavior, or a precise code reference.
3. Never invent defects to satisfy a critique quota. Report only evidence-backed
   findings or explicit hypotheses that still need verification.
4. Distinguish pre-existing failures from regressions introduced during the loop.
5. Keep the repository usable and the loop resumable after every iteration.
6. Prefer a fresh-context or independent reviewer when the host can provide one;
   otherwise use a separate adversarial stance in the current agent.
7. Ask the user only when a genuine blocker cannot be resolved safely from the
   repository, available tools, or a reasonable documented assumption.
8. Respect the user's explicit budget, scope, safety constraints, repository
   instructions, and stop request.

## Operating model

GigiLoop differs from a naive self-rating loop through these controls:

- **Evidence-gated completion** — no proof, no pass.
- **Baseline awareness** — record branch, HEAD, dirty state, verification commands,
  and failures that existed before changes.
- **Independent review when available** — use a separate reviewer/subagent with
  fresh context and preferably read-only access to the diff.
- **Evidence-gated critique** — findings need evidence; hypotheses are labeled as
  hypotheses and must be tested before they can lower a score.
- **Reconciliation after critique** — adversarial findings can lower scores before
  the loop decides whether it is done.
- **All-criteria gate** — do not average a weak criterion away.
- **Progressive verification** — run fast affected tests during iterations and the
  full relevant suite at milestones and the final gate.
- **Plateau re-strategy** — repeated lack of material progress requires a new
  approach, not another identical attempt.
- **Honest exit** — blockers and budget limits produce evidence-backed reports,
  never fabricated success.
- **Stale-checkpoint detection** — invalidate stale evidence if repository state
  changed outside the loop.

## Phase 0 — Intake, repository baseline, and budget

Before editing code:

1. Read repository instructions such as `AGENTS.md`, `CLAUDE.md`, contribution
   docs, local skill instructions, and relevant package scripts.
2. Restate the goal in one sentence and derive explicit acceptance criteria.
3. Record scope and explicit boundaries: files, modules, APIs, compatibility,
   performance, security, and anything that must not change.
4. Capture the optional budget: wall-clock target, maximum iterations, or an
   instruction such as "ship when solid; do not chase 10". Default to 25
   iterations when no limit is given.
5. Capture repository state where available:
   - branch;
   - HEAD SHA;
   - dirty/clean state;
   - diff fingerprint or equivalent;
   - test, lint, typecheck, build, and format commands.
6. Run the cheapest meaningful baseline verification before changing code.
   Record pre-existing failures separately from loop regressions.
7. Create or refresh the checkpoint described in `references/checkpoint.md`.

If a baseline command is prohibitively expensive, record why and run the closest
representative check instead. Never imply a baseline was run when it was not.

## Phase 1 — Define the rubric

Define 3–6 objective criteria. Each criterion must include:

- name;
- weight, with total weight = 100%;
- evidence required;
- definition of a 9/10 pass;
- highest plausible verification tier.

Use `references/scoring.md` for score anchors and evidence ceilings.

Example:

| Criterion | Weight | Evidence | 9/10 means |
|---|---:|---|---|
| Correctness | 40% | affected tests + integration behavior | core and edge paths pass |
| Regression safety | 25% | regression test + relevant suite | defect cannot recur unnoticed |
| Robustness | 20% | error-path tests + review | failures are handled explicitly |
| Project fit | 15% | lint/typecheck + diff review | follows repository conventions |

Do not redefine the rubric merely to make the current implementation pass. Change
it only when new user requirements or repository facts materially change the goal,
and record the reason.

## Phase 2 — Iteration loop

Repeat **A → B → C → D → E → F** until an exit condition is reached.

### A — Work

Fix the single highest-impact verified defect or unmet criterion. Prefer one
coherent improvement over many loosely related edits. Preserve existing behavior
outside scope unless the rubric explicitly requires broader change.

### B — Verify

Choose verification proportional to the change:

- run targeted unit/regression tests for affected code each iteration;
- run affected integration tests when boundaries change;
- run lint/typecheck/build when relevant to the edited surface;
- run manual or UI verification when behavior cannot be adequately tested in code;
- run the full relevant suite at meaningful milestones and always at the final gate.

When no tests exist, add the smallest meaningful reproducible check rather than
relying on inspection alone. Follow `references/verification.md`.

A failing verification does not automatically mean the current edit caused it.
Compare against the baseline and classify it as pre-existing, new regression, or
unresolved/unknown.

### C — Score

Score every rubric criterion from 0–10 using only current evidence.
For each criterion record:

- score;
- evidence references;
- verification tier;
- one-line justification;
- known uncertainty.

Never score above the ceiling justified by the evidence tier in
`references/scoring.md`.

### D — Red-team

Review the current diff and assumptions as an adversary.

Prefer this order:

1. a fresh-context independent reviewer/subagent when available;
2. a separate adversarial pass in the current agent otherwise.

Look for material problems such as logic errors, missing edge cases, weak tests,
security exposure, concurrency issues, swallowed errors, incompatible API changes,
hardcoded assumptions, dead code, or scope creep.

For each finding include:

- location or affected behavior;
- severity;
- confidence;
- evidence already observed;
- a reproducer, test, or concrete verification path;
- proposed remediation.

Find **up to three material evidence-backed flaws**. Do not fabricate additional
findings to reach a quota.

If no material flaw is supported by evidence:

- state that no additional evidence-backed material flaw was found;
- formulate one plausible adversarial hypothesis or edge case;
- test or inspect that hypothesis before proceeding.

### E — Reconcile

Reconcile the score sheet with the adversarial review before deciding completion.

- Confirmed material findings must lower the affected criterion if the previous
  score no longer reflects reality.
- Rejected findings must record the evidence that falsified them.
- Unverified hypotheses cannot be presented as defects; either test them now or
  carry them forward as explicit uncertainty.

The post-reconciliation score sheet is authoritative.

### F — Decide

- If every criterion is at least 9/10 after reconciliation, proceed to the final
  gate.
- Otherwise, make the highest-impact confirmed issue the next action.
- If progress is flat, apply the plateau controls below.
- Rewrite the checkpoint at the end of the iteration.

## Phase 3 — Plateau controls and budget

Track material progress, not cosmetic score movement.

After 3 consecutive iterations without material improvement, choose a genuinely
new strategy:

- challenge an implementation assumption;
- switch algorithm, architecture, or debugging method;
- isolate the defect with a smaller reproducer;
- use a different available tool or reviewer;
- split scope into independently verifiable pieces;
- inspect upstream/downstream contracts;
- ask the user once if an external dependency or missing requirement is genuinely
  blocking progress.

Default hard cap: 25 iterations per goal. User-provided budgets override it.
At the cap, stop with an evidence-backed report containing failed criteria,
verified blockers, remaining uncertainty, and the exact recommended next action.

## Phase 4 — Final gate

Do not declare success the first time the score sheet reaches 9+.

1. Confirm the checkpoint matches the current branch/HEAD/worktree. If repository
   state changed externally, invalidate stale evidence and re-baseline affected
   checks.
2. Run the full relevant test suite from the cleanest practical state.
3. Run relevant lint, typecheck, build, and format checks.
4. Review the complete diff produced by the loop.
5. Check for leftover debug output, temporary files, unkept TODOs, hardcoded
   secrets, accidental generated files, ignored errors, and out-of-scope edits.
6. Perform one final adversarial pass. Reconcile any new finding with the rubric.
7. Update the checkpoint with the final evidence and result.

Only then declare success.

## Phase 5 — Exit conditions

### SUCCESS

Require all of the following:

- every criterion ≥ 9/10 after reconciliation;
- no known material unresolved defect;
- required final verification passes;
- no new regression relative to baseline;
- final adversarial pass reconciled;
- checkpoint updated with final evidence.

### FAILURE / BLOCKED / BUDGET EXHAUSTED

Use only for:

- iteration or user budget reached;
- genuine external blocker;
- required tool or dependency unavailable;
- explicit user stop;
- repository state that cannot safely be modified.

Report what is verified, what remains unverified, and the next concrete action.

## Persistence and stale-state handling

Use `.gigiloop/checkpoint.md` in the project when writable. If the host requires a
platform-specific state path, mirror or adapt it, but treat the checkpoint as the
authoritative loop state.

At the start of every resumed turn:

1. read the checkpoint;
2. compare recorded branch/HEAD/diff state with the current repository;
3. invalidate evidence tied to changed code;
4. resume from `next_action` only after stale-state reconciliation.

Use the host's todo/task mechanism when available, but do not depend on a specific
tool such as `todowrite` for correctness.

See `references/checkpoint.md` for the canonical fields.

## User-visible updates

Keep updates short and useful at iteration boundaries:

`Iter 4 — Correctness 9, Regression safety 8, Robustness 7. Fixed timeout propagation; next: retry idempotency test.`

Do not expose hidden chain-of-thought. Report actions, evidence, scores, blockers,
and next steps.

## Worked example pattern

Treat examples as illustrative until backed by a reproducible benchmark. Do not
present hypothetical outcomes as measured results.

Example goal: "Fix the login bug and harden the login flow to 9/10."

- Baseline: reproduce the bug and record existing test failures.
- Rubric: root cause 30%, regression safety 25%, auth edge cases 30%, secrets
  handling 15%.
- Iteration: fix root cause, add regression test, run affected suite.
- Red-team: inspect expired/locked/rate-limited states and secret handling.
- Reconcile: lower any criterion invalidated by confirmed findings.
- Final gate: full relevant suite + diff review + final adversarial pass.

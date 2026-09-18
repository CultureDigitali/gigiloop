# Adversarial multi-agent orchestration

Use independent roles when the host supports subagents, fresh contexts, or parallel workers. The roles are logical contracts: a host may map them to real subagents or execute them sequentially with context separation.

## Roles

### 1. Builder

Own implementation only.

- consume the current goal, acceptance criteria, confirmed findings, constraints, and next action;
- make the smallest coherent change that attacks the highest-impact gap;
- do not approve its own work;
- publish changed paths, assumptions, and the exact checks it expects to prove the change.

### 2. Verifier

Own reproducible evidence.

- run targeted checks first and broader checks when required;
- classify failures as pre-existing, new regression, or unresolved;
- reject unverifiable claims from the Builder;
- never alter production code merely to make verification pass.

The Verifier may add a minimal regression test when that is part of the agreed verification contract, but test changes remain subject to integrity review.

### 3. Red Team

Own hostile review.

- inspect the diff, assumptions, tests, contracts, error paths, boundaries, security implications, and protected work;
- report at most three material evidence-backed findings;
- include a reproducer, failing test, concrete code path, or falsification method;
- never invent a defect to satisfy a quota.

Prefer a fresh context with read-only access to implementation evidence.

### 4. Judge

Own reconciliation and the stop/continue decision.

- compare the Verifier evidence with Red Team findings;
- lower scores invalidated by confirmed findings;
- reject unsupported findings and record why;
- enforce evidence ceilings and the all-criteria gate;
- choose the single next action with the highest expected effect on the weakest critical criterion.

The Judge must not implement fixes in the same decision pass.

### 5. Improver

Run only after correctness is stable or when the user explicitly requests self-improvement.

Ask: **what material improvement would make the result measurably better without changing the agreed goal?**

Candidate improvements may include:

- simpler architecture;
- lower latency or resource use;
- stronger failure handling;
- clearer developer ergonomics;
- safer defaults;
- better observability;
- broader relevant tests;
- removal of accidental complexity.

Every candidate requires a hypothesis, measurable benefit, cost/risk estimate, and verification plan. Do not implement speculative improvements merely because they sound desirable.

## Turn protocol

A complete adversarial iteration is:

```text
Builder -> Verifier -> Score -> Red Team -> Judge -> Checkpoint
                                      |
                                      +-> next Builder action
```

When self-improvement is enabled and the primary goal already meets its threshold:

```text
Judge -> Improver hypothesis -> Builder -> Verifier -> Red Team -> Judge
```

The original acceptance criteria remain protected. An improvement that regresses an already-passing critical criterion must be reverted or repaired before completion.

## Independence rules

When real subagents are available:

- give Builder only the implementation task and necessary repository context;
- give Verifier the acceptance criteria and current code state, not the Builder's confidence narrative;
- give Red Team the goal, constraints, diff, and verification evidence, but ask it to derive its own failure hypotheses;
- give Judge the structured outputs of all roles and require evidence citations for every decision.

Do not share hidden chain-of-thought between roles. Share only task context, observable evidence, findings, and decisions.

When separate subagents are unavailable, emulate role separation sequentially and explicitly record that independent review was unavailable. Do not award T5 solely from same-context role-play.

## Parallelism

Parallelize only work that has no shared-write dependency.

Safe examples:

- Verifier runs unit tests while a read-only Red Team inspects the already-frozen diff;
- two read-only reviewers inspect security and API compatibility independently.

Unsafe examples:

- multiple Builders editing the same files concurrently;
- a test-modifying Verifier racing the Builder;
- a Judge deciding before all evidence for the iteration is frozen.

If parallel workers produce conflicting conclusions, the Judge must reconcile them against reproducible evidence rather than majority vote.

## Plateau escalation

After three flat iterations:

1. stop the current implementation strategy;
2. ask Red Team to challenge the root assumption, not merely inspect syntax;
3. ask Verifier for the smallest reproducer that discriminates between competing hypotheses;
4. ask Improver for a materially different approach;
5. have Judge choose one approach and record why it differs from the failed strategy.

Do not spend iterations on cosmetic score changes.

## Completion ownership

No single role can declare success.

`SUCCESS` requires:

- Verifier: current evidence passes;
- Red Team: no unresolved material finding;
- Judge: every critical criterion clears its threshold after reconciliation;
- integrity gate: no unexplained verifier weakening or protected-work conflict;
- final gate: repository state and evidence are current.


## Durable worker lifecycle

When a host can preserve subagent or session identity, prefer a stable worker ID per logical role instance. A worker that has completed its current turn should normally become `sleeping`, not be destroyed. Revive that worker for related follow-up work before creating another equivalent worker, subject to the host's context-quality constraints.

Use the coordination runtime's leased inbox when work must survive crashes or context replacement. A worker finish boundary must either acknowledge its current receipt or explicitly requeue ownership; it must not report completion while an owned message is ambiguous. If another queued message exists, `finish` may atomically claim it and keep the worker active.

Context compaction is a handoff, not a new project. Preserve the GigiLoop run ID and write a structured handoff containing the observable summary, next action, worker snapshot, and pending message IDs. Hidden chain-of-thought is never part of the handoff.

See `references/coordination.md` for the executable protocol.

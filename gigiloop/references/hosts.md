# Host portability

Use the canonical GigiLoop protocol independently of the coding-agent host.

## Core rule

Treat host-specific features as optional accelerators. Never make correctness depend on a feature that only one host provides.

Preserve these invariants everywhere:

1. establish repository baseline before edits;
2. define measurable acceptance criteria;
3. verify each iteration with proportionate checks;
4. score from evidence only;
5. adversarially review the current diff;
6. reconcile scores after critique;
7. detect plateaus and change strategy;
8. checkpoint enough state to resume safely;
9. run a final verification gate before success.

## Capability adaptation

### Independent reviewer / subagent available

Prefer a fresh-context reviewer with read-only access to the relevant diff when the host supports it. Feed the reviewer the acceptance criteria, baseline facts, current diff, and verification evidence. Do not feed it the builder's self-justification unless necessary.

### No independent reviewer

Run a distinct adversarial pass in the current agent. Re-open the diff and evidence as if reviewing another contributor. Findings still require evidence or a concrete verification path.

### Todo / task state available

Mirror the next action and important blockers into the host task mechanism, but keep `.gigiloop/checkpoint.md` authoritative when the repository is writable.

### No persistent task mechanism

Use the checkpoint only. Keep it compact and sufficient to recover goal, rubric, baseline, current evidence, unresolved findings, iteration count, and next action.

### Hooks / workflows available

Use hooks or host workflows to automate deterministic verification where useful, but do not hide required evidence behind an opaque automation. Record the command or result that justifies each pass.

### Host cannot write the repository

Run analysis and verification where possible, then report the exact edit or command required. Do not claim completion if the requested modification could not be applied.

## Installation ecosystem

GigiLoop follows the Agent Skills `SKILL.md` convention. The `skills` CLI can install the canonical skill to supported hosts. Repository-level adapters outside the skill package are convenience wrappers and must not diverge from this reference.
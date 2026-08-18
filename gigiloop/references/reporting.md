# Final reporting contract

Produce a compact, evidence-backed report whenever GigiLoop exits.

## Required fields

```markdown
## GigiLoop result

- **Status:** SUCCESS | BLOCKED | BUDGET EXHAUSTED | STOPPED
- **Profile:** strict | balanced | fast
- **Goal:** one-sentence goal
- **Iterations:** N
- **Repository state:** branch / HEAD / relevant diff state

### Changes made
- concise list of material changes

### Verification
| Check | Scope | Result | Evidence state |
|---|---|---|---|
| command or method | affected scope | pass/fail/not run | baseline/targeted/milestone/final |

### Scorecard
| Criterion | Score | Tier | Evidence | Remaining uncertainty |
|---|---:|---|---|---|

### Integrity
- **Pre-existing failures:** none / list
- **Protected local changes:** preserved / conflict described
- **Verifier changes:** none / justified changes described
- **Destructive operations:** none / authorized operation described

### Remaining risks or blockers
- none / concrete item

### Next action
- none for SUCCESS / exact next step otherwise
```

## Status rules

### SUCCESS

Use only when every acceptance criterion passes after reconciliation and the final gate is complete.

### BLOCKED

Use when an external dependency, permission, missing requirement, unavailable tool, or unsafe repository state prevents further verified progress.

### BUDGET EXHAUSTED

Use when the explicit time, cost, token, or iteration budget is reached before the completion contract passes.

### STOPPED

Use when the user explicitly stops the loop.

## Evidence language

Use these distinctions consistently:

- **Verified:** directly supported by a current test, command result, reproducible behavior, or inspected code state.
- **Inferred:** a reasoned conclusion supported by evidence but not directly tested.
- **Unverified:** not checked or no longer current after a relevant change.
- **Blocked:** could not be verified because of a named external limitation.

Do not use “fixed,” “safe,” “complete,” or “production-ready” for an unverified claim.

## Brevity

Keep the user-facing report decision-ready. Link or cite detailed logs/checkpoints rather than dumping raw output. Do not expose hidden chain-of-thought, discarded drafts, or internal role dialogue.

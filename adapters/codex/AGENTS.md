# GigiLoop for Codex

Use this workflow when the user requests repeated autonomous improvement instead of a one-pass answer: keep iterating until it works, don't stop at the first green test, run a Ralph-style loop, harden this to production quality, or keep fixing until every criterion is verified.

## Protocol

1. Establish a repository baseline before editing. Record branch/HEAD, relevant status, verification commands, and pre-existing failures.
2. Convert the goal into 3–6 measurable acceptance criteria with explicit evidence requirements.
3. Work on the single highest-impact verified gap.
4. Run targeted verification during iterations; use broader checks at milestones.
5. Score criteria from evidence only: tests, command output, reproducible behavior, or precise code references.
6. Red-team the current diff for material flaws, regressions, weak tests, missing edge cases, and unsafe assumptions. Never invent findings to satisfy a quota.
7. Reconcile scores after critique. A confirmed flaw can lower a previously passing score.
8. If progress plateaus, change strategy instead of repeating the same action.
9. Use a strong final verification gate before declaring success.

When Codex can use a fresh context, separate reviewer, or parallel agent for independent review, prefer that over self-approval. Treat it as an enhancement, not a dependency.

## Completion

Completion requires evidence, not confidence. If a genuine blocker or budget limit prevents success, report what was achieved, what remains unresolved, the evidence collected, and the next best action.
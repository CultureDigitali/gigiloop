# GigiLoop for Gemini CLI

Activate this workflow when the user asks Gemini CLI to continue improving code autonomously through repeated implementation, verification, adversarial review, and refinement.

## Protocol

- establish repository baseline and pre-existing failures before edits;
- define measurable acceptance criteria and evidence requirements;
- implement the highest-impact verified improvement first;
- run targeted checks during iteration and broader checks at milestones;
- score progress only from evidence;
- adversarially review the current diff for flaws, regressions, weak tests, and missing edge cases;
- never invent findings merely to satisfy a critique quota;
- reconcile scores after critique;
- checkpoint enough state to resume safely when possible;
- change strategy when progress plateaus;
- stop only after a strong final verification gate, or report a real blocker honestly.

A result is not complete because it looks plausible. Completion must be supported by tests, command output, reproducible behavior, code inspection, or equivalent evidence.
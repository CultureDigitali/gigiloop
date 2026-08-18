# Show HN — submission draft

**Title:** Show HN: GigiLoop – an autonomous coding skill that won't ship until its own work is backed by evidence

**Body:**

Most "keep trying until it works" agents stop at the first green test and call it done. GigiLoop (an OpenCode skill) is built for the moment that isn't enough.

It establishes a baseline, defines measurable acceptance criteria, iterates on the highest-impact issue, adversarially reviews its own diff, reconciles scores *after* critique, and only ships after a final verification gate.

The parts that make it different from a naive loop:

- Evidence-gated scoring — every pass condition must point to a test, command output, or a precise code reference. No proof, no score.
- Score reconciliation — a confirmed flaw can *lower* a score before completion is decided. It is not allowed to congratulate itself.
- Evidence-gated red-team — up to three material findings per iteration, never a fabricated one to hit a quota.
- All-criteria gate — every acceptance criterion must clear the threshold; averaging strong over weak is forbidden.
- Plateau → re-strategy — three flat iterations force a genuinely different approach instead of churn.
- Honest exit — a blocker or budget ends the loop with a report instead of a fabricated 9/10.

It also ships a reproducible benchmark protocol: compare one-pass, naive-loop, and GigiLoop from the same commit, with hidden evaluators and raw evidence preserved. A score is publishable only when the evidence is independently auditable.

Repo: https://github.com/CultureDigitali/gigiloop
Install: `npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode`

MIT licensed. Would love feedback on the reconciliation rule and the benchmark protocol — both are the parts I'm least sure about.

# GigiLoop v0.2.0 — Verification-First Autonomous Coding

GigiLoop is an autonomous coding skill for OpenCode and compatible agent-skill hosts.
It keeps working, testing, and adversarially reviewing its own diff until your acceptance
criteria are backed by evidence — not agent confidence.

> Think Ralph Loop, but with a hostile reviewer and an evidence gate.

## What's in v0.2.0

- **Evidence-gated scoring** — every pass condition must point to a test, command output, reproducible behavior, or a precise code reference. No proof, no score.
- **Baseline awareness** — records pre-existing failures before editing, so regressions are distinguishable from the starting state.
- **Score reconciliation** — confirmed adversarial findings can *lower* scores before completion is decided. No self-congratulation.
- **Evidence-gated red-team** — report up to three material findings per iteration; never fabricate one to hit a number.
- **Independent reviewer when available** — fresh-context/subagent review is preferred over same-context self-approval.
- **Progressive verification** — targeted checks during iterations, full relevant verification at milestones and the final gate.
- **Plateau → re-strategy** — three flat iterations force a genuinely different approach instead of churn.
- **Checkpoint validation** — branch/HEAD/diff state is compared before reusing old evidence, so it survives compaction and repo changes.
- **All-criteria gate** — every acceptance criterion must clear the threshold; averaging strong over weak is forbidden.
- **Honest exit** — a blocker or budget ends the loop with a report instead of a fabricated 9/10.

## The loop

```
GOAL + BASELINE + RUBRIC
        │
        ▼
    A. WORK → B. VERIFY → C. SCORE → D. RED-TEAM → E. RECONCILE → F. DECIDE
        │                                                          │
   < 9/10                                                    all ≥ 9/10
        │                                                          │
        └──────────────────────► FINAL GATE ──► DONE
```

## Install

```bash
npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode
# or globally
npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode -g
# or manually
git clone https://github.com/CultureDigitali/gigiloop ~/.config/opencode/skills/gigiloop
```

## Prove it, don't market fiction

`benchmarks/` ships a reproducible protocol: compare one-pass, naive-loop, and GigiLoop
from the same starting commit, with hidden evaluators and raw evidence preserved. A score
is publishable only when the evidence directory makes the result independently auditable.

MIT licensed. Feedback and benchmark cases welcome.

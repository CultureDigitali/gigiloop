# GigiLoop benchmark protocol

This directory is for reproducible evidence, not promotional anecdotes.

## Goal

Compare three workflows from the **same starting commit**:

1. one-pass agent;
2. naive keep-going / Ralph-style loop;
3. GigiLoop.

Keep the model, host, tool permissions, task statement, repository state, and budget as comparable as practical.

## Required benchmark record

Each case should include:

```text
case/
├── README.md
├── task.md
├── start.sha
├── evaluator.md
├── results/
│   ├── one-pass.md
│   ├── naive-loop.md
│   └── gigiloop.md
└── evidence/
    └── ... raw logs, test output, patches, or machine-readable results
```

## Evaluation rules

- Prefer hidden tests or an evaluator not shown to the coding agent.
- Record the exact model and host version when known.
- Record wall-clock time, iterations, and token/cost data when available.
- Preserve raw evidence for every published score.
- Distinguish pre-existing failures from introduced regressions.
- Do not publish a percentage or quality score that cannot be reconstructed from the case files.
- Do not cherry-pick only successful GigiLoop cases; failed or neutral cases are useful evidence too.

## Suggested case families

- payment retry/idempotency;
- authentication edge cases and secret leakage;
- accessibility/keyboard regressions;
- concurrency and race conditions;
- API compatibility regressions;
- error propagation and retry behavior.

## Result table template

| Workflow | Hidden checks passed | Regressions | Iterations | Time | Notes |
|---|---:|---:|---:|---:|---|
| One pass | — | — | 1 | — | |
| Naive loop | — | — | — | — | |
| GigiLoop | — | — | — | — | |

A result is publishable only when the evidence directory makes the table independently auditable.

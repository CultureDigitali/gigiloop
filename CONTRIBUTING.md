# Contributing to GigiLoop

Contributions are welcome, especially when they make autonomous coding more verifiable rather than merely more persistent.

## High-value contributions

- reproducible benchmark cases;
- false-positive or false-negative adversarial findings;
- portability fixes for Agent Skills-compatible hosts;
- checkpoint/resume edge cases;
- verification strategies for specific ecosystems;
- examples where GigiLoop performs worse than a simpler workflow.

## Pull requests

Before opening a PR:

1. Keep `gigiloop/SKILL.md` focused on control flow and non-obvious rules.
2. Move detailed reusable guidance into `gigiloop/references/`.
3. Do not add marketing claims without reproducible evidence.
4. Preserve the evidence-gated critique rule: never require the agent to invent findings to satisfy a quota.
5. Preserve post-critique reconciliation before completion.
6. Update `CHANGELOG.md` for material behavior changes.
7. Ensure the skill frontmatter remains valid YAML with lowercase `name` and a precise trigger-oriented `description`.

## Benchmark submissions

A benchmark contribution should identify the starting commit, task, model/host where possible, evaluator, raw evidence, and results for comparable workflows. See `benchmarks/README.md`.

Negative results are acceptable. Benchmark selection should not be restricted to cases where GigiLoop wins.

## Reporting behavior bugs

Include:

- host and model;
- original request;
- relevant checkpoint excerpt;
- expected behavior;
- observed behavior;
- evidence showing why the result is wrong or unsafe.

Remove secrets, credentials, customer data, and proprietary source before posting public issues.

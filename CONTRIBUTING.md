# Contributing to GigiLoop

Contributions are welcome when they make autonomous coding more verifiable, safer, more portable, or easier to evaluate.

## High-value contributions

- reproducible benchmark cases, including cases where GigiLoop loses;
- false-positive or false-negative adversarial findings;
- verifier-weakening examples and defenses;
- protected-work or checkpoint/resume edge cases;
- portability fixes for Agent Skills-compatible hosts;
- verification strategies for specific languages and ecosystems;
- clearer evidence or reporting contracts.

## Pull requests

Before opening a PR:

1. Keep `gigiloop/SKILL.md` focused on control flow and non-obvious rules.
2. Move detailed reusable guidance into one-level-deep files under `gigiloop/references/`.
3. Keep host adapters small and behaviorally aligned with the canonical skill.
4. Do not add marketing or benchmark claims without reproducible evidence.
5. Preserve evidence-gated critique and post-critique score reconciliation.
6. Preserve verification-integrity rules: passing by weakening tests or thresholds is not valid.
7. Preserve protected-work and destructive-operation safeguards.
8. Update `CHANGELOG.md` for material behavior or compatibility changes.
9. Run the GitHub validation workflow and inspect the complete diff.
10. Ensure `SKILL.md` frontmatter remains valid with lowercase `name` and a trigger-oriented `description`.

## Visual and branding changes

The repository owner approved the canonical rendered assets listed in `assets/visual-manifest.json`.

A visual replacement must include:

- explicit approval of the rendered result;
- updated asset files;
- updated SHA-256 hashes and dimensions in the manifest;
- updated README/metadata references;
- passing asset-integrity CI;
- a changelog entry.

Do not silently replace a visual by reusing an existing filename. Do not merge third-party marks into the GigiLoop logo.

## Benchmark submissions

A benchmark contribution should identify:

- starting commit and task;
- compared workflows;
- model, host, profile, and budget;
- evaluator or hidden checks;
- raw evidence and final code state;
- verifier/test changes;
- results, including negative outcomes.

See `benchmarks/README.md`.

## Reporting behavior bugs

Include:

- host, model, and operating profile;
- original request and acceptance criteria;
- relevant checkpoint excerpt;
- expected and observed behavior;
- current evidence showing why the result is wrong or unsafe;
- whether tests, thresholds, user work, or destructive operations were involved.

Remove secrets, credentials, customer data, and proprietary source before posting publicly.

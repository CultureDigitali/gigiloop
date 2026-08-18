# Progressive verification strategy

Match verification cost to change risk while preserving a strong final gate.

## Verification levels

### Baseline

Run before edits when practical:

- repository status and current diff;
- the cheapest meaningful existing test or reproducer;
- relevant lint, typecheck, build, or startup check;
- known failing checks and unavailable dependencies.

Record failures that already exist.

### Targeted iteration checks

Run after each coherent change:

- the regression test for the reported defect;
- unit tests for edited modules;
- deterministic checks for affected contracts;
- manual reproduction when automation is unavailable.

Targeted checks make the loop fast. They do not replace broader final verification.

### Milestone checks

Run when a boundary, API, data model, security control, or substantial implementation strategy changes:

- affected integration tests;
- broader package/module suite;
- lint, typecheck, build, or security checks for the changed surface;
- compatibility checks promised by the rubric.

### Final gate

Run the strongest relevant verification permitted by the operating profile and budget:

- full relevant automated suite;
- lint, typecheck, build, format, and security checks;
- complete diff inspection;
- protected-work and verification-contract comparison;
- final adversarial review and score reconciliation.

## Evidence record

For each check record:

- exact command or method;
- relevant scope;
- result and exit status;
- iteration and code state;
- baseline, targeted, milestone, or final classification;
- evidence tier;
- limitation or skipped dependency.

Evidence tied to changed code becomes stale. Re-run it before using it to justify completion.

## Negative and adversarial tests

Do not verify only the happy path. Select the most relevant of:

- malformed or missing input;
- zero, empty, maximum, and boundary values;
- retries and idempotency;
- timeouts, partial failures, and unavailable dependencies;
- authorization and privilege boundaries;
- concurrency, ordering, and race conditions;
- backward compatibility and migration behavior;
- secret leakage, logs, and error messages;
- accessibility and keyboard behavior for UI work.

## No-test repositories

Create the smallest meaningful reproducible check:

- assertion script;
- smoke test;
- golden-output comparison with reviewed expected output;
- deterministic command sequence;
- manual protocol with observable expected behavior.

Inspection alone normally cannot justify a 9/10 correctness score.

## Verification integrity

Follow `integrity.md`. A green result obtained by deleting tests, lowering thresholds, disabling checks, blindly updating snapshots, or narrowing scope is not a valid pass.

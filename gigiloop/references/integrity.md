# Verification and repository integrity

Use this reference whenever a change touches tests, quality thresholds, snapshots, migrations, generated files, or pre-existing local work.

## Verification contract

At baseline, record the checks and quality signals that define a legitimate pass:

- test files and relevant suites;
- coverage thresholds;
- lint, typecheck, build, security, and formatting configuration;
- snapshots, fixtures, golden outputs, or approval files;
- manual acceptance steps;
- repository instructions and protected APIs.

Treat this as a contract. Passing by weakening the contract is not completion.

## Forbidden shortcuts

Do not do any of the following merely to obtain green output:

- delete, skip, quarantine, or comment out a failing test;
- reduce assertions or replace behavioral checks with implementation-detail mocks;
- lower coverage, lint, type, performance, security, or quality thresholds;
- disable a check, hook, workflow, compiler flag, or strict mode;
- update a snapshot, fixture, or golden output without verifying the new behavior is correct;
- change acceptance criteria after seeing the implementation;
- hide a failure by swallowing an error, broadening an exception, or suppressing output;
- narrow the tested scope while claiming broader verification.

If a test or threshold is genuinely wrong, document:

1. why it is wrong;
2. the evidence that falsifies it;
3. the replacement verification;
4. any independent or fresh-context review obtained;
5. the user-visible impact.

## Preserve pre-existing work

Before editing, identify uncommitted and unrelated changes. Treat them as protected input.

- Do not overwrite, discard, reformat, or silently absorb unrelated changes.
- Separate loop changes from pre-existing changes in the checkpoint and final report.
- Prefer a branch, worktree, patch, or narrowly scoped edit when isolation is available.
- Re-check protected changes before the final gate.

## Destructive operations

Do not run destructive or difficult-to-reverse operations without explicit authorization or an already-established repository workflow that clearly requires them.

Examples:

- `git reset --hard`, `git clean -fd`, force push, branch deletion;
- destructive database migrations or data resets;
- bulk file deletion or generated-file replacement outside scope;
- credential rotation, deployment, release, or production mutation;
- rewriting history or removing user-owned artifacts.

Before an authorized risky operation:

1. state the exact operation and affected scope;
2. preserve a recoverable checkpoint or backup when possible;
3. verify authorization and repository state;
4. execute the smallest sufficient operation;
5. verify the result and recovery path.

## Integrity classification

Classify every suspected integrity issue as one of:

- **confirmed regression** — introduced by the loop and reproduced;
- **pre-existing failure** — present at baseline;
- **verification weakening** — a pass was obtained by reducing scrutiny;
- **protected-work conflict** — the loop would overwrite or entangle user work;
- **unresolved hypothesis** — plausible but not yet verified.

A confirmed verification weakening or protected-work conflict blocks success until resolved or explicitly accepted by the user.

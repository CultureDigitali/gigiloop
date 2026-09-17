# GigiLoop local runtime and recovery protocol

Use `scripts/gigiloop.py` when the host can execute Python 3. The runtime uses only the Python standard library and turns the textual loop contract into machine-checkable state and recovery primitives.

## Design rule

**Local verification is authoritative; remote CI is optional evidence.**

GitHub Actions, hosted CI, and agent-host task state may accelerate verification, but GigiLoop must remain able to continue when they are unavailable, quota-exhausted, delayed, or temporarily failing for infrastructure reasons.

Do not interpret unavailable remote CI as either a pass or a product failure. Run the strongest equivalent local checks, record the limitation, and continue according to the selected profile.

## Quick start

From the target repository root, with the skill installed:

```bash
python <skill-path>/scripts/gigiloop.py doctor --root .
python <skill-path>/scripts/gigiloop.py init --root . --goal "fix the login regression" --profile balanced
python <skill-path>/scripts/gigiloop.py status --root .
```

When resuming after a context reset, process restart, or external repository edit:

```bash
python <skill-path>/scripts/gigiloop.py resume --root .
```

Exit code `2` from `resume` means repository drift was detected. Current evidence that depended on the previous state is marked stale and affected checks must be re-baselined before scoring or completion.

## Checkpoint authority

The canonical machine-readable checkpoint is:

```text
.gigiloop/checkpoint.json
```

`checkpoint.json` is runtime state and must not be committed. Human-readable reports may summarize it, but they do not replace it.

The runtime writes checkpoints atomically using a unique same-directory temporary file followed by `os.replace`. Read-modify-write mutations are serialized with `.gigiloop/checkpoint.lock`, so concurrent heartbeat/checkpoint/supervisor updates do not silently overwrite each other. Stale locks are recoverable after a bounded interval.

The checkpoint records:

- schema version, run ID, generation, status, profile, iteration, and budget;
- goal, scope, constraints, verification contract, baseline, rubric, and evidence;
- repository branch, HEAD, dirty state, untracked files, and content-sensitive fingerprint;
- findings, integrity exceptions, plateau state, next action, and timestamps;
- runtime phase, heartbeat, restart count, last process exit code, and re-baseline requirement.

Never store secrets, tokens, customer data, or large logs in the checkpoint.

## Repository fingerprint

The runtime fingerprint is intended to answer one question: **does the evidence still refer to the same relevant repository state?**

For Git repositories it includes:

- branch and HEAD;
- staged diff;
- unstaged diff;
- names and contents of untracked files;
- symlink targets as link metadata without dereferencing the external target.

Git output is decoded with surrogate escaping so unusual non-UTF-8 filenames cannot crash fingerprinting on POSIX filesystems.

`.git/**` and `.gigiloop/**` are excluded so checkpoint writes do not invalidate their own evidence.

For non-Git directories, the runtime falls back to a deterministic content hash of files outside `.git/**` and `.gigiloop/**`.

A changed fingerprint does not automatically mean a regression. It means evidence freshness must be reconciled before reuse.

## Heartbeat and phases

Long-running host integrations should update the heartbeat at meaningful boundaries:

```bash
python <skill-path>/scripts/gigiloop.py heartbeat --root . --phase work --message "implementing retry guard"
python <skill-path>/scripts/gigiloop.py heartbeat --root . --phase verify --message "running affected suite"
```

Allowed phases are:

- `intake`
- `work`
- `verify`
- `score`
- `review`
- `reconcile`
- `final`

A heartbeat proves liveness only. It is not verification evidence.

## Atomic checkpoint advancement

At iteration boundaries, persist the next resumable action:

```bash
python <skill-path>/scripts/gigiloop.py checkpoint \
  --root . \
  --increment \
  --phase verify \
  --next-action "run auth integration tests"
```

Use terminal status only when the reporting contract is satisfied:

```bash
python <skill-path>/scripts/gigiloop.py checkpoint --root . --status success --phase final --next-action "none"
```

The runtime enforces the maximum iteration budget by changing status to `budget_exhausted` after the configured limit is exceeded.

## Supervisor mode

The runtime can supervise a host command that is safe to restart:

```bash
python <skill-path>/scripts/gigiloop.py supervise \
  --root . \
  --idle-seconds 900 \
  --max-restarts 10 \
  -- your-agent-command --resume-from .gigiloop/checkpoint.json
```

Behavior:

1. reconcile repository state before each launch;
2. launch the command;
3. watch the checkpoint heartbeat;
4. terminate and restart after a stale heartbeat;
5. restart after an unexpected process exit;
6. stop on terminal checkpoint status or restart/wall-clock budget exhaustion.

Only supervise commands that are documented as restart-safe or idempotent. The supervisor does not make an unsafe deployment, migration, payment, or destructive command safe.

## GitHub Actions / hosted CI fallback

Use this hierarchy when GitHub Actions or another hosted CI system is unavailable:

1. **Local deterministic runtime checks** — run `self-test`, `validate-repo`, and project-specific verification locally.
2. **Local project checks** — tests, lint, typecheck, build, security and integration commands from the repository.
3. **Independent reviewer/subagent** — when the host provides one, review local evidence and the diff without depending on hosted CI.
4. **Hosted CI** — add its result when available; do not block ordinary progress solely because minutes/quota are exhausted unless the user or repository policy explicitly requires that remote check.

If branch protection or release policy explicitly requires hosted CI, local checks can support progress but cannot replace that external merge/release gate. Report `BLOCKED` only at the point that the unavailable remote gate actually prevents the requested outcome.

## Recorded local verification

Use `verify` to execute explicit project checks and persist evidence in the checkpoint:

```bash
python <skill-path>/scripts/gigiloop.py verify \
  --root . \
  --kind targeted \
  --tier T3 \
  --check "pytest -q" \
  --check "python -m compileall src"
```

Each check records command, exit status, evidence tier, classification, repository fingerprint, timestamps, and a bounded output tail. If any check mutates the repository, all evidence from that verification batch is marked `stale` and the command exits with code `2` (`VERIFY_REBASELINE_REQUIRED`). A failing check exits `1`; a clean current batch exits `0`.

Supervisor termination is process-tree aware: POSIX workers run in their own session/process group, and restarts terminate descendants before relaunching. Restart-budget or wall-clock exhaustion writes terminal `budget_exhausted` state instead of leaving the checkpoint falsely `active`.

## Runtime validation commands

These commands are designed to be identical locally and in CI:

```bash
python gigiloop/scripts/gigiloop.py self-test
python gigiloop/scripts/gigiloop.py validate-repo
python gigiloop/scripts/gigiloop.py pack --output dist/skill.zip
```

`pack` validates first and produces a deterministic ZIP with normalized timestamps. Repeated packaging of the same skill tree should produce identical bytes.

## Failure semantics

Treat runtime outputs as operational signals:

- `SELFTEST_OK` — core runtime invariants passed.
- `VALIDATION_OK` — repository structure and declared invariants passed.
- `RESUME_OK` — checkpoint still matches the relevant repository state.
- `RESUME_REBASELINE_REQUIRED` — repository drift was detected; stale evidence must be refreshed.
- supervisor restart-budget or wall-clock exhaustion — stop and report `BUDGET EXHAUSTED` unless a stronger blocker applies.

Never turn a runtime error into a success claim. Fix the runtime/configuration problem or document the exact limitation.
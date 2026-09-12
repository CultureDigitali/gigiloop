# Canonical GigiLoop checkpoint

Use `.gigiloop/checkpoint.json` as the authoritative resumable state when the project is writable.
Prefer `scripts/gigiloop.py` to create, validate, reconcile, and atomically update it.

The runtime checkpoint is intentionally machine-readable so a fresh agent can resume without trusting conversation memory or manually parsing prose.

## Core schema

The exact schema may gain backward-compatible fields, but these top-level records are required by the v0.4 runtime:

```json
{
  "schema_version": 2,
  "run_id": "uuid",
  "generation": 1,
  "status": "active",
  "iteration": 0,
  "profile": "balanced",
  "goal": "one-sentence goal",
  "scope": {"included": [], "excluded": []},
  "constraints": [],
  "budget": {
    "max_iterations": 25,
    "wall_clock": null,
    "cost_or_token_limit": null
  },
  "repository": {
    "mode": "git",
    "branch": "feature/example",
    "head_sha": "...",
    "dirty": true,
    "untracked": [],
    "fingerprint": "sha256",
    "protected_local_changes": [],
    "instructions_read": []
  },
  "verification_contract": {
    "tests": [],
    "thresholds": [],
    "snapshots_or_golden_files": [],
    "static_checks": [],
    "manual_acceptance": [],
    "approved_exceptions": []
  },
  "baseline": {
    "commands": [],
    "pre_existing_failures": [],
    "unavailable_checks": []
  },
  "rubric": [],
  "current_evidence": [],
  "findings": {
    "confirmed": [],
    "falsified": [],
    "hypotheses": []
  },
  "integrity": {
    "verifier_changes": [],
    "protected_work_conflicts": [],
    "destructive_operations": [],
    "integrity_blockers": []
  },
  "progress": {
    "previous_scores": {},
    "score_delta": null,
    "flat_iterations": 0,
    "last_material_change": null
  },
  "runtime": {
    "phase": "intake",
    "heartbeat_at": "2026-01-01T00:00:00Z",
    "last_resume_at": "2026-01-01T00:00:00Z",
    "resume_requires_rebaseline": false,
    "supervisor_restarts": 0,
    "last_exit_code": null,
    "host": null
  },
  "next_action": "establish baseline and verification contract",
  "created_at": "2026-01-01T00:00:00Z",
  "last_updated": "2026-01-01T00:00:00Z"
}
```

Terminal status values are:

- `success`
- `blocked`
- `budget_exhausted`
- `stopped`

## Runtime commands

Create the checkpoint:

```bash
python <skill-path>/scripts/gigiloop.py init --root . --goal "<goal>" --profile balanced
```

Inspect without rewriting:

```bash
python <skill-path>/scripts/gigiloop.py status --root . --json
```

Reconcile after resume/context reset/process restart:

```bash
python <skill-path>/scripts/gigiloop.py resume --root .
```

Persist iteration/phase/next action:

```bash
python <skill-path>/scripts/gigiloop.py checkpoint \
  --root . --increment --phase verify --next-action "run affected integration tests"
```

## Freshness rules

At every resume:

1. recompute repository state and fingerprint;
2. compare it with the checkpoint fingerprint;
3. if relevant state changed, mark current evidence stale and increment the checkpoint generation;
4. re-run the smallest sufficient affected checks before reusing scores;
5. continue from `next_action` only after reconciliation.

The fingerprint includes branch, HEAD, staged and unstaged diffs, and content of untracked files in Git repositories. `.git/**` and `.gigiloop/**` are excluded so runtime writes do not invalidate their own state.

## Atomicity and crash behavior

The runtime writes JSON to a temporary sibling file and then replaces the checkpoint atomically. This avoids a partially written canonical state after a normal process interruption.

A fresh agent must still validate the checkpoint before using it. A malformed, missing, or unsupported schema is a blocker to blind resume; reconstruct the state from repository evidence rather than guessing.

## Heartbeat

`runtime.heartbeat_at` is a liveness signal for optional supervision. Update it at meaningful phase boundaries; do not use it as evidence that verification succeeded.

A stale heartbeat may cause the supervisor to restart a restart-safe agent process. Restart behavior is governed by `references/runtime.md`.

## Write rules

Update the checkpoint:

- at the end of every material iteration;
- before an intentional stop or handoff;
- before/after a risky authorized operation;
- after repository drift reconciliation;
- before the final report.

Do not commit `.gigiloop/checkpoint.json`. Do not store secrets, access tokens, personal data, proprietary raw logs, or large command output. Store concise results and references instead.

## Manual fallback

When Python execution is unavailable, maintain an equivalent state file manually and preserve the same fields and freshness rules. State clearly that machine validation/atomic writes were unavailable; do not imply runtime checks occurred.
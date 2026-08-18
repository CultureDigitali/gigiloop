# Canonical GigiLoop checkpoint

Use `.gigiloop/checkpoint.md` as the authoritative resumable state when the project is writable.
Keep it compact, current, and sufficient for a fresh agent to resume without relying on conversation memory.

## Template

```yaml
loop_id: unique-id
status: active | success | blocked | budget_exhausted | stopped
iteration: 1
profile: balanced              # strict | balanced | fast

goal: one-sentence goal
scope:
  included: []
  excluded: []
constraints: []
budget:
  max_iterations: 25
  wall_clock: null
  cost_or_token_limit: null

repository:
  branch: null
  head_sha: null
  dirty_before: null
  current_dirty_state: null
  diff_fingerprint: null
  protected_local_changes: []
  repository_instructions_read: []

verification_contract:
  tests: []
  thresholds: []
  snapshots_or_golden_files: []
  static_checks: []
  manual_acceptance: []
  approved_exceptions: []

baseline:
  commands: []
  pre_existing_failures: []
  unavailable_checks: []

rubric:
  - name: Correctness
    weight: 40
    pass_definition: null
    critical_failure: null
    max_evidence_tier: T4
    score: 0
    evidence: []
    uncertainty: null

current_evidence:
  - id: evidence-1
    iteration: 1
    code_state: null            # HEAD / diff fingerprint
    method: null                # command, manual check, code inspection
    scope: null
    result: null
    exit_status: null
    evidence_tier: T0
    freshness: current          # current | stale | invalidated

findings:
  confirmed: []
  falsified: []
  hypotheses: []

integrity:
  verifier_changes: []
  protected_work_conflicts: []
  destructive_operations: []
  integrity_blockers: []

progress:
  previous_scores: {}
  score_delta: null
  flat_iterations: 0
  last_material_change: null

next_action: null
last_updated: null
```

## Freshness rules

At the start of every resumed turn:

1. compare branch, HEAD, dirty state, and diff fingerprint;
2. compare protected local changes and verification contract;
3. mark evidence stale when relevant code or test configuration changed;
4. re-run the smallest sufficient checks before reusing stale scores;
5. resume from `next_action` only after reconciliation.

## Write rules

Rewrite the checkpoint at the end of every iteration and immediately before an intentional stop, user handoff, risky authorized operation, or final report.

Do not store secrets, access tokens, personal data, or large raw logs. Store concise references and results.

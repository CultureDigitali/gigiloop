# Canonical GigiLoop checkpoint

Store the project checkpoint at `.gigiloop/checkpoint.md` when writable. Keep it concise enough to re-read at every resumed turn.

Recommended shape:

```yaml
loop_id: <stable id>
status: active | success | blocked | budget-exhausted | stopped
iteration: 0

 goal: <one sentence>
 scope:
   include: []
   exclude: []
 constraints: []
 budget:
   max_iterations: 25
   other: null

repository:
  branch: <branch or unknown>
  head_sha: <sha or unknown>
  dirty_before: <true|false|unknown>
  diff_fingerprint: <hash/summary or unknown>

verification:
  baseline_run: []
  baseline_failures: []
  commands:
    test: null
    lint: null
    typecheck: null
    build: null
  last_results: []

rubric:
  - name: Correctness
    weight: 40
    score: 0
    tier: T0
    evidence: []
    uncertainty: null

findings:
  confirmed: []
  falsified: []
  hypotheses: []

progress:
  previous_total: null
  flat_iterations: 0
  last_material_change: null

next_action: <single highest-impact next step>
```

## Resume protocol

Before trusting saved evidence:

1. Compare current branch and HEAD with the checkpoint.
2. Inspect whether the working diff changed outside the loop.
3. Mark evidence tied to changed code as stale.
4. Re-run the cheapest checks needed to restore confidence.
5. Resume from `next_action` only after stale-state reconciliation.

Do not treat the checkpoint as proof. It is an index of proof that may need revalidation.

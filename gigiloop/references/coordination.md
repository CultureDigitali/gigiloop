# Durable worker coordination

GigiLoop can persist multi-agent coordination separately from verification state using:

```text
.gigiloop/coordination.json
```

The coordination layer is optional and host-neutral. It does not automate ChatGPT, Codex, Claude, or any other provider UI. A host adapter may map real subagents or sessions onto the durable worker records.

## Design goals

The coordination runtime adds five properties that are useful for long-running agent systems:

1. **Stable worker identity** — a worker ID may survive context resets and host restarts.
2. **Sleeping reuse** — completed workers sleep by default instead of being destroyed, preserving their host session reference when the host supports it.
3. **Durable inbox** — messages survive crashes and are processed through leases and receipts rather than volatile in-memory queues.
4. **Finish boundary** — a worker cannot silently disappear while owning an unacknowledged message; finishing may atomically acknowledge the current task and claim the next queued task.
5. **Compact handoff** — context replacement creates a structured handoff snapshot without changing the underlying GigiLoop run identity.

## Initialize

First initialize the normal GigiLoop checkpoint, then coordination:

```bash
python <skill-path>/scripts/gigiloop.py init --root . --goal "<goal>" --profile balanced
python <skill-path>/scripts/coordination.py init --root . --max-active-workers 2
```

The coordination state is bound to the checkpoint `run_id`. If a new GigiLoop run replaces the checkpoint, the old coordination file is rejected instead of being silently reused.

## Worker lifecycle

Create or revive a worker:

```bash
python <skill-path>/scripts/coordination.py worker-start \
  --root . \
  --id builder-1 \
  --role builder \
  --session-ref "<host-session-id>"
```

A sleeping or failed worker may be revived with the same ID. If no new `--session-ref` is provided, the previous value is retained.

Put a worker to sleep:

```bash
python <skill-path>/scripts/coordination.py worker-sleep \
  --root . \
  --id builder-1 \
  --result "implementation complete"
```

Use `worker-fail` for an explicit failed worker. Use `finish --permanent` only when the worker identity must never be reused during that run.

The `max_active_workers` limit counts only workers in `working` state. Sleeping workers remain registered without consuming an active slot.

## Durable inbox

Queue work:

```bash
python <skill-path>/scripts/coordination.py send \
  --root . \
  --target builder-1 \
  --kind after_turn \
  --message "Run the retry regression fix" \
  --dedupe-key retry-fix-1
```

`--dedupe-key` makes repeated sends idempotent while a matching message is still queued or claimed.

Claim work:

```bash
python <skill-path>/scripts/coordination.py claim \
  --root . \
  --id builder-1 \
  --lease-seconds 300
```

The claim returns a receipt. Repeated claims by the same worker replay the existing claim while its lease is valid rather than incrementing attempts or duplicating execution.

Acknowledge explicitly:

```bash
python <skill-path>/scripts/coordination.py ack \
  --root . \
  --id builder-1 \
  --receipt "<receipt>"
```

If a claim lease expires, the message is requeued automatically on the next coordination mutation.

## Finish boundary

Prefer `finish` at a worker turn boundary:

```bash
python <skill-path>/scripts/coordination.py finish \
  --root . \
  --id builder-1 \
  --receipt "<receipt>" \
  --result "retry fix complete"
```

Semantics:

- if the worker owns a claimed message, the matching receipt is required;
- the current message is acknowledged atomically;
- if another message is queued for that worker, it is immediately claimed and the worker remains `working`;
- otherwise the worker becomes `sleeping`;
- `--permanent` moves it to `finished`.

This creates a deterministic handoff point between worker turns and prevents a worker from reporting completion while leaving an owned message ambiguous.

## Immediate vs after-turn work

Message kinds are:

- `immediate` — higher priority when selecting the next queued task;
- `after_turn` — normal follow-up work.

The runtime does not interrupt an operating-system process by itself. Host adapters decide how to map `immediate` delivery to their own cancellation/interruption primitives.

## Compact handoff

Before replacing a long context or after a host restart, write a compact handoff:

```bash
python <skill-path>/scripts/coordination.py handoff \
  --root . \
  --reason compact \
  --summary "Auth implementation complete; verifier found one timeout edge case." \
  --next-action "Builder fixes timeout propagation, then verifier reruns integration tests."
```

A handoff records:

- stable run ID;
- old and new context generations;
- checkpoint generation and repository fingerprint;
- current GigiLoop iteration;
- worker IDs, roles, states, host session refs, and turn counts;
- queued/claimed message IDs;
- summary and exact next action.

A new context must run normal `gigiloop.py resume` as well. The handoff preserves orchestration context; it does not override repository-fingerprint reconciliation or stale-evidence rules.

## Host integration rules

Adapters should follow these rules:

- Map a host subagent/session identifier to `session_ref` when one exists.
- Reuse sleeping workers before creating unnecessary new workers.
- Never share hidden chain-of-thought between workers; share task inputs, observable evidence, summaries, and results only.
- Treat message receipts as delivery ownership, not as proof that the requested engineering work succeeded.
- Keep verification evidence in `checkpoint.json`; keep orchestration state in `coordination.json`.
- Do not store secrets, tokens, private customer data, or large raw logs in either file.
- A host without persistent subagents may still use the queue and handoff semantics with sequential logical workers.

## Status and recovery

Inspect coordination state:

```bash
python <skill-path>/scripts/coordination.py status --root . --json
```

The coordination file is excluded from repository fingerprints because it lives under `.gigiloop/**`. This prevents worker bookkeeping from invalidating code evidence.

If the process crashes after claiming work but before acknowledging it, the lease eventually expires and another turn may reclaim the task. Choose a lease long enough for normal task execution, and use idempotent task design for external side effects.

## Security boundary

The coordination runtime stores only state. It does not grant shell, filesystem, browser, or network access.

Any host that executes queued work must enforce its own tool permissions, sandboxing, approval policy, and destructive-operation safeguards. GigiLoop coordination must not be used to turn a non-idempotent payment, migration, deployment, deletion, or other irreversible operation into an automatically retryable task without a separate safety design.

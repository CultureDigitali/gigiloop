# Host portability rules

Keep GigiLoop's correctness contract stable across agent hosts. Treat host-specific features as accelerators, not requirements.

## Capability discovery

At intake, determine whether the host provides:

- native Agent Skills discovery;
- subagents or fresh-context reviewers;
- persistent task/todo state;
- shell and repository tools;
- Python 3 for the bundled runtime;
- hooks or deterministic command execution;
- worktrees, branches, or isolated sandboxes;
- resumable session state;
- restart-safe CLI entrypoints that can be supervised.

Record unavailable capabilities only when they affect the selected profile or evidence ceiling.

## Fallback matrix

| Capability | Preferred use | Fallback |
|---|---|---|
| Fresh-context reviewer | independent red-team and T5 evidence | separate hostile review pass in current agent; cap confidence appropriately |
| Persistent task state | mirror iteration and next action | `.gigiloop/checkpoint.json` via bundled runtime |
| Hooks | deterministic baseline/final checks | run commands explicitly and record results |
| Worktree/sandbox | isolate loop changes from user work | use a branch or narrowly scoped edits; preserve/re-check local changes |
| Native skill loading | install canonical `gigiloop/SKILL.md` | documented adapter such as `AGENTS.md`, `GEMINI.md`, or Cursor `.mdc` |
| Long-running session | continuous loop | checkpoint each iteration; resume after fingerprint reconciliation |
| Hosted CI | independent remote verification | local runtime + project checks + reviewer; remote CI remains required only when repository policy requires it |
| Host-native restart | recover from idle/crash | optional `scripts/gigiloop.py supervise` for restart-safe CLI commands |
| Python 3 | deterministic checkpoint/runtime operations | manual equivalent state with lower automation guarantee |

## Hosted CI is not the state store

GitHub Actions or another hosted CI service may be unavailable because of quota, billing, infrastructure, permissions, queue delay, or service outage. GigiLoop must not lose progress or forget its next action for those reasons.

Use this order:

1. preserve local checkpoint and repository state;
2. run local deterministic GigiLoop validation;
3. run repository tests/lint/type/build/security checks locally;
4. use an independent reviewer/subagent when available;
5. add remote CI evidence when available.

If branch protection, release policy, or the user explicitly requires a hosted check, do not counterfeit that gate. Continue development locally and report the remote requirement as a blocker only when it prevents the requested merge/release outcome.

## Profile portability

Preserve profile meaning across hosts:

- **strict:** do not claim full strict completion when required integration checks or independent review are unavailable; report the evidence ceiling or blocker.
- **balanced:** use targeted checks, milestone checks, reconciliation, and a full relevant final gate.
- **fast:** honor the explicit budget but retain baseline, integrity, reconciliation, and honest-exit rules.

Never silently downgrade a profile because a host lacks a feature.

## State authority

Use host task lists, plans, or todos as mirrors only. `.gigiloop/checkpoint.json` is canonical when writable because UI state may disappear, compact, or drift.

At resume, recompute repository fingerprint before trusting stored evidence. Follow `runtime.md` and `checkpoint.md`.

## Tool truthfulness

Do not claim a command, test, review, CI job, or inspection ran merely because the host normally supports it. Record actual results only.

When a required check cannot run:

1. record the missing capability;
2. run the strongest available substitute;
3. lower the evidence tier when appropriate;
4. report the limitation in final status.

## Multi-agent mapping

When subagents are available, map them to Builder, Verifier, Red Team, Judge, and optional Improver roles from `orchestration.md`.

When only one context exists, execute the roles sequentially. Same-context role separation is useful for discipline but is not independent review and cannot by itself justify T5.

## Included adapters

- OpenCode / native Agent Skills: `gigiloop/SKILL.md`
- Claude Code / native Agent Skills: `gigiloop/SKILL.md`
- Codex convenience wrapper: `adapters/codex/AGENTS.md`
- Cursor convenience rule: `.cursor/rules/gigiloop.mdc`
- Gemini CLI convenience wrapper: `adapters/gemini-cli/GEMINI.md`

Adapters must remain short and refer back to the canonical skill so behavior does not drift.

# Host portability rules

Keep GigiLoop's correctness contract stable across agent hosts. Treat host-specific features as accelerators, not requirements.

## Capability discovery

At intake, determine whether the host provides:

- native Agent Skills discovery;
- subagents or fresh-context reviewers;
- persistent task/todo state;
- shell and repository tools;
- hooks or deterministic command execution;
- worktrees, branches, or isolated sandboxes;
- resumable session state.

Record unavailable capabilities only when they affect the selected profile or evidence ceiling.

## Fallback matrix

| Capability | Preferred use | Fallback |
|---|---|---|
| Fresh-context reviewer | independent red-team and T5 evidence | separate hostile review pass in current agent; cap confidence appropriately |
| Persistent task state | mirror iteration and next action | use `.gigiloop/checkpoint.md` as authoritative state |
| Hooks | deterministic baseline/final checks | run the commands explicitly and record results |
| Worktree/sandbox | isolate loop changes from user work | use a branch or narrowly scoped edits; preserve and re-check local changes |
| Native skill loading | install canonical `gigiloop/SKILL.md` | use a documented adapter such as `AGENTS.md`, `GEMINI.md`, or Cursor `.mdc` |
| Long-running session | continuous loop | checkpoint after every iteration and resume only after stale-state reconciliation |

## Profile portability

Preserve the meaning of profiles across hosts:

- **strict:** do not claim full strict completion when required integration checks or independent review are unavailable; report the evidence ceiling or blocker.
- **balanced:** use targeted checks, milestone checks, reconciliation, and a full relevant final gate.
- **fast:** honor the explicit budget but retain baseline, integrity, reconciliation, and honest-exit rules.

Never silently downgrade a profile because the host lacks a feature.

## State authority

Use the host's task list, plan, or todo system when helpful, but treat `.gigiloop/checkpoint.md` as the canonical resumable state. Host UI state may disappear or drift.

## Tool truthfulness

Do not claim a command, test, review, or inspection ran merely because the host normally supports it. Record only actual tool results.

When a host cannot execute a required check:

1. record the missing capability;
2. run the strongest available substitute;
3. lower the evidence tier when appropriate;
4. report the limitation in the final status.

## Included adapters

- OpenCode / native Agent Skills: `gigiloop/SKILL.md`
- Claude Code / native Agent Skills: `gigiloop/SKILL.md`
- Codex convenience wrapper: `adapters/codex/AGENTS.md`
- Cursor convenience rule: `.cursor/rules/gigiloop.mdc`
- Gemini CLI convenience wrapper: `adapters/gemini-cli/GEMINI.md`

Adapters must remain short and refer back to the canonical skill so behavior does not drift.

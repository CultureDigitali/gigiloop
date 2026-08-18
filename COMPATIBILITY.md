# GigiLoop compatibility guide

GigiLoop has one canonical source of truth:

- `gigiloop/SKILL.md`
- supporting rules in `gigiloop/references/`

Host adapters are intentionally small wrappers. They improve discovery but must not redefine the workflow.

## Recommended installation

```bash
npx skills add CultureDigitali/gigiloop --skill gigiloop
```

The Agent Skills CLI can detect supported agents interactively. Target a host explicitly with `-a` and add `-g` for a user-level installation.

```bash
# OpenCode
npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode

# Claude Code
npx skills add CultureDigitali/gigiloop --skill gigiloop -a claude-code

# Codex
npx skills add CultureDigitali/gigiloop --skill gigiloop -a codex

# Cursor
npx skills add CultureDigitali/gigiloop --skill gigiloop -a cursor

# Gemini CLI
npx skills add CultureDigitali/gigiloop --skill gigiloop -a gemini-cli

# Every detected supported target
npx skills add CultureDigitali/gigiloop --skill gigiloop --agent '*'
```

## Support levels

| Host | Canonical skill | Convenience adapter | Notes |
|---|---:|---:|---|
| OpenCode | ✅ | — | direct Agent Skill |
| Claude Code | ✅ | — | direct Agent Skill |
| Codex | ✅ | `adapters/codex/AGENTS.md` | adapter is optional |
| Cursor | ✅ | `.cursor/rules/gigiloop.mdc` | adapter improves rule discovery |
| Gemini CLI | ✅ | `adapters/gemini-cli/GEMINI.md` | adapter is optional |
| GitHub Copilot | ✅ | — | use Agent Skills installation target |
| Cline | ✅ | — | use Agent Skills installation target |
| OpenHands | ✅ | — | use Agent Skills installation target |
| Amp | ✅ | — | use Agent Skills installation target |
| Other Agent Skills hosts | usually | host-dependent | preserve the canonical integrity and evidence contract |

“Supported” means the host can consume the instruction set. It does not guarantee identical subagent, hook, shell, persistence, or sandbox capabilities.

## Behavioral compatibility contract

Every host must preserve these behaviors:

1. choose and record `strict`, `balanced`, or `fast` without silently downgrading;
2. baseline repository state, failures, and protected local work;
3. define measurable acceptance criteria and a verification contract;
4. verify with current evidence tied to the current code state;
5. reject test/threshold weakening used merely to obtain green output;
6. preserve unrelated user work and avoid unauthorized destructive operations;
7. adversarially review and reconcile before completion;
8. checkpoint progress and invalidate stale evidence after external changes;
9. exit as `SUCCESS`, `BLOCKED`, `BUDGET EXHAUSTED`, or `STOPPED` with a current evidence report.

Read `gigiloop/references/hosts.md` for capability fallbacks.

## Manual installation

Copy the complete `gigiloop/` directory into the Agent Skills directory used by the host. Do not copy only `SKILL.md`; the references, metadata, and logo asset are part of the skill bundle.

When a host does not discover generic skills, use the relevant adapter and keep its reference to the canonical skill.

## Fresh-context review

A separate reviewer/subagent provides stronger evidence but is not universally available.

- When available, prefer it for strict mode and potential T5 scoring.
- When unavailable, use a distinct adversarial pass and cap confidence according to `gigiloop/references/scoring.md`.
- Never claim independent review when the same context performed both implementation and approval.

## State and checkpoints

Host task lists and plans are useful mirrors. `.gigiloop/checkpoint.md` remains authoritative because host UI state may disappear, compact, or drift.

## Branding and endorsement

The approved compatibility artwork is stored at `assets/gigiloop-compatibility.jpg`. Third-party product names and marks identify compatibility only; they do not imply endorsement, sponsorship, or partnership.

See `assets/BRANDING.md` for the canonical GigiLoop visual assets and change-control rules.

# GigiLoop compatibility

GigiLoop keeps one canonical Agent Skill in [`gigiloop/SKILL.md`](gigiloop/SKILL.md). The core protocol is deliberately host-neutral so the same evidence-first loop can run across modern coding agents that support the Agent Skills format.

The open `skills` CLI currently supports OpenCode, Claude Code, Codex, Cursor, Gemini CLI and dozens of additional hosts. Use the canonical skill whenever the host supports Agent Skills directly; use the included adapters only when a host-specific workflow file is useful.

## First-class targets

| Host | Agent Skills target | Typical project path | GigiLoop status |
|---|---|---|---|
| OpenCode | `opencode` | `.agents/skills/` | Native skill |
| Claude Code | `claude-code` | `.claude/skills/` | Native skill |
| Codex | `codex` | `.agents/skills/` | Native skill + `AGENTS.md` adapter |
| Cursor | `cursor` | `.agents/skills/` | Native skill + optional `.mdc` adapter |
| Gemini CLI | `gemini-cli` | `.agents/skills/` | Native skill + `GEMINI.md` adapter |
| GitHub Copilot | `github-copilot` | `.agents/skills/` | Native skill |
| Cline | `cline` | `.agents/skills/` | Native skill |
| OpenHands | `openhands` | `.openhands/skills/` | Native skill |
| Amp | `amp` | `.agents/skills/` | Native skill |

The wider Agent Skills ecosystem includes many more compatible hosts. Run `npx skills add CultureDigitali/gigiloop --skill gigiloop` interactively to select the agents detected on your machine, or target specific hosts with `-a`.

## Installation examples

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

# GitHub Copilot
npx skills add CultureDigitali/gigiloop --skill gigiloop -a github-copilot

# Install to every detected agent
npx skills add CultureDigitali/gigiloop --skill gigiloop --agent '*'
```

Add `-g` for a global installation.

## Host-neutral behavior

GigiLoop preserves the same core contract on every host:

1. establish the repository baseline before edits;
2. turn the goal into measurable acceptance criteria;
3. work on the highest-impact verified gap;
4. run targeted checks while iterating and broader checks at milestones;
5. score only from evidence;
6. red-team the current diff without inventing findings;
7. reconcile scores after critique;
8. change strategy when progress plateaus;
9. checkpoint enough state to resume safely;
10. require a final verification gate before declaring success.

Host-specific capabilities are enhancements, not dependencies. If the host exposes subagents, a fresh-context reviewer, todo/task state, hooks, or native checkpoints, GigiLoop may use them. If those features are absent, the canonical protocol still works.

## Included adapters

- [`adapters/codex/AGENTS.md`](adapters/codex/AGENTS.md)
- [`adapters/gemini-cli/GEMINI.md`](adapters/gemini-cli/GEMINI.md)
- [`.cursor/rules/gigiloop.mdc`](.cursor/rules/gigiloop.mdc)

These wrappers intentionally remain small. The canonical `gigiloop/SKILL.md` is the source of truth.

## Trademark note

Product names and third-party marks belong to their respective owners. Compatibility means GigiLoop can be installed or adapted for the named host; it does **not** imply endorsement, sponsorship, or partnership.
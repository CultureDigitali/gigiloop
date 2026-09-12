# GigiLoop compatibility guide

GigiLoop has one canonical source of truth:

- `gigiloop/SKILL.md`
- supporting rules in `gigiloop/references/`
- deterministic runtime in `gigiloop/scripts/gigiloop.py`

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

| Host | Canonical skill | Convenience adapter | v0.4 runtime |
|---|---:|---:|---:|
| OpenCode | ✅ | — | ✅ when Python/shell are available |
| Claude Code | ✅ | — | ✅ when Python/shell are available |
| Codex | ✅ | `adapters/codex/AGENTS.md` | ✅ when Python/shell are available |
| Cursor | ✅ | `.cursor/rules/gigiloop.mdc` | ✅ when Python/shell are available |
| Gemini CLI | ✅ | `adapters/gemini-cli/GEMINI.md` | ✅ when Python/shell are available |
| GitHub Copilot | ✅ | — | host-dependent |
| Cline | ✅ | — | host-dependent |
| OpenHands | ✅ | — | host-dependent |
| Amp | ✅ | — | host-dependent |
| Other Agent Skills hosts | usually | host-dependent | manual fallback if Python unavailable |

“Supported” means the host can consume the instruction set. It does not guarantee identical subagent, hook, shell, persistence, sandbox, or process-supervision capabilities.

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
9. recover safely from context/process interruption when the host supports persistence or the bundled runtime;
10. continue locally when hosted CI quota is exhausted unless policy explicitly requires the remote gate;
11. exit as `SUCCESS`, `BLOCKED`, `BUDGET EXHAUSTED`, or `STOPPED` with a current evidence report.

Read `gigiloop/references/hosts.md` for capability fallbacks.

## v0.4 local runtime

When Python 3 is available, use:

```bash
python <skill-path>/scripts/gigiloop.py doctor --root .
python <skill-path>/scripts/gigiloop.py init --root . --goal "<goal>" --profile balanced
python <skill-path>/scripts/gigiloop.py resume --root .
```

The runtime provides:

- atomic `.gigiloop/checkpoint.json` state;
- repository fingerprinting including untracked file contents;
- stale-evidence invalidation after repository drift;
- heartbeat and phase tracking;
- optional supervision/restart for restart-safe CLI agents;
- local repository validation and deterministic packaging;
- the same validator used by GitHub Actions.

The runtime has no third-party Python dependency.

## GitHub Actions quota / outage behavior

Hosted CI is evidence, not the persistence layer.

If GitHub Actions minutes are exhausted or the service is unavailable:

1. retain `.gigiloop/checkpoint.json` locally;
2. run GigiLoop `self-test` / `validate-repo` locally where relevant;
3. run the target project's own tests, lint, typecheck, build, security, and integration checks locally;
4. use fresh-context/subagent review when available;
5. continue the loop;
6. return to hosted CI when available or when branch protection/release policy requires it.

Never claim a required remote check passed when it did not run.

## Manual installation

Copy the complete `gigiloop/` directory into the Agent Skills directory used by the host. Do not copy only `SKILL.md`; references, runtime script, metadata, and logo asset are part of the bundle.

When a host does not discover generic skills, use the relevant adapter and keep its reference to the canonical skill.

## Fresh-context review and multi-agent roles

A separate reviewer/subagent provides stronger evidence but is not universally available.

- When available, map subagents to Builder, Verifier, Red Team, Judge, and optional Improver roles from `gigiloop/references/orchestration.md`.
- Prefer independent review for strict mode and potential T5 scoring.
- When unavailable, use a distinct sequential adversarial pass and cap confidence according to `gigiloop/references/scoring.md`.
- Never claim independent review when the same context performed both implementation and approval.

## State and checkpoints

Host task lists and plans are useful mirrors. `.gigiloop/checkpoint.json` remains authoritative because host UI state may disappear, compact, or drift.

At resume, reconcile the checkpoint against the current repository fingerprint before trusting stored evidence.

## Branding and endorsement

The approved compatibility artwork is stored at `assets/gigiloop-compatibility.jpg`. Third-party product names and marks identify compatibility only; they do not imply endorsement, sponsorship, or partnership.

See `assets/BRANDING.md` for the canonical GigiLoop visual assets and change-control rules.

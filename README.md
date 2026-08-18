<p align="center">
  <img src="assets/gigiloop-superbanner.jpg" alt="GigiLoop — One skill. Many agent hosts. Verification-First Autonomous Coding." width="100%" />
</p>

<h1 align="center">GigiLoop</h1>

<p align="center">
  <strong>The autonomous coding loop that does not trust itself.</strong><br>
  Baseline → Build → Verify → Red-team → Reconcile → Repeat.
</p>

<p align="center">
  <a href="#quick-start"><strong>Quick start</strong></a> ·
  <a href="#works-across-agent-hosts">Compatibility</a> ·
  <a href="COMPATIBILITY.md">Host guide</a> ·
  <a href="benchmarks/README.md">Benchmarks</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

GigiLoop is a **verification-first autonomous coding skill** for OpenCode, Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot, Cline, OpenHands, Amp, and the wider Agent Skills ecosystem.

It turns “keep trying until it works” into a bounded engineering protocol:

1. baseline the repository and protect existing work;
2. define measurable acceptance criteria;
3. fix the highest-impact gap;
4. verify without weakening the tests;
5. attack the diff with an adversarial review;
6. reconcile the scores after critique;
7. ship only after a final evidence gate.

> **Think Ralph Loop, but with a hostile reviewer, protected user work, and an evidence contract.**

No vibes. No “looks good to me.” **Proof or it does not pass.**

---

## Why GigiLoop exists

Naive autonomous loops fail in predictable ways. GigiLoop builds controls around those failure modes.

| Naive autonomous loop | GigiLoop |
|---|---|
| “Rate yourself and keep going” | **Evidence-gated scoring** — tests, command output, reproducible behavior, or precise code references. |
| Treats every failing test as its own regression | **Baseline awareness** — pre-existing failures are recorded before edits. |
| Critiques itself, then ignores the critique | **Score reconciliation** — confirmed findings can lower a previously passing score. |
| Deletes or relaxes tests to turn CI green | **Verification integrity** — weakening the verifier blocks completion. |
| Overwrites unrelated local work | **Protected-work guard** — pre-existing edits are tracked and re-checked. |
| Invents problems to satisfy a quota | **Evidence-gated red-team** — up to three material findings; never fabricate one. |
| Uses the same context to approve its own work | **Independent reviewer when available** — fresh-context or subagent review is preferred. |
| Re-runs an expensive full suite after every tiny edit | **Progressive verification** — fast affected checks while iterating, broad checks at milestones and the final gate. |
| Loops forever on a plateau | **Plateau → re-strategy** — repeated flat progress forces a genuinely different approach. |
| Resumes from stale memory after the repo changed | **Checkpoint validation** — stale evidence is invalidated when repository state changes. |
| Averages strong criteria over weak ones | **All-criteria gate** — every critical criterion must clear the requested threshold. |
| Never admits defeat | **Honest exit** — blockers and budgets produce reports instead of fabricated success. |

---

## Works across agent hosts

<p align="center">
  <img src="assets/gigiloop-compatibility.jpg" alt="GigiLoop compatibility with OpenCode, Claude Code, Codex, Cursor, and Gemini CLI" width="100%" />
</p>

The canonical [`gigiloop/SKILL.md`](gigiloop/SKILL.md) is host-neutral. Small adapters are included where a native workflow file improves discovery or usability.

| Host | Install target | Support |
|---|---|---|
| **OpenCode** | `opencode` | ✅ Native Agent Skill |
| **Claude Code** | `claude-code` | ✅ Native Agent Skill |
| **Codex** | `codex` | ✅ Native + `AGENTS.md` adapter |
| **Cursor** | `cursor` | ✅ Native + optional `.mdc` rule |
| **Gemini CLI** | `gemini-cli` | ✅ Native + `GEMINI.md` adapter |
| **GitHub Copilot** | `github-copilot` | ✅ Agent Skill |
| **Cline** | `cline` | ✅ Agent Skill |
| **OpenHands** | `openhands` | ✅ Agent Skill |
| **Amp** | `amp` | ✅ Agent Skill |
| **Many more** | interactive detection | ✅ Agent Skills ecosystem |

See [`COMPATIBILITY.md`](COMPATIBILITY.md) for paths, adapters, and portability rules.

Compatibility is descriptive and does not imply vendor endorsement. See [`assets/BRANDING.md`](assets/BRANDING.md) for visual and trademark guidance.

---

## Quick start

### Install once

```bash
npx skills add CultureDigitali/gigiloop --skill gigiloop
```

Choose a detected agent interactively, or target one explicitly:

```bash
npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode
npx skills add CultureDigitali/gigiloop --skill gigiloop -a claude-code
npx skills add CultureDigitali/gigiloop --skill gigiloop -a codex
npx skills add CultureDigitali/gigiloop --skill gigiloop -a cursor
npx skills add CultureDigitali/gigiloop --skill gigiloop -a gemini-cli
```

Add `-g` for a global/user-level installation. Use `--agent '*'` to install to all detected supported targets.

### Give it a concrete goal

```text
gigiloop: fix the login regression and harden the auth flow until every acceptance criterion is verified at 9/10
```

```text
gigiloop: build an idempotent payments endpoint with tests; do not stop at the happy path
```

```text
gigiloop: keep iterating until the failing CI job is fixed without weakening or skipping any checks
```

Natural constraints work too:

```text
strict profile
max 8 iterations
stay inside src/auth/**
do not change the public API
preserve my current uncommitted edits
```

---

## Choose the loop profile

GigiLoop adapts verification depth without changing its core integrity rules.

| Profile | Best for | Quality contract |
|---|---|---|
| **strict** | authentication, payments, security, migrations, production incidents, public APIs | regression test + integration checks + independent review when available + full final gate |
| **balanced** | ordinary features, refactors, and bug fixes | targeted iteration checks + milestone regression checks + adversarial review + full relevant final gate |
| **fast** | explicit budget or low-risk exploration | baseline + meaningful reproducible check + reconciliation + strongest final checks allowed by the budget |

Default: **balanced**. High-consequence work automatically escalates toward **strict**. GigiLoop never silently downgrades the requested profile.

---

## The loop

```text
GOAL + PROFILE + BASELINE + RUBRIC
                 │
                 ▼
             A. WORK
                 │
                 ▼
            B. VERIFY
                 │
                 ▼
             C. SCORE
                 │
                 ▼
           D. RED-TEAM
                 │
                 ▼
           E. RECONCILE
                 │
                 ▼
            F. DECIDE
            │         │
       below bar   all criteria
            │       verified
            └───┐     ▼
                └─ FINAL GATE ──► DONE
```

![GigiLoop protocol diagram](assets/loop.svg)

---

## Verification integrity

A green check is not valid when the agent changed the rules merely to obtain it.

GigiLoop explicitly guards against:

- deleted, skipped, quarantined, or weakened tests;
- reduced assertions or over-mocked behavior;
- lowered coverage, lint, type, performance, or security thresholds;
- disabled hooks, workflows, compiler flags, or strict mode;
- blindly updated snapshots or golden files;
- acceptance criteria changed after seeing the implementation;
- hidden failures, swallowed errors, or narrowed verification scope;
- destructive Git/data operations without authorization;
- overwritten or entangled pre-existing user edits.

A genuinely incorrect test may be changed only with evidence, replacement verification, and a documented explanation. See [`gigiloop/references/integrity.md`](gigiloop/references/integrity.md).

---

## Evidence tiers

A strong score requires proportionally strong, current evidence.

| Tier | Typical evidence | Practical ceiling |
|---|---|---:|
| **T0** | intuition, unsupported claim, or stale evidence | 4/10 |
| **T1** | static inspection only | 6/10 |
| **T2** | lint, typecheck, build, deterministic static checks | 7/10 |
| **T3** | targeted automated tests or reproducible behavior | 8/10 |
| **T4** | tests + adversarial edge cases + relevant integration verification | 9/10 |
| **T5** | T4 + independent/fresh-context review + clean final and integrity gates | 10/10 |

Evidence is tied to a code state. A relevant edit invalidates stale evidence before the score can be reused.

See [`gigiloop/references/scoring.md`](gigiloop/references/scoring.md).

---

## Resumable without trusting stale state

When the project is writable, GigiLoop uses `.gigiloop/checkpoint.md` to track:

- goal, scope, profile, constraints, and budget;
- branch, HEAD, dirty state, and protected local changes;
- verification contract and baseline failures;
- rubric, scores, evidence tiers, and evidence freshness;
- confirmed findings, hypotheses, and integrity exceptions;
- iteration count and next action.

When the repository changes outside the loop, affected evidence is invalidated before resuming. See [`gigiloop/references/checkpoint.md`](gigiloop/references/checkpoint.md).

---

## Exit report

GigiLoop exits with one explicit status:

- `SUCCESS`
- `BLOCKED`
- `BUDGET EXHAUSTED`
- `STOPPED`

The report distinguishes what was **verified**, **inferred**, **unverified**, and **blocked**, and includes the scorecard, commands/checks, integrity state, remaining risks, and next action. See [`gigiloop/references/reporting.md`](gigiloop/references/reporting.md).

---

## Benchmarks: prove it, do not market fiction

GigiLoop does **not** publish invented success rates. [`benchmarks/README.md`](benchmarks/README.md) defines a reproducible comparison protocol for:

- a normal one-pass coding agent;
- a naive keep-going / Ralph-style loop;
- GigiLoop;

from the same starting commit, under comparable model/host budgets and independent or hidden checks where practical.

Measured claims belong in the README only when raw evidence can reconstruct them.

---

## Repository layout

```text
.
├── README.md
├── COMPATIBILITY.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── SECURITY.md
├── assets/
│   ├── gigiloop-logo.jpg
│   ├── gigiloop-superbanner.jpg
│   ├── gigiloop-compatibility.jpg
│   ├── visual-manifest.json
│   ├── loop.svg
│   └── BRANDING.md
├── adapters/
│   ├── codex/AGENTS.md
│   └── gemini-cli/GEMINI.md
├── .cursor/rules/gigiloop.mdc
├── benchmarks/README.md
├── marketing/
└── gigiloop/
    ├── SKILL.md
    ├── assets/gigiloop-logo.jpg
    ├── agents/openai.yaml
    └── references/
        ├── checkpoint.md
        ├── hosts.md
        ├── integrity.md
        ├── reporting.md
        ├── scoring.md
        └── verification.md
```

---

## Contributing

Reproducible benchmark cases, failure reports, portability fixes, host adapters, and reviewer strategies are especially useful. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

If GigiLoop catches something a normal coding pass would have shipped, open a benchmark case with the evidence.

---

## 📣 Share

Ready-to-post launch drafts for free developer channels live in [`marketing/`](marketing/), including Show HN, Reddit, X, LinkedIn, Product Hunt, Lobsters, and the self-case-study.

Discussion: [github.com/CultureDigitali/gigiloop/discussions](https://github.com/CultureDigitali/gigiloop/discussions)

---

## License

MIT. Third-party names and marks remain property of their respective owners and are referenced only to describe compatibility.

---

⭐ **If GigiLoop improves a real result, star the repository and share the reproducible case. Evidence is more useful than hype.**

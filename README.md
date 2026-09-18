<p align="center">
  <img src="assets/gigiloop-superbanner.jpg" alt="GigiLoop — One skill. Many agent hosts. Verification-First Autonomous Coding." width="100%" />
</p>

<h1 align="center">GigiLoop</h1>

<p align="center">
  <strong>The autonomous coding loop that does not trust itself — and can resume after the agent forgets.</strong><br>
  Baseline → Build → Verify → Red-team → Reconcile → Repeat.
</p>

<p align="center">
  <a href="#quick-start"><strong>Quick start</strong></a> ·
  <a href="#v040-resumable-runtime">v0.4 Runtime</a> ·
  <a href="#multi-agent-adversarial-loop">Multi-agent loop</a> ·
  <a href="COMPATIBILITY.md">Compatibility</a> ·
  <a href="benchmarks/README.md">Benchmarks</a>
</p>

GigiLoop is a **verification-first autonomous coding skill + local runtime** for OpenCode, Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot, Cline, OpenHands, Amp, and the wider Agent Skills ecosystem.

It turns “keep trying until it works” into a bounded engineering protocol that can survive context resets, process exits, stale sessions, repository drift, and exhausted GitHub Actions minutes.

> **Proof or it does not pass. Checkpoint or it does not resume.**

---

## Why GigiLoop exists

Naive autonomous coding loops fail predictably:

| Failure mode | GigiLoop control |
|---|---|
| “Rate yourself and keep going” | **Evidence-gated scoring** tied to actual tests, commands, behavior, or code state. |
| Treat every red test as a new regression | **Baseline awareness** separates pre-existing failures. |
| Critique yourself, then ignore it | **Mandatory reconciliation** after adversarial review. |
| Delete/relax tests to obtain green output | **Verification-integrity contract** blocks fake passes. |
| Overwrite unrelated user work | **Protected-work guard** records and re-checks local changes. |
| Loop forever without progress | **Plateau detection** forces re-strategy. |
| Lose state after context reset | **Atomic machine checkpoint** in `.gigiloop/checkpoint.json`. |
| Trust stale evidence after files changed | **Repository fingerprint reconciliation** invalidates stale evidence. |
| Stop when the coding agent goes idle | **Heartbeat + optional supervisor** can restart restart-safe CLI agents. |
| Depend on GitHub Actions to keep working | **Local-first validation** continues when hosted CI quota is exhausted. |
| One agent writes, tests, and approves itself | **Builder / Verifier / Red Team / Judge** separation when subagents exist. |
| “Improve it” becomes endless scope creep | **Measured Improver pass** only after correctness is stable. |

---

## Quick start

### Install once

```bash
npx skills add CultureDigitali/gigiloop --skill gigiloop
```

Target a host explicitly when useful:

```bash
npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode
npx skills add CultureDigitali/gigiloop --skill gigiloop -a claude-code
npx skills add CultureDigitali/gigiloop --skill gigiloop -a codex
npx skills add CultureDigitali/gigiloop --skill gigiloop -a cursor
npx skills add CultureDigitali/gigiloop --skill gigiloop -a gemini-cli
```

Add `-g` for global/user installation. Use `--agent '*'` for all detected supported targets.

### Give the agent a measurable goal

```text
gigiloop: fix the login regression and keep iterating until every critical criterion is verified at 9/10
```

```text
gigiloop: build an idempotent payments endpoint; use strict mode and do not stop at the happy path
```

```text
gigiloop: harden this project, then run one self-improvement pass and keep only improvements that survive verification and red-team review
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

## v0.4.0: resumable runtime

The skill now ships a single-file, standard-library Python runtime:

```text
gigiloop/scripts/gigiloop.py
```

No third-party Python package is required.

### Inspect host capability

```bash
python <skill-path>/scripts/gigiloop.py doctor --root .
```

### Start a resumable loop

```bash
python <skill-path>/scripts/gigiloop.py init \
  --root . \
  --goal "fix the login regression" \
  --profile balanced
```

This creates:

```text
.gigiloop/checkpoint.json
```

The checkpoint is deliberately ignored by Git and records the run ID, generation, profile, iteration, budget, repository fingerprint, evidence, findings, integrity state, phase, heartbeat, restart counters, and exact next action.

### Resume after a crash/context reset

```bash
python <skill-path>/scripts/gigiloop.py resume --root .
```

If repository state changed after the checkpoint, GigiLoop marks affected evidence stale instead of trusting yesterday’s score.

### Record heartbeat / phase

```bash
python <skill-path>/scripts/gigiloop.py heartbeat --root . --phase verify --message "running auth suite"
```

### Advance state atomically

```bash
python <skill-path>/scripts/gigiloop.py checkpoint \
  --root . \
  --increment \
  --phase work \
  --next-action "fix retry idempotency"
```

### Supervise a restart-safe CLI agent

```bash
python <skill-path>/scripts/gigiloop.py supervise \
  --root . \
  --idle-seconds 900 \
  --max-restarts 10 \
  -- your-agent-command --resume-from .gigiloop/checkpoint.json
```

The supervisor watches the heartbeat, restarts after unexpected exit or stale heartbeat, and respects restart/wall-clock budgets.

**Important:** supervision only applies to commands that are safe to restart. It does not make destructive migrations, production deployments, payments, or other non-idempotent operations safe.

See [`gigiloop/references/runtime.md`](gigiloop/references/runtime.md).

---

## Durable workers and compact handoff

For hosts that expose subagents or persistent sessions, GigiLoop includes an optional standard-library coordination runtime:

```text
gigiloop/scripts/coordination.py
```

It adds stable worker IDs, reusable sleeping workers, a durable leased inbox with delivery receipts, an atomic finish boundary, and structured context handoffs. The state lives in `.gigiloop/coordination.json` and is bound to the main checkpoint `run_id`.

```bash
python <skill-path>/scripts/coordination.py init --root . --max-active-workers 2
python <skill-path>/scripts/coordination.py worker-start --root . --id builder-1 --role builder --session-ref "<host-session-id>"
python <skill-path>/scripts/coordination.py send --root . --target builder-1 --message "fix timeout propagation" --dedupe-key timeout-fix
python <skill-path>/scripts/coordination.py claim --root . --id builder-1
python <skill-path>/scripts/coordination.py finish --root . --id builder-1 --receipt "<receipt>" --result "timeout fix complete"
python <skill-path>/scripts/coordination.py handoff --root . --reason compact --summary "implementation complete" --next-action "run verifier"
```

The coordination layer is provider-neutral: it does **not** automate ChatGPT or any other provider UI. A host adapter maps its own subagent/session identifiers to GigiLoop worker records. Message delivery is orchestration state, not verification evidence; the normal verification/fingerprint contract remains authoritative.

See [`gigiloop/references/coordination.md`](gigiloop/references/coordination.md).

---

## Remote control from mobile

GigiLoop can now accept bounded `run` / `resume` commands from a trusted phone or remote client while the coding agent continues on the Mac/workstation.

```text
GitHub mobile / Telegram / dashboard
              │
              ▼
   labeled GitHub Issue command
              │
              ▼
        remote.py daemon
              │
       local allowlist + repo pin
              │
              ▼
   Codex / OpenCode / other host
              │
              ▼
      GigiLoop final gate
```

The phone never supplies shell commands or local paths. The workstation keeps the executable argv, repository locations and actor allowlist in a private local config. Commands are idempotent and completion comes from the GigiLoop checkpoint.

```bash
python <skill-path>/scripts/remote.py validate-config --config ~/.config/gigiloop/remote.json
python <skill-path>/scripts/remote.py daemon --config ~/.config/gigiloop/remote.json
```

See [`gigiloop/references/remote.md`](gigiloop/references/remote.md).

---

## GitHub Actions minutes exhausted? Keep working.

GigiLoop v0.4 explicitly treats hosted CI as **evidence**, not the persistence layer.

When GitHub Actions is exhausted/unavailable:

1. preserve `.gigiloop/checkpoint.json`;
2. run GigiLoop runtime self-test and validator locally;
3. run the project’s tests/lint/typecheck/build/security/integration checks locally;
4. use independent subagent review when available;
5. continue the loop;
6. retry hosted CI when available or when branch protection/release policy truly requires it.

Local development must not freeze merely because GitHub minutes ran out.

If repository policy explicitly requires a hosted status check for merge/release, GigiLoop never fabricates that result: it continues locally and reports the remote gate as `BLOCKED` only when it becomes the actual blocker.

---

## Multi-agent adversarial loop

When the host supports subagents/fresh contexts, GigiLoop maps work into five roles:

| Role | Responsibility | Forbidden shortcut |
|---|---|---|
| **Builder** | implement highest-impact gap | approve its own work |
| **Verifier** | reproduce behavior and run checks | change production code just to obtain green output |
| **Red Team** | attack assumptions, diff, edge cases, integrity | invent findings to satisfy a quota |
| **Judge** | reconcile evidence and choose next action | implement fixes in the same decision pass |
| **Improver** | propose measurable improvements after correctness | expand scope without measurable expected value |

A normal iteration becomes:

```text
Builder → Verifier → Score → Red Team → Judge → Checkpoint
                                              ↓
                                         next action
```

A self-improvement iteration runs only after the primary goal already passes:

```text
Judge → Improver hypothesis → Builder → Verifier → Red Team → Judge
```

If the host has no subagents, GigiLoop keeps the same logical roles sequentially. It does **not** pretend that same-context role-play is independent review.

See [`gigiloop/references/orchestration.md`](gigiloop/references/orchestration.md).

---

## Operating profiles

| Profile | Best for | Quality contract |
|---|---|---|
| **strict** | auth, payments, security, migrations, production incidents, public APIs | regression test + integration checks + independent review when available + full final gate |
| **balanced** | ordinary features, refactors, bug fixes | targeted checks + milestone checks + adversarial review + full relevant final gate |
| **fast** | explicit budget or low-risk exploration | baseline + meaningful reproducible check + reconciliation + strongest final checks allowed by budget |

Default: **balanced**. High-consequence work escalates toward **strict**. GigiLoop never silently downgrades the requested profile.

---

## The core loop

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

## Repository fingerprint and evidence freshness

For Git repositories, the v0.4 fingerprint includes:

- current branch;
- HEAD SHA;
- staged diff;
- unstaged diff;
- untracked path names **and file contents**.

`.git/**` and `.gigiloop/**` are intentionally excluded so writing a checkpoint cannot invalidate its own evidence.

A fingerprint change does not automatically mean “regression”; it means the old evidence may no longer apply. `resume` marks previous current evidence stale and forces re-baselining of affected checks.

---

## Verification integrity

A green check is invalid if the agent changed the rules merely to obtain it.

GigiLoop rejects:

- deleted, skipped, quarantined, or weakened tests;
- reduced assertions or excessive mocking;
- lowered coverage/lint/type/performance/security thresholds;
- disabled workflows, hooks, compiler flags, or strict mode;
- blindly updated snapshots/golden files;
- acceptance criteria changed after implementation;
- hidden/swallowed failures;
- unauthorized destructive Git/data operations;
- overwritten unrelated user work.

See [`gigiloop/references/integrity.md`](gigiloop/references/integrity.md).

---

## Evidence tiers

| Tier | Typical evidence | Practical ceiling |
|---|---|---:|
| **T0** | intuition, unsupported claim, stale evidence | 4/10 |
| **T1** | static inspection only | 6/10 |
| **T2** | lint/typecheck/build/static deterministic checks | 7/10 |
| **T3** | targeted automated tests or reproducible behavior | 8/10 |
| **T4** | T3 + adversarial edge cases + integration/regression verification | 9/10 |
| **T5** | T4 + independent/fresh review + clean final/integrity gates | 10/10 |

Strong confidence never upgrades weak evidence.

See [`gigiloop/references/scoring.md`](gigiloop/references/scoring.md).

---

## Local validator = CI validator

The repository no longer embeds a second validator implementation in the GitHub workflow.

Run exactly what CI runs:

```bash
python gigiloop/scripts/gigiloop.py self-test
python gigiloop/scripts/gigiloop.py validate-repo
python gigiloop/scripts/gigiloop.py pack --output dist/skill.zip
```

Packaging normalizes ZIP timestamps, so an unchanged skill tree should produce byte-identical `skill.zip` files.

CI additionally packages the exact repository revision and uploads both artifacts.

---

## Exit report

Every loop exits explicitly as one of:

- `SUCCESS`
- `BLOCKED`
- `BUDGET EXHAUSTED`
- `STOPPED`

The report distinguishes verified, inferred, unverified, and blocked claims and includes scorecard, commands/checks, integrity state, remaining risks, and next action.

See [`gigiloop/references/reporting.md`](gigiloop/references/reporting.md).

---

## Works across agent hosts

<p align="center">
  <img src="assets/gigiloop-compatibility.jpg" alt="GigiLoop compatibility with OpenCode, Claude Code, Codex, Cursor, and Gemini CLI" width="100%" />
</p>

The canonical skill is host-neutral. Small adapters improve discovery without redefining behavior.

See [`COMPATIBILITY.md`](COMPATIBILITY.md) and [`gigiloop/references/hosts.md`](gigiloop/references/hosts.md).

Compatibility is descriptive and does not imply vendor endorsement.

---

## Benchmarks

GigiLoop does **not** publish invented success rates.

[`benchmarks/README.md`](benchmarks/README.md) defines reproducible comparisons between:

- a normal one-pass coding agent;
- a naive keep-going/Ralph-style loop;
- GigiLoop.

Measured claims belong in public documentation only when raw evidence can reconstruct them.

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
├── adapters/
├── benchmarks/
└── gigiloop/
    ├── SKILL.md
    ├── scripts/
    │   └── gigiloop.py
    ├── agents/
    │   └── openai.yaml
    ├── assets/
    │   └── gigiloop-logo.jpg
    └── references/
        ├── checkpoint.md
        ├── hosts.md
        ├── integrity.md
        ├── orchestration.md
        ├── reporting.md
        ├── runtime.md
        ├── scoring.md
        └── verification.md
```

---

## Contributing

High-value contributions include reproducible failure cases, resume/checkpoint bugs, false positive/negative red-team findings, host adapters, verifier-integrity attacks, supervisor edge cases, and benchmark cases where GigiLoop loses.

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

---

## License

MIT. Third-party names and marks remain property of their respective owners and are referenced only to describe compatibility.

---

⭐ If GigiLoop improves a real result, share the reproducible case. Evidence is more useful than hype.

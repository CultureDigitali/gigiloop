# Changelog

All notable changes to GigiLoop are documented here.

## [Unreleased]

### Added
- Optional provider-neutral `coordination.py` runtime with stable worker identity, reusable sleeping workers, active-worker limits, and persistent host session references.
- Durable inbox with immediate/after-turn priorities, idempotent dedupe keys, leases, replayable receipts, expiry recovery, and atomic acknowledge/next-claim behavior.
- Worker finish boundary that prevents a worker from disappearing while owning an unacknowledged message and can atomically continue with queued follow-up work.
- Structured compact/context-reset handoffs preserving the GigiLoop run identity, checkpoint generation, repository fingerprint, worker snapshot, pending message IDs, summary, and next action.
- Coordination regression suite, self-test, CI compile/self-test coverage, and a dedicated coordination reference guide.

### Changed
- Skill and orchestration guidance now prefers sleeping/reusing compatible workers over needless recreation when the host supports persistent sessions.
- Verification state and orchestration state are explicitly separated: receipts prove delivery ownership, never code correctness.

## v0.5.0

### Added
- First-class `verify` runtime command that executes explicit local checks, records reproducible evidence against the current repository fingerprint, and returns distinct exit codes for pass, failure, and repository drift.
- Cross-process checkpoint mutation lock with stale-lock recovery to serialize heartbeat, checkpoint, resume, supervisor, and evidence updates.
- Dedicated runtime regression suite covering checkpoint races, supervisor budgets, process trees, packaging permissions, symlinks, non-UTF-8 filenames, and verification evidence freshness.

### Changed
- Supervisor processes now run in an isolated process group/session where supported and terminate descendants before restart or exit.
- Deterministic packaging preserves executable permission bits for the runtime and rejects symlinks instead of silently dereferencing them into the skill archive.
- Git command output uses surrogate-escape decoding so unusual filenames cannot crash repository fingerprinting on POSIX systems.
- CI now compiles the runtime, executes the regression suite, validates repository invariants, verifies deterministic packaging, and checks that the packaged CLI remains executable.

### Fixed
- Fixed concurrent atomic writers colliding on the same `.tmp` checkpoint path.
- Fixed lost-update races between heartbeat/checkpoint/supervisor processes.
- Fixed supervisor wall-clock and restart-budget exhaustion leaving the checkpoint incorrectly `active`.
- Fixed terminal `success` being overwritten by a late supervisor budget event.
- Fixed child processes surviving supervisor restarts after the direct parent exits.
- Fixed repository fingerprints dereferencing external symlinks and changing when external target contents changed.
- Fixed non-UTF-8 untracked filenames crashing Git output decoding.
- Fixed deterministic ZIP packaging stripping executable bits from `gigiloop/scripts/gigiloop.py`.
- Fixed verification evidence being treated as current when the verification command itself changed repository state.

## v0.4.0

### Added
- Standard-library local runtime in `gigiloop/scripts/gigiloop.py` with `doctor`, `init`, `status`, `resume`, `heartbeat`, `checkpoint`, `supervise`, `self-test`, `validate-repo`, and deterministic `pack` commands.
- Machine-readable `.gigiloop/checkpoint.json` with atomic writes, run IDs, schema versioning, runtime phase, heartbeat, restart state, and repository fingerprint reconciliation.
- Content-sensitive repository fingerprints that include branch/HEAD, staged/unstaged changes, and untracked file contents while excluding GigiLoop's own runtime state.
- Recovery protocol for context resets, crashes, process exits, stale heartbeats, and externally changed repositories.
- Optional supervisor for restart-safe CLI agents with idle detection, restart budget, backoff, and wall-clock budget.
- Explicit local-first fallback when GitHub Actions or other hosted CI is unavailable, quota-exhausted, delayed, or infrastructure-failing.
- Adversarial multi-agent orchestration contract: Builder, Verifier, Red Team, Judge, and optional Improver.
- Controlled self-improvement pass after primary correctness is stable, with measurable benefit/risk/verification requirements.
- Deterministic skill packaging with normalized ZIP timestamps and byte-for-byte reproducibility checks.

### Changed
- Canonical persistent state moved from prose `.gigiloop/checkpoint.md` to machine-readable `.gigiloop/checkpoint.json`.
- GitHub Actions now calls the same runtime self-test, validator, and packager used locally instead of carrying a second embedded validator implementation.
- Hosted CI is treated as an accelerator/evidence source rather than the sole mechanism for preserving loop progress.
- GigiLoop's canonical control plane now includes explicit idle/crash recovery, CI quota fallback, role separation, and optional self-improvement.
- `.gitignore` now excludes the whole `.gigiloop/` runtime state directory and local `dist/` packages.

### Fixed
- Fixed checkpoint self-invalidation risk by excluding `.gigiloop/**` from repository fingerprints.
- Fixed stale-evidence reuse after external repository changes by marking prior current evidence stale during resume reconciliation.
- Fixed untracked-file blind spots by hashing both untracked path names and file contents.
- Fixed partial-checkpoint corruption risk by writing runtime state atomically.
- Fixed CI/runtime drift by consolidating validation logic into one executable implementation.
- Removed the hardcoded `v0.3.1` success string from validation output.
- Added explicit behavior for exhausted GitHub Actions minutes/quota so autonomous work can continue locally until a truly required remote merge/release gate is reached.

## v0.3.1

### Fixed
- Replaced the simplified vector logo/banner interpretation with the approved rendered GigiLoop logo, superbanner, and compatibility artwork.
- Added an immutable visual manifest with SHA-256 hashes and expected dimensions so approved visuals cannot be silently replaced by a same-named file.
- Connected the approved logo to `gigiloop/agents/openai.yaml` for skill UI display.

### Added
- Strict, balanced, and fast operating profiles with explicit quality contracts.
- Verification-integrity rules that reject deleted/skipped tests, lowered thresholds, disabled checks, blind snapshot updates, and goalpost movement.
- Protected-work safeguards for pre-existing uncommitted changes and unrelated user edits.
- Destructive-operation safeguards for resets, cleaning, force-pushes, migrations, deployments, releases, and bulk deletion.
- Evidence freshness records tied to iteration and repository state.
- Explicit final statuses and a decision-ready completion report contract.
- `integrity.md` and `reporting.md` progressive-disclosure references.

### Changed
- Rebuilt README onboarding around the approved superbanner, compatibility artwork, loop profiles, integrity controls, and one-command installation.
- Expanded checkpoint state to track profile, protected work, verification contract, integrity findings, and stale evidence.
- Strengthened CI to validate skill metadata, references, exact visual hashes, dimensions, and icon paths.

## v0.3

### Added
- Multi-host compatibility guide covering OpenCode, Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot, Cline, OpenHands, Amp, and the wider Agent Skills ecosystem.
- Codex `AGENTS.md`, Gemini CLI `GEMINI.md`, and Cursor `.mdc` convenience adapters.
- Host-portability reference that keeps GigiLoop behavior consistent when subagents, task state, hooks, or persistent state differ by host.
- Refreshed GigiLoop logo and repository hero banner.
- Branding/trademark guidance for third-party compatibility references.

### Changed
- Rebuilt the README around the hooks “One skill. Many agent hosts.” and “The loop that does not trust itself.”
- Made the universal Agent Skills CLI install command the primary onboarding path.
- Expanded repository layout and contribution messaging for host adapters and portability fixes.

## v0.2

### Changed
- Repositioned GigiLoop as verification-first autonomous coding.
- Added repository baseline and pre-existing failure tracking.
- Replaced mandatory three-flaw critique with evidence-gated adversarial review.
- Added independent/fresh-context reviewer preference when available.
- Added mandatory post-critique score reconciliation.
- Added progressive verification instead of full-suite-only iteration policy.
- Added stale-checkpoint detection using repository state.
- Removed hard dependency on OpenCode-specific todo tooling.
- Moved persistent state to `.gigiloop/checkpoint.md` by default.
- Updated installation guidance around the Agent Skills CLI.

### Added
- Evidence-tier scoring reference.
- Canonical checkpoint reference.
- Verification strategy reference.
- Reproducible benchmark protocol.
- Repository contribution and security guidance.
- Skill validation CI.

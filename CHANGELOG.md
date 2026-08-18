# Changelog

All notable changes to GigiLoop are documented here.

## [Unreleased]

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

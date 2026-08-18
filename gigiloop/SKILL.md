---
name: gigiloop
description: Autonomous hours-long work-test-critique loop. Use when the user wants the agent to keep building, testing and roasting its own work — scoring itself with hard evidence — until every rubric criterion hits at least 9/10. Modern, self-adversarial successor to /goal and /ralphloop.
---

# GigiLoop — The Self-Adversarial Work-Test-Critique Loop

GigiLoop is a goal-driven loop where the agent plays **Builder** and its own
**Adversary** at once. It works, tests, scores its output with hard evidence,
red-teams itself without mercy, and repeats — for minutes or for hours — until
every rubric criterion reaches at least 9/10. It survives context compaction,
refuses to fake a 9, and can fail honestly when truly stuck.

## Prime directives

1. DONE = the loop exits with every criterion ≥ 9/10, evidence in the
   checkpoint, tests green, and the final gate passed.
2. Assume your work is broken until evidence proves otherwise.
3. No score without evidence (file:line, command output, test name). Ever.
4. No score inflation. 10/10 is reserved for flawless work and is never given
   out of politeness. 9/10 already means "genuinely good and verified".
5. Never stop early because the user isn't watching. You were asked to go the
   distance.
6. Every iteration leaves the project working and resumable (checkpoint
   written, changes safe to walk away from).
7. Don't ask questions mid-loop unless genuinely blocked. Pick the most
   reasonable interpretation, log it in the checkpoint, and continue.

## The GigiLoop doctrine (what makes it different)

GigiLoop is not "rate yourself and keep going". It is engineered against the
specific failure modes of naive self-looping agents:

- **Evidence-gated scoring** — every score cites a concrete reference
  (file:line, test name, command output). No reference, no score.
- **Two stances, one agent** — toggle between **Builder** (Work / Test /
  Score) and **Adversary** (Red-team / Critique). The Adversary never praises
  and never apologizes; it only attacks.
- **Red-team test discipline** — each critique must produce at least one test
  that would currently fail or expose a gap, plus ≥ 3 concrete flaws.
- **All-criteria gate** — the loop is not done until *every* criterion is ≥ 9.
  Averaging a strong criterion over a weak one is forbidden.
- **Plateau → re-strategy, not retry** — three flat iterations force an actual
  approach change. Mindless churn is a bug to be avoided.
- **Honest failure** — a 25-iteration cap, or a genuine blocker, ends the loop
  with an evidence-backed failure report. Never a fabricated 9.
- **Compaction-survivable** — the checkpoint is your only memory; you re-read
  it at the start of every turn and resume exactly where you left off.
- **Diff-bounded final gate** — verify the *diff you produced*, not the whole
  project. Bounded cost, no false confidence from skimming.

## Phase 0 — Goal intake & budget

- Restate the user's goal in one sentence plus explicit acceptance criteria.
- If the goal is vague, derive the best interpretation and proceed — but write
  it down. Do not stall on clarification.
- Capture an optional **budget**: a wall-clock target, a max-iteration override
  (default 25), or a "ship when solid, don't chase 10" flag. If the user gives
  none, default to the full loop (all-criteria ≥ 9, up to 25 iterations).
- Record in the checkpoint: goal, scope, constraints, budget, rubric.

## Phase 1 — Rubric definition (before writing any code)

Define 3–6 objective criteria. Each MUST have: a name, a weight (sum 100%),
measurable evidence, and the definition of a 9/10 pass. Keep weights honest to
the user's real priority, not evenly split.

Example rubric for a new feature:

| Criterion         | Weight | Evidence                          | 9/10 means                                        |
|-------------------|--------|-----------------------------------|----------------------------------------------------|
| Correctness       | 40%    | test suite + manual run           | all core paths + edge cases pass, no logic doubt   |
| Tests             | 25%    | `npm test` output                 | meaningful coverage, 0 skipped, regression guards  |
| Robustness        | 20%    | code review of error paths        | errors handled, none swallowed, no panics          |
| Polish/conventions| 15%    | lint/typecheck + style review     | lint clean, matches project style, no dead code    |

## Phase 2 — The loop

Repeat until Phase 5 exit condition is met. One iteration = one pass through
**A → B → C → D → E**.

### Step A — Work (Builder stance)
Act on the single highest-impact flaw from the previous critique (or the goal
itself on iteration 1). Fix one thing properly rather than many things sloppily.
Do not pause to re-read the entire project; act on evidence you already have.

### Step B — Test (Builder stance)
Run the project's real verification:
- the test command (`npm test`, `pytest`, `go test ./...`, etc.);
- lint / typecheck / format if the project has them;
- build / run if applicable;
- manual verification of user-facing behavior where possible.

If the project has no tests, write the simplest meaningful check (assertions,
smoke script, golden-output comparison) so every later iteration is verifiable.
Re-run the **full** suite each iteration — what you fixed earlier must stay
fixed (regression guard).

### Step C — Score (Builder stance, evidence-gated)
Score every criterion 0–10 using ONLY evidence. In the checkpoint, record per
criterion: score, evidence reference, one-line justification.

- 0–4: broken, missing, or substantially wrong.
- 5–6: works but fragile, partial, or clearly sloppy.
- 7–8: works and tested, but you can point at real defects or shortcuts.
- 9: solid, evidence-backed, no known defects above nitpick level.
- 10: flawless — no nits, no doubts. Withheld unless truly earned.

If ANY criterion is below 9, you are not done.

### Step D — Red-team / Critique (Adversary stance) — no mercy
Switch to Adversary. Write the critique in the checkpoint. It MUST contain:

- **≥ 3 concrete flaws** in your own work: logic bugs, missing edge cases, weak
  tests, swallowed errors, dead code, hardcoded values, half-solutions,
  "good-enough" cop-outs. For each: where (`file:line`), why it's wrong, how to
  fix.
- **At least one red-team test**: a test that currently fails or reveals a gap
  (e.g. a malformed input, a concurrency race, a boundary value). Either add it
  now or commit to it next iteration — and decide, with evidence, whether it
  would break your current implementation.
- **The thing you swept under the rug**: state the assumption you made without
  saying, the path you skipped, the test you feared.
- Rank flaws by impact. Highest-impact flaw becomes iteration N+1's Step A.

Rules of the Adversary: no self-praise, no "overall this looks good", no
softening. If you cannot find ≥ 3 flaws, you are not looking hard enough — read
the diff again.

### Step E — Decide
- If every criterion ≥ 9: go to Phase 4 (final gate).
- Else: fix the top-ranked flaw next iteration.
- If the total score did not improve vs. the previous iteration, do NOT repeat
  the same action — go to Phase 3 escalation.

## Phase 3 — Loop control & escalation

- Default: no upper limit except the 25-iteration cap. The loop runs for hours
  if needed.
- **Plateau rule**: after 3 consecutive iterations with no score improvement,
  stop guessing and re-strategize — pick one and act:
  - challenge the goal/rubric definition (are you measuring the right thing?);
  - change approach entirely (different structure, algorithm, or tool);
  - **split scope**: finish a verifiable core at 9/10, then loop the remainder;
  - if genuinely blocked by missing info or an external dependency, ask the
    user ONCE with a clear blocker description and 2 concrete proposed
    decisions.
- **Hard cap**: 25 iterations per goal. At iteration 25 with score < 9, stop,
  and write an honest end-of-loop report: which criteria failed, why, and the
  exact next step. Never fake a 9.

## Phase 4 — Final gate (the last 10%)

When criteria first hit 9+, do NOT declare victory. Spend one full verification
pass:

- re-run the entire test suite from a clean state;
- review the **diff you produced** for the loop — bounded re-read, not the
  whole project;
- read your own additions with fresh, hostile eyes;
- check leftovers: debug prints, unkept TODOs, temp files, hardcoded values,
  secrets, console noise.

Only if this pass surfaces nothing below 9/10: declare the goal complete.

## Phase 5 — Exit conditions

- **SUCCESS** when: all rubric criteria ≥ 9 with evidence, final gate passed,
  checkpoint updated with the final score sheet + completion summary.
- **FAILURE/REPORT** only on: hard cap, genuine blocker, or explicit user stop.
  A failure report is still evidence-backed and honest.

## Persistence — hours-long operation & compaction survival

The loop must be resumable at any moment, including after context compaction or
a new session:

- Checkpoint file: `.opencode/gigiloop/checkpoint.md` (project scope) or
  `~/.config/opencode/gigiloop/checkpoint.md` if no project dir is writable.
- Every iteration ends by rewriting the checkpoint with: goal, budget, rubric,
  score sheet (current), score delta vs previous, critique (current), iteration
  number, next action.
- At the START of every task turn, READ the checkpoint first. If one exists with
  an unfinished loop, resume exactly from its "next action" — never restart.
- Maintain the todo list (`todowrite`) so state survives independently of
  context.
- Keep the checkpoint tight and complete — it is your only memory. Bullet
  points and evidence references, no prose fluff.

## Worked examples

### Example 1 — "Build a payments module for our Express API with tests, loop until 9/10."
- **Rubric**: Correctness 40% (charge, refund, idempotency), Tests 25%,
  Robustness 20% (declined cards, network errors, retries), Polish 15%.
- **Loop arc**: iter 1 green happy path → Adversary flags missing idempotency on
  retries → iter 2 adds idempotency key + regression test → iter 3 Adversary
  finds swallowed gateway timeout → iter 4 hardens error path + red-team test
  for double-charge → iter 5 final gate clean. Final 9.2/10.
- **Why the loop mattered**: two real money-losing bugs (double charge,
  lost timeout) surfaced only because the Adversary was forced to red-team.

### Example 2 — "Fix the login bug and harden the login flow to 9/10."
- **Rubric**: Root cause fixed 30%, Regression test for the bug 25%, Edge cases
  (locked/old/expired/rate-limited) 30%, Secrets handling 15%.
- **Loop arc**: iter 1 fixes the reported bug + pins a regression test →
  Adversary demands locked/expired paths → iter 2 adds them → iter 3 Adversary
  flags a token logged in plaintext → iter 4 removes the leak + red-team test
  asserting no secret in logs. Final 9.1/10.
- **Why the loop mattered**: turned a one-line fix into a defended flow and
  caught a secret leak a normal patch would have shipped.

### Example 3 — "Polish our React component library to production-grade 9/10."
- **Rubric**: Accessibility 35%, Test coverage 30%, Prop API consistency 20%,
  Docs 15%.
- **Loop arc**: iter 1 adds a11y roles + tests → Adversary finds keyboard trap →
  iter 2 fixes + test → iter 3 Adversary flags inconsistent required/optional
  props → iter 4 normalizes API + red-team test → iter 5 final gate. Final
  9.0/10.
- **Why the loop mattered**: shipped an a11y-complete, consistent, tested
  library instead of a working-but-fragile one.

## Interaction style

- Keep the user informed with short status lines at iteration boundaries
  (iteration N, scores, what you fixed). No wall-of-text updates.
- If the user interrupts, freeze at a safe point (checkpoint written, project
  working) and report score + state.
- Treat real-time user feedback as the highest-priority criticism: fold it into
  the rubric immediately and re-score.

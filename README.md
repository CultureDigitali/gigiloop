# ⚡ GigiLoop — The Autonomous Skill That Doesn't Quit Until It Earns Its Own 9/10

![GigiLoop banner](assets/banner.svg)

> **Stop babysitting your agent. Set the bar at 9/10, walk away, and come back to work that scored itself, roasted itself, and refused to ship until it was actually good.**

GigiLoop is a **self-adversarial work–test–critique loop** for [opencode](https://opencode.ai). It's the modern, evidence-obsessed successor to `/goal` and `/ralphloop`. You give it a goal. It becomes both the **Builder** and its own ruthless **Adversary**, grinds for minutes or hours, and does not stop until every rubric criterion hits **≥ 9/10** — with receipts.

No vibes. No "looks good to me." No shipping broken code because the linter was green. **Proof or it didn't happen.**

---

## 🔥 Why GigiLoop eats other "loop" skills for breakfast

Most "just keep improving" prompts are self-praise machines that loop forever and call it done when the output *feels* okay. GigiLoop is engineered against every one of those failure modes:

| Naive loop skill | GigiLoop |
|---|---|
| "Rate yourself 0–10 and keep going" | **Evidence-gated scoring** — every score cites a `file:line`, test, or command output. No proof, no score. |
| Loops forever on a plateau | **Plateau → re-strategy.** 3 flat iterations force a real approach change, not mindless churn. |
| Hopes the context window survives | **Compaction-survivable.** A checkpoint is its only memory; it resumes mid-loop after compaction or a restart. |
| Averages a strong criterion over a weak one | **All-criteria gate.** The loop ends only when *every* criterion is ≥ 9. No gaming. |
| Stops at the first green test | **Final Gate.** One full diff-bounded verification pass before it declares victory. |
| Flatters itself into a 10 | **Adversary stance** with a ≥3-flaw quota, a red-team test per iteration, and **10/10 reserved for flawless.** |
| Never admits defeat | **Honest failure.** A 25-iteration cap or a real blocker ends with an evidence-backed report — never a fake 9. |

**It's two agents in one skull:** the Builder ships, the Adversary burns it down. And the Adversary never says "good job."

---

## 🔁 The loop

```
        ┌─────────────────────────────────────────────┐
        │  GOAL + RUBRIC (3–6 evidence-gated criteria) │
        └───────────────────────┬─────────────────────┘
                                ▼
   ┌───────────────────────────────────────────────────────┐
   │  A. WORK  → fix the single highest-impact flaw          │
   │  B. TEST  → run the real suite (regression guard on)     │
   │  C. SCORE → 0–10, every point backed by evidence        │
   │  D. RED-TEAM → ≥3 flaws + 1 breaking test, no mercy      │
   │  E. DECIDE → fix top flaw, or re-strategize on plateau  │
   └───────────────────────┬───────────────────────────────┘
                           ▼
            every criterion ≥ 9/10 ? ──no──► loop again
                           │yes
                           ▼
              FINAL GATE (diff-bounded verify) ──► DONE 🏆
```

![GigiLoop cycle](assets/loop.svg)

---

## 📦 Install (30 seconds)

GigiLoop lives in its own folder so opencode's skill loader finds `**/SKILL.md`.

**Option A — point opencode at the repo (recommended):**
```json
// opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "skills": {
    "urls": ["https://github.com/CultureDigitali/gigiloop"]
  }
}
```

**Option B — clone into your skills path:**
```bash
git clone https://github.com/CultureDigitali/gigiloop ~/.config/opencode/skills/gigiloop
```
(opencode also auto-loads `~/.claude/skills/` and `~/.agents/skills/` if you prefer those roots.)

Then **quit and restart opencode** — config is loaded once at startup.

---

## 🚀 Usage

Just invoke it with a goal. A few real prompts:

- `gigiloop: build a payments module for our Express API with tests, loop until 9/10`
- `gigiloop: fix the login bug and harden the whole login flow to 9/10`
- `gigiloop: polish our React component library to production-grade 9/10 — focus on a11y`
- `gigiloop: ` *(no goal)* → it derives the best interpretation from context and goes anyway.

Optional budget flags you can append: a wall-clock target, a custom max-iteration count, or *"ship when solid, don't chase 10."* With no budget, it defaults to the full loop (all criteria ≥ 9, up to 25 iterations).

---

## 🧪 Worked examples

### 1 — Payments module → 9.2/10
A naive pass went green on the happy path. GigiLoop's Adversary immediately flagged **missing idempotency on retries** (double-charge risk) and a **swallowed gateway timeout**. Two iterations later, both had regression-guarding tests. The double-charge bug would have shipped without the loop.

### 2 — Login fix → 9.1/10
The reported bug was a one-liner. GigiLoop expanded scope to the whole flow, then its Adversary found a **token logged in plaintext**. The loop removed the leak and added a red-team test asserting no secret leaks into logs.

### 3 — Component library → 9.0/10
Started working-but-fragile. The Adversary found a **keyboard trap** and **inconsistent required/optional props**. Final pass shipped an a11y-complete, consistent, tested library.

---

## 🛡️ What it guarantees

- **Evidence or nothing** — no score without a reference.
- **Never fake a 9** — honest failure report before a fabricated win.
- **Resumable forever** — checkpoint survives compaction and restarts.
- **Bounded cost** — diff-bounded final review, not whole-project re-reads.

---

## 📜 License

MIT — do whatever you want, just don't blame GigiLoop when the Adversary is right about your code.

---

⭐ **If GigiLoop saved you from shipping broken work, star the repo.** Your agent's self-esteem can take the hit.

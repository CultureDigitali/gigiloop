# LinkedIn — post draft

**Headline hook:** Your AI coding agent is probably declaring "done" too early. Here's the fix we just open-sourced.

Most autonomous coding loops stop the moment a test turns green. In practice that's where the
real bugs hide — edge cases, swallowed errors, secret leakage, regressions nobody re-checked.

We built GigiLoop, an open-source skill for OpenCode, around one discipline: **proof, not
confidence.**

- Every quality score must cite evidence — a test, command output, or a precise code reference.
- The agent red-teams its own diff and can *lower* its own score when it finds a real flaw.
- It only ships after every acceptance criterion clears the bar, plus a final verification gate.
- If it's stuck, it says so — it never fakes a 9/10.

It also ships a reproducible benchmark protocol so claims stay auditable.

We're using it on our own work now, and the first surprise was how often the "done" state failed
its own red-team pass.

Open source (MIT): https://github.com/CultureDigitali/gigiloop
Install in one line: `npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode`

If you run agentic coding loops, I'd love to hear how you stop the agent from gaming its own
verifier — that's the hard part.

#AIAgents #DevTools #OpenSource #SoftwareEngineering #LLM

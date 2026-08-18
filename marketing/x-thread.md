# X / Twitter — thread draft

1/ Most AI coding loops stop at the first green test and call it done.
We built one that *refuses* to ship until its own work is backed by evidence.
Meet GigiLoop. 🧵

2/ The core rule: evidence-gated scoring.
Every score (0–10) must point to a test, command output, or a precise code reference.
No proof → no score. Intuition caps at 4/10.

3/ It plays two roles in one loop: Builder and Adversary.
The Adversary red-teams the diff every iteration — up to 3 *material* findings.
Crucially: it never fabricates a flaw just to hit a quota.

4/ And confirmed findings can *lower* the score before completion.
An agent that only ever raises its own grade is useless. GigiLoop can mark itself down.

5/ The gate: ALL acceptance criteria must clear the bar.
Averaging a strong criterion over a weak one is forbidden.
Plus a plateau→re-strategy rule so it doesn't churn forever.

6/ Honest exit: a blocker or a budget ends the loop with a report.
It never fakes a 9/10 to look done. That's the whole point.

7/ It even ships a reproducible benchmark protocol:
one-pass vs naive-loop vs GigiLoop, same commit, hidden evaluators, raw evidence kept.
Prove it, don't market fiction.

8/ Open source, MIT.
Repo + install: https://github.com/CultureDigitali/gigiloop
`npx skills add CultureDigitali/gigiloop --skill gigiloop -a opencode`
Feedback on the reconciliation rule welcome. 🚀

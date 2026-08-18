# Reddit — post drafts

Post only one or two of these, spaced out, in the most on-topic subreddits. Don't cross-post
the same text everywhere — that reads as spam and gets removed.

## r/opencode  (most on-topic — self-promo usually tolerated for OSS tooling)
**Title:** I turned /ralphloop into a verification-gated skill for opencode — it refuses to ship without evidence
**Body:**
Shipped GigiLoop after getting tired of autonomous loops that declare victory at the first green test.
Key idea: every score must cite evidence (test / command output / code ref), and a confirmed
flaw can *lower* the score before completion. It also has a plateau→re-strategy rule so it
doesn't churn, and an honest-exit so it never fakes a 9/10.
Repo + install in comments. Curious what this community thinks of the reconciliation rule.

## r/aiagents  (on-topic, active)
**Title:** Self-adversarial coding loop: an agent that red-teams its own diff and reconciles scores
**Body:**
Wrote a skill (OpenCode) where the agent plays Builder and Adversary in one loop. The Adversary
quota is "up to 3 material findings, never fabricated," and confirmed findings can reduce scores.
Includes a reproducible benchmark protocol (one-pass vs naive-loop vs GigiLoop from the same commit).
Honest question for the sub: how do you stop an agent from gaming its own verifier? That's the
hard part I'm still working on.

## r/opensource  (self-promo allowed, but lead with the "why", not the link)
**Title:** Built an open-source autonomous coding skill because I was tired of agents that fake done
**Body:**
Short story + what verification-first means + link. Ask for contributors to the benchmark protocol.

## r/programming  (strict self-promo rules — only if it fits a discussion)
Lead with the idea ("agents that grade their own homework and can lower their grade"), not the repo.
Link only if the thread invites it.

## Avoid
- r/programminghumor, r/technology — wrong fit, will be removed.
- Posting the identical text to 5 subs in one hour — that's the "ridicolo" path. One good post, then wait.

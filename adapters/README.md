# GigiLoop host adapters

The canonical source of truth is [`../gigiloop/SKILL.md`](../gigiloop/SKILL.md).

This directory contains intentionally small host-native wrappers for environments that also use files such as `AGENTS.md`, `GEMINI.md`, or Cursor rules. They are bootstrap aids, not separate implementations of GigiLoop.

If a host can load the Agent Skill directly, prefer the canonical skill so behavior does not drift.
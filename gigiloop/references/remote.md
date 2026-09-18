# Remote Command Layer

GigiLoop Remote lets a trusted mobile or remote client submit bounded coding goals to a Mac or workstation that already has the repositories and coding-agent hosts installed.

The durable transport is **GitHub Issues**. The local controller is `scripts/remote.py`.

## Security model

Remote input is data, never shell code.

- only issues with the `gigiloop-command` label are read;
- the issue author must be in the local `allowed_actors` allowlist;
- the issue target must exist in the local `projects` allowlist;
- the local Git origin may be pinned with `expected_repo`;
- the issue schema accepts only `run` and `resume`;
- unknown fields are rejected;
- `max_iterations` is bounded;
- agent commands come only from local configuration;
- subprocesses use argument arrays with `shell=False`;
- `command_id` is persisted locally for idempotency;
- success is derived from the GigiLoop checkpoint, not from a remote claim.

Do not add a generic remote shell action. Destructive deployment, payment, migration, deletion, secret-management, or other irreversible workflows require their own explicit safety contract.

## Command envelope

Create an issue in the configured control repository with label `gigiloop-command`. Its body is JSON:

```json
{
  "schema": "gigiloop.remote.v1",
  "command_id": "cmd-9e3cc00d-7cb6-4d63-9750-87fe8b8bba22",
  "target": "gigipec",
  "action": "run",
  "goal": "Implement the next approved GIGIpec task and keep iterating until the GigiLoop final gate passes.",
  "profile": "balanced",
  "max_iterations": 25,
  "metadata": {}
}
```

Generate a valid body locally:

```bash
python <skill-path>/scripts/remote.py issue-template \
  --target gigipec \
  --action run \
  --profile balanced \
  --goal "Implement the next approved task and verify it."
```

## Local configuration

Keep the real configuration **outside repositories** because it contains local filesystem paths.

Example:

```json
{
  "schema": "gigiloop.remote-config.v1",
  "control_repo": "CultureDigitali/gigimaster",
  "allowed_actors": ["CultureDigitali"],
  "gigiloop_script": "/Users/me/.agents/skills/gigiloop/scripts/gigiloop.py",
  "state_file": "/Users/me/.local/state/gigiloop/remote-state.json",
  "poll_seconds": 30,
  "projects": {
    "gigipec": {
      "path": "/Users/me/Projects/gigipec",
      "expected_repo": "CultureDigitali/gigipec",
      "agent_command": ["opencode", "run", "--prompt", "{prompt}"]
    }
  }
}
```

Available placeholders inside the local `agent_command` argv array:

- `{prompt}`
- `{goal}`
- `{profile}`
- `{max_iterations}`
- `{target}`
- `{command_id}`

A placeholder remains part of a single argv token. It is never interpolated into a shell string.

## Run modes

Validate configuration:

```bash
python <skill-path>/scripts/remote.py validate-config --config ~/.config/gigiloop/remote.json
```

Process pending commands once:

```bash
python <skill-path>/scripts/remote.py once --config ~/.config/gigiloop/remote.json
```

Run continuously:

```bash
python <skill-path>/scripts/remote.py daemon --config ~/.config/gigiloop/remote.json
```

The controller comments status transitions back onto the issue:

```text
ACCEPTED -> RUNNING -> SUCCESS
                    -> BLOCKED
                    -> BUDGET_EXHAUSTED
                    -> STOPPED
                    -> FAILED
```

Terminal issues are closed automatically. An incomplete run remains open so it can be inspected or resumed.

## Relationship with coordination

Remote transport and durable worker coordination solve different problems:

- `remote.py`: **how a trusted remote command reaches the workstation**;
- `coordination.py`: **how workers, leases, receipts, and context handoffs survive locally**;
- `gigiloop.py`: **whether the engineering result is actually verified**.

A remote command may start an agent that then initializes or resumes `coordination.py`, but remote delivery itself is never verification evidence.

## Mobile clients

Any authenticated client that can create a correctly labeled GitHub issue can act as a frontend: GitHub mobile, a Telegram bot, a private dashboard, or an internal API.

Prefer frontends that only emit the strict envelope above. Do not expose arbitrary local paths or executable commands to the mobile client.

## Prerequisites

The workstation needs:

- Python 3;
- Git;
- GitHub CLI `gh`, already authenticated to the control repository;
- the selected coding-agent executable;
- the relevant repositories already cloned;
- a local configuration file with explicit project mappings.

The remote layer does not bypass provider quotas, host permissions, repository policy, or GigiLoop's completion gate.

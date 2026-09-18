#!/usr/bin/env python3
"""GigiLoop Remote Command Layer.

GitHub Issues is used as a durable command journal. Issue content is data only:
agent commands are defined locally and subprocesses always run with shell=False.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import uuid

SCHEMA = "gigiloop.remote.v1"
CONFIG_SCHEMA = "gigiloop.remote-config.v1"
STATE_SCHEMA = "gigiloop.remote-state.v1"
CONTROL_LABEL = "gigiloop-command"
PROFILES = {"strict", "balanced", "fast"}
ACTIONS = {"run", "resume"}
TERMINAL = {"success", "blocked", "budget_exhausted", "stopped"}
TARGET_RE = re.compile(r"^[A-Za-z0-9._-]{1,80}$")
COMMAND_ID_RE = re.compile(r"^[A-Za-z0-9._:-]{8,160}$")


def now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def atomic_write(path: Path, data: dict) -> None:
    path = path.expanduser().resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    tmp = Path(tmp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(data, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, path)
    finally:
        try:
            tmp.unlink()
        except FileNotFoundError:
            pass


def extract_json(body: str) -> dict:
    text = (body or "").strip()
    if text.startswith("```"):
        lines = text.splitlines()
        if len(lines) >= 3 and lines[-1].strip() == "```":
            lines = lines[1:-1]
            if lines and lines[0].strip().lower() == "json":
                lines = lines[1:]
            text = "\n".join(lines).strip()
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid command JSON: {exc.msg}") from exc
    if not isinstance(value, dict):
        raise ValueError("command must be a JSON object")
    return value


def validate_envelope(raw: dict) -> dict:
    allowed = {"schema", "command_id", "target", "action", "goal", "profile", "max_iterations", "metadata"}
    unknown = sorted(set(raw) - allowed)
    if unknown:
        raise ValueError("unknown command fields: " + ", ".join(unknown))
    if raw.get("schema") != SCHEMA:
        raise ValueError(f"schema must be {SCHEMA}")
    command_id = raw.get("command_id")
    if not isinstance(command_id, str) or not COMMAND_ID_RE.fullmatch(command_id):
        raise ValueError("invalid command_id")
    target = raw.get("target")
    if not isinstance(target, str) or not TARGET_RE.fullmatch(target):
        raise ValueError("invalid target")
    action = raw.get("action")
    if action not in ACTIONS:
        raise ValueError("invalid action")
    goal = raw.get("goal")
    if not isinstance(goal, str) or not goal.strip() or len(goal) > 12000:
        raise ValueError("goal must be non-empty and <= 12000 chars")
    profile = raw.get("profile", "balanced")
    if profile not in PROFILES:
        raise ValueError("invalid profile")
    maximum = raw.get("max_iterations", 25)
    if not isinstance(maximum, int) or isinstance(maximum, bool) or not 1 <= maximum <= 100:
        raise ValueError("max_iterations must be 1..100")
    metadata = raw.get("metadata") or {}
    if not isinstance(metadata, dict):
        raise ValueError("metadata must be an object")
    return {
        "schema": SCHEMA, "command_id": command_id, "target": target, "action": action,
        "goal": goal.strip(), "profile": profile, "max_iterations": maximum, "metadata": metadata,
    }


def load_json(path: Path) -> dict:
    try:
        data = json.loads(path.expanduser().read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"Invalid JSON {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise SystemExit(f"JSON root must be object: {path}")
    return data


def validate_config(raw: dict) -> dict:
    if raw.get("schema") != CONFIG_SCHEMA:
        raise ValueError(f"config schema must be {CONFIG_SCHEMA}")
    repo = raw.get("control_repo")
    if not isinstance(repo, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
        raise ValueError("control_repo must be owner/repo")
    actors = raw.get("allowed_actors")
    if not isinstance(actors, list) or not actors or not all(isinstance(x, str) and x for x in actors):
        raise ValueError("allowed_actors must be a non-empty list")
    script = raw.get("gigiloop_script")
    if not isinstance(script, str) or not script:
        raise ValueError("gigiloop_script is required")
    projects = raw.get("projects")
    if not isinstance(projects, dict) or not projects:
        raise ValueError("projects must be non-empty")
    normalized = {}
    for name, item in projects.items():
        if not isinstance(name, str) or not TARGET_RE.fullmatch(name) or not isinstance(item, dict):
            raise ValueError(f"invalid project {name!r}")
        path, command = item.get("path"), item.get("agent_command")
        if not isinstance(path, str) or not path:
            raise ValueError(f"project {name} path required")
        if not isinstance(command, list) or not command or not all(isinstance(x, str) and x for x in command):
            raise ValueError(f"project {name} agent_command must be argv array")
        expected = item.get("expected_repo")
        if expected is not None and (not isinstance(expected, str) or "/" not in expected):
            raise ValueError(f"project {name} expected_repo invalid")
        normalized[name] = {
            "path": str(Path(path).expanduser().resolve()),
            "agent_command": command,
            "expected_repo": expected,
        }
    poll = raw.get("poll_seconds", 30)
    if not isinstance(poll, (int, float)) or isinstance(poll, bool) or poll < 5:
        raise ValueError("poll_seconds must be >= 5")
    return {
        "schema": CONFIG_SCHEMA, "control_repo": repo, "allowed_actors": sorted(set(actors)),
        "gigiloop_script": str(Path(script).expanduser().resolve()),
        "state_file": str(Path(raw.get("state_file", "~/.local/state/gigiloop/remote-state.json")).expanduser().resolve()),
        "poll_seconds": float(poll), "projects": normalized,
    }


def default_state() -> dict:
    return {"schema": STATE_SCHEMA, "processed": {}, "updated_at": now()}


def load_state(path: Path) -> dict:
    if not path.exists():
        return default_state()
    data = load_json(path)
    if data.get("schema") != STATE_SCHEMA or not isinstance(data.get("processed"), dict):
        raise SystemExit(f"Invalid remote state: {path}")
    return data


def pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False


@contextmanager
def controller_lock(state_path: Path):
    lock = state_path.expanduser().resolve().with_name(state_path.name + ".lock")
    lock.parent.mkdir(parents=True, exist_ok=True)
    token = f"{os.getpid()}:{uuid.uuid4()}"
    for _attempt in range(2):
        try:
            fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                handle.write(token + "\n")
                handle.flush()
                os.fsync(handle.fileno())
            break
        except FileExistsError:
            try:
                existing = lock.read_text(encoding="utf-8").strip()
                pid_text = existing.split(":", 1)[0]
                existing_pid = int(pid_text)
            except (OSError, ValueError):
                existing_pid = -1
            if pid_alive(existing_pid):
                raise RuntimeError(f"another GigiLoop Remote controller is active (pid={existing_pid})")
            try:
                lock.unlink()
            except FileNotFoundError:
                pass
    else:
        raise RuntimeError(f"unable to acquire remote controller lock: {lock}")
    try:
        yield
    finally:
        try:
            if lock.read_text(encoding="utf-8").strip() == token:
                lock.unlink()
        except FileNotFoundError:
            pass


def run(argv: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=str(cwd) if cwd else None, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, errors="surrogateescape", shell=False)


def git_origin(root: Path) -> str | None:
    if not shutil.which("git"):
        return None
    proc = run(["git", "remote", "get-url", "origin"], root)
    return proc.stdout.strip() if proc.returncode == 0 else None


def canonical_github_repo(origin: str | None) -> str | None:
    if not origin:
        return None
    value = origin.strip()
    prefixes = (
        "git@github.com:",
        "ssh://git@github.com/",
        "https://github.com/",
    )
    for prefix in prefixes:
        if value.startswith(prefix):
            path = value[len(prefix):].removesuffix(".git").strip("/")
            parts = path.split("/")
            if len(parts) == 2 and all(re.fullmatch(r"[A-Za-z0-9_.-]+", part) for part in parts):
                return "/".join(parts)
            return None
    return None


def repo_matches(origin: str | None, expected: str | None) -> bool:
    if not expected:
        return True
    return canonical_github_repo(origin) == expected


def checkpoint(root: Path) -> dict | None:
    path = root / ".gigiloop/checkpoint.json"
    if not path.exists():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"status": "invalid"}
    if not isinstance(data, dict):
        return {"status": "invalid"}
    return {key: data.get(key) for key in ("status", "iteration", "run_id", "generation", "next_action")}


def prepare(config: dict, project: dict, envelope: dict) -> tuple[bool, str]:
    root = Path(project["path"])
    cp = root / ".gigiloop/checkpoint.json"
    if envelope["action"] == "resume" or cp.exists():
        proc = run([sys.executable, config["gigiloop_script"], "resume", "--root", str(root)], root)
        if proc.returncode not in {0, 2}:
            return False, (proc.stderr or proc.stdout or "resume failed")[-2000:]
        return True, (proc.stdout or proc.stderr).strip()[-2000:]
    proc = run([
        sys.executable, config["gigiloop_script"], "init", "--root", str(root), "--goal", envelope["goal"],
        "--profile", envelope["profile"], "--max-iterations", str(envelope["max_iterations"]),
    ], root)
    return proc.returncode == 0, (proc.stdout or proc.stderr or "init failed").strip()[-2000:]


def agent_prompt(envelope: dict) -> str:
    verb = "Resume" if envelope["action"] == "resume" else "Execute"
    return (
        f"{verb} this GigiLoop run.\nGoal: {envelope['goal']}\nProfile: {envelope['profile']}\n"
        f"Maximum iterations: {envelope['max_iterations']}\n"
        "Reconcile checkpoint state first. Preserve unrelated work. Record reproducible evidence, run adversarial review, "
        "and declare success only after the GigiLoop final gate passes."
    )


def agent_argv(project: dict, envelope: dict) -> list[str]:
    replacements = {
        "{prompt}": agent_prompt(envelope), "{goal}": envelope["goal"], "{profile}": envelope["profile"],
        "{max_iterations}": str(envelope["max_iterations"]), "{target}": envelope["target"],
        "{command_id}": envelope["command_id"],
    }
    result = []
    for token in project["agent_command"]:
        for key, value in replacements.items():
            token = token.replace(key, value)
        result.append(token)
    return result


def gh(args: list[str]) -> subprocess.CompletedProcess[str]:
    if not shutil.which("gh"):
        return subprocess.CompletedProcess(["gh", *args], 127, "", "gh executable not found")
    return run(["gh", *args])


def gh_json(args: list[str]):
    proc = gh(args)
    if proc.returncode != 0:
        raise RuntimeError((proc.stderr or proc.stdout or "gh failed")[-2000:])
    return json.loads(proc.stdout or "null")


def issues(repo: str) -> list[dict]:
    data = gh_json(["issue", "list", "--repo", repo, "--label", CONTROL_LABEL, "--state", "open", "--limit", "100",
                    "--json", "number,title,body,author,url,createdAt,updatedAt"])
    if not isinstance(data, list):
        raise RuntimeError("unexpected gh issue list response")
    return data


def comment(repo: str, number: int, body: str) -> None:
    proc = gh(["issue", "comment", str(number), "--repo", repo, "--body", body])
    if proc.returncode != 0:
        raise RuntimeError((proc.stderr or proc.stdout or "comment failed")[-2000:])


def close(repo: str, number: int) -> None:
    proc = gh(["issue", "close", str(number), "--repo", repo])
    if proc.returncode != 0:
        raise RuntimeError((proc.stderr or proc.stdout or "close failed")[-2000:])


def status_message(label: str, env: dict, detail: str = "") -> str:
    text = (
        f"<!-- gigiloop-remote:{env['command_id']} -->\n"
        f"**GigiLoop Remote · {label}**\n"
        f"Target: `{env['target']}` · Action: `{env['action']}` · Profile: `{env['profile']}`"
    )
    return text + ("\n\n" + detail[-3000:] if detail else "")


def process(config: dict, state: dict, issue: dict) -> tuple[str, str, dict | None]:
    number = issue.get("number")
    actor = (issue.get("author") or {}).get("login") if isinstance(issue.get("author"), dict) else None
    if not isinstance(number, int):
        return "rejected", "issue number missing", None
    if actor not in config["allowed_actors"]:
        return "rejected", f"author {actor!r} is not allowlisted", None
    try:
        env = validate_envelope(extract_json(issue.get("body") or ""))
    except ValueError as exc:
        return "rejected", str(exc), None
    if env["command_id"] in state["processed"]:
        return "duplicate", state["processed"][env["command_id"]].get("status", "recorded"), env
    project = config["projects"].get(env["target"])
    if not project:
        return "rejected", f"target {env['target']!r} is not configured locally", env
    root = Path(project["path"])
    if not root.is_dir():
        return "failed", f"project path missing: {root}", env
    if not repo_matches(git_origin(root), project.get("expected_repo")):
        return "rejected", "git origin does not match configured repository", env

    state["processed"][env["command_id"]] = {
        "status": "claimed",
        "target": env["target"],
        "issue_number": number,
        "updated_at": now(),
        "detail": "durably claimed before local execution",
    }
    state["updated_at"] = now()
    atomic_write(Path(config["state_file"]), state)

    comment(config["control_repo"], number, status_message("ACCEPTED", env))
    ok, detail = prepare(config, project, env)
    if not ok:
        result = "failed"
    else:
        comment(config["control_repo"], number, status_message("RUNNING", env, detail))
        proc = run(agent_argv(project, env), root)
        cp = checkpoint(root)
        cp_status = cp.get("status") if isinstance(cp, dict) else None
        if cp_status == "success":
            result = "success"
        elif cp_status in TERMINAL:
            result = cp_status
        elif proc.returncode != 0:
            result = "failed"
        else:
            result = "incomplete"
        output = ((proc.stdout or "") + ("\n" if proc.stdout and proc.stderr else "") + (proc.stderr or "")).strip()
        detail = f"agent_exit={proc.returncode}; checkpoint={json.dumps(cp, sort_keys=True)}"
        if output:
            detail += "\n\nOutput tail:\n" + output[-2500:]

    state["processed"][env["command_id"]] = {
        "status": result, "target": env["target"], "issue_number": number, "updated_at": now(), "detail": detail[-2000:]
    }
    state["updated_at"] = now()
    return result, detail, env


def once(config: dict) -> dict:
    state_path = Path(config["state_file"])
    with controller_lock(state_path):
        state = load_state(state_path)
        report = {"seen": 0, "processed": 0, "results": []}
        for issue in sorted(issues(config["control_repo"]), key=lambda x: x.get("number", 0)):
            report["seen"] += 1
            result, detail, env = process(config, state, issue)
            if result != "duplicate":
                report["processed"] += 1
            report["results"].append({"issue": issue.get("number"), "status": result})
            atomic_write(state_path, state)
            number = issue.get("number")
            if isinstance(number, int) and result != "duplicate":
                if env is None:
                    env = {"command_id": f"invalid-{number}", "target": "unknown", "action": "unknown", "profile": "unknown"}
                comment(config["control_repo"], number, status_message(result.upper(), env, detail))
                if result in TERMINAL | {"rejected", "failed"}:
                    close(config["control_repo"], number)
        if not state_path.exists():
            atomic_write(state_path, state)
        return report


def cmd_validate_envelope(a) -> int:
    print(json.dumps(validate_envelope(load_json(Path(a.file))), indent=2, sort_keys=True))
    return 0


def cmd_validate_config(a) -> int:
    print(json.dumps(validate_config(load_json(Path(a.config))), indent=2, sort_keys=True))
    return 0


def cmd_template(a) -> int:
    env = validate_envelope({
        "schema": SCHEMA, "command_id": a.command_id or f"cmd-{uuid.uuid4()}", "target": a.target,
        "action": a.action, "goal": a.goal, "profile": a.profile, "max_iterations": a.max_iterations, "metadata": {},
    })
    print(json.dumps(env, indent=2, sort_keys=True))
    return 0


def cmd_once(a) -> int:
    report = once(validate_config(load_json(Path(a.config))))
    print(json.dumps(report, indent=2, sort_keys=True))
    return 2 if any(x["status"] == "failed" for x in report["results"]) else 0


def cmd_daemon(a) -> int:
    config = validate_config(load_json(Path(a.config)))
    delay = a.poll_seconds or config["poll_seconds"]
    while True:
        try:
            print(json.dumps({"at": now(), **once(config)}, sort_keys=True), flush=True)
        except KeyboardInterrupt:
            return 0
        except Exception as exc:
            print(f"REMOTE_ERROR {type(exc).__name__}: {exc}", file=sys.stderr, flush=True)
        time.sleep(delay)


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="GigiLoop Remote Command Layer")
    s = p.add_subparsers(dest="cmd", required=True)
    q=s.add_parser("validate-envelope"); q.add_argument("--file", required=True); q.set_defaults(fn=cmd_validate_envelope)
    q=s.add_parser("validate-config"); q.add_argument("--config", required=True); q.set_defaults(fn=cmd_validate_config)
    q=s.add_parser("issue-template"); q.add_argument("--target", required=True); q.add_argument("--action", choices=sorted(ACTIONS), default="run"); q.add_argument("--goal", required=True); q.add_argument("--profile", choices=sorted(PROFILES), default="balanced"); q.add_argument("--max-iterations", type=int, default=25); q.add_argument("--command-id"); q.set_defaults(fn=cmd_template)
    q=s.add_parser("once"); q.add_argument("--config", required=True); q.set_defaults(fn=cmd_once)
    q=s.add_parser("daemon"); q.add_argument("--config", required=True); q.add_argument("--poll-seconds", type=float); q.set_defaults(fn=cmd_daemon)
    return p


if __name__ == "__main__":
    args = parser().parse_args()
    try:
        raise SystemExit(int(args.fn(args)))
    except (ValueError, RuntimeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(2)

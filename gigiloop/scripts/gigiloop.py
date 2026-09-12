#!/usr/bin/env python3
"""GigiLoop v0.4 runtime: local validation, resumable checkpoints and supervision.

Standard-library only. Designed to travel inside the GigiLoop skill package.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
import zipfile
from datetime import datetime, timezone
from struct import unpack
from typing import Any, Iterable

SCHEMA_VERSION = 2
TERMINAL_STATUSES = {"success", "blocked", "budget_exhausted", "stopped"}
VALID_STATUSES = {"active", *TERMINAL_STATUSES}
VALID_PROFILES = {"strict", "balanced", "fast"}
CHECKPOINT_REL = Path(".gigiloop/checkpoint.json")
IGNORED_STATE_PREFIXES = (".gigiloop/", ".git/")


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def parse_utc(value: str | None) -> float | None:
    if not value:
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return None


def run(cmd: list[str], cwd: Path, check: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=str(cwd), text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=check)


def is_git_repo(root: Path) -> bool:
    if not shutil.which("git"):
        return False
    return run(["git", "rev-parse", "--is-inside-work-tree"], root).stdout.strip() == "true"


def find_root(start: Path | None = None) -> Path:
    current = (start or Path.cwd()).resolve()
    if current.is_file():
        current = current.parent
    if shutil.which("git"):
        proc = run(["git", "rev-parse", "--show-toplevel"], current)
        if proc.returncode == 0 and proc.stdout.strip():
            return Path(proc.stdout.strip()).resolve()
    probe = current
    while probe != probe.parent:
        if (probe / "gigiloop" / "SKILL.md").exists() or (probe / ".gigiloop").exists():
            return probe
        probe = probe.parent
    return current


def _hash_stream(parts: Iterable[bytes]) -> str:
    digest = hashlib.sha256()
    for part in parts:
        digest.update(len(part).to_bytes(8, "big"))
        digest.update(part)
    return digest.hexdigest()


def _ignored(rel: str) -> bool:
    rel = rel.replace(os.sep, "/")
    return rel == ".git" or rel == ".gigiloop" or rel.startswith(IGNORED_STATE_PREFIXES)


def repository_state(root: Path) -> dict[str, Any]:
    """Return stable state data and a fingerprint that ignores GigiLoop's own state."""
    root = root.resolve()
    if is_git_repo(root):
        branch_proc = run(["git", "branch", "--show-current"], root)
        head_proc = run(["git", "rev-parse", "HEAD"], root)
        head = head_proc.stdout.strip() if head_proc.returncode == 0 else None
        branch = branch_proc.stdout.strip() or None
        unstaged = run(["git", "diff", "--binary", "--no-ext-diff", "--", ".", ":(exclude).gigiloop/**"], root)
        staged = run(["git", "diff", "--cached", "--binary", "--no-ext-diff", "--", ".", ":(exclude).gigiloop/**"], root)
        others = run(["git", "ls-files", "--others", "--exclude-standard", "-z"], root)
        untracked: list[str] = []
        untracked_parts: list[bytes] = []
        if others.returncode == 0:
            for raw in others.stdout.split("\0"):
                if not raw or _ignored(raw):
                    continue
                path = root / raw
                if path.is_file():
                    untracked.append(raw)
                    untracked_parts.append(raw.encode("utf-8", "surrogateescape"))
                    try:
                        untracked_parts.append(path.read_bytes())
                    except OSError:
                        untracked_parts.append(b"<unreadable>")
        parts = [
            b"git-v2",
            (branch or "").encode(),
            (head or "").encode(),
            unstaged.stdout.encode("utf-8", "surrogateescape"),
            staged.stdout.encode("utf-8", "surrogateescape"),
            *untracked_parts,
        ]
        return {
            "mode": "git",
            "branch": branch,
            "head_sha": head,
            "dirty": bool(unstaged.stdout or staged.stdout or untracked),
            "untracked": sorted(untracked),
            "fingerprint": _hash_stream(parts),
        }

    entries: list[bytes] = [b"filesystem-v1"]
    files: list[str] = []
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        rel = path.relative_to(root).as_posix()
        if _ignored(rel):
            continue
        files.append(rel)
        entries.append(rel.encode("utf-8", "surrogateescape"))
        try:
            entries.append(path.read_bytes())
        except OSError:
            entries.append(b"<unreadable>")
    return {
        "mode": "filesystem",
        "branch": None,
        "head_sha": None,
        "dirty": None,
        "untracked": [],
        "files": len(files),
        "fingerprint": _hash_stream(entries),
    }


def atomic_json_write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temp, path)


def load_checkpoint(root: Path) -> dict[str, Any]:
    path = root / CHECKPOINT_REL
    if not path.exists():
        raise SystemExit(f"Checkpoint not found: {path}. Run 'init' first.")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"Invalid checkpoint: {exc}") from exc
    validate_checkpoint(data)
    return data


def validate_checkpoint(data: dict[str, Any]) -> None:
    if data.get("schema_version") != SCHEMA_VERSION:
        raise ValueError(f"Unsupported checkpoint schema: {data.get('schema_version')!r}")
    if data.get("status") not in VALID_STATUSES:
        raise ValueError(f"Invalid checkpoint status: {data.get('status')!r}")
    if data.get("profile") not in VALID_PROFILES:
        raise ValueError(f"Invalid profile: {data.get('profile')!r}")
    if not isinstance(data.get("iteration"), int) or data["iteration"] < 0:
        raise ValueError("iteration must be a non-negative integer")
    if not isinstance(data.get("repository"), dict) or not data["repository"].get("fingerprint"):
        raise ValueError("repository.fingerprint is required")
    if not data.get("run_id"):
        raise ValueError("run_id is required")


def new_checkpoint(root: Path, goal: str, profile: str, max_iterations: int) -> dict[str, Any]:
    state = repository_state(root)
    now = utc_now()
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": str(uuid.uuid4()),
        "generation": 1,
        "status": "active",
        "iteration": 0,
        "profile": profile,
        "goal": goal,
        "scope": {"included": [], "excluded": []},
        "constraints": [],
        "budget": {"max_iterations": max_iterations, "wall_clock": None, "cost_or_token_limit": None},
        "repository": {**state, "protected_local_changes": [], "instructions_read": []},
        "verification_contract": {
            "tests": [], "thresholds": [], "snapshots_or_golden_files": [],
            "static_checks": [], "manual_acceptance": [], "approved_exceptions": []
        },
        "baseline": {"commands": [], "pre_existing_failures": [], "unavailable_checks": []},
        "rubric": [],
        "current_evidence": [],
        "findings": {"confirmed": [], "falsified": [], "hypotheses": []},
        "integrity": {
            "verifier_changes": [], "protected_work_conflicts": [],
            "destructive_operations": [], "integrity_blockers": []
        },
        "progress": {
            "previous_scores": {}, "score_delta": None, "flat_iterations": 0,
            "last_material_change": None
        },
        "runtime": {
            "phase": "intake", "heartbeat_at": now, "last_resume_at": now,
            "resume_requires_rebaseline": False, "supervisor_restarts": 0,
            "last_exit_code": None, "host": os.environ.get("GIGILOOP_HOST")
        },
        "next_action": "establish baseline and verification contract",
        "created_at": now,
        "last_updated": now,
    }


def mark_stale(data: dict[str, Any], reason: str) -> int:
    changed = 0
    for item in data.get("current_evidence", []):
        if item.get("freshness") == "current":
            item["freshness"] = "stale"
            item["stale_reason"] = reason
            changed += 1
    return changed


def reconcile_resume(root: Path, data: dict[str, Any], write: bool = True) -> tuple[bool, dict[str, Any]]:
    current = repository_state(root)
    previous = data.get("repository", {}).get("fingerprint")
    drift = previous != current["fingerprint"]
    now = utc_now()
    runtime = data.setdefault("runtime", {})
    runtime["last_resume_at"] = now
    runtime["heartbeat_at"] = now
    runtime["resume_requires_rebaseline"] = drift
    if drift:
        stale_count = mark_stale(data, "repository state changed since checkpoint")
        data["generation"] = int(data.get("generation", 1)) + 1
        data.setdefault("resume_events", []).append({
            "at": now, "type": "repository_drift", "previous_fingerprint": previous,
            "current_fingerprint": current["fingerprint"], "evidence_marked_stale": stale_count,
        })
    data["repository"] = {**data.get("repository", {}), **current}
    data["last_updated"] = now
    if write:
        atomic_json_write(root / CHECKPOINT_REL, data)
    return drift, data


def jpeg_size(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if data[:2] != b"\xff\xd8":
        raise ValueError(f"{path}: not a JPEG")
    i = 2
    while i < len(data):
        if data[i] != 0xFF:
            i += 1
            continue
        while i < len(data) and data[i] == 0xFF:
            i += 1
        if i >= len(data):
            break
        marker = data[i]
        i += 1
        if marker in (0xD8, 0xD9):
            continue
        if i + 2 > len(data):
            break
        length = unpack(">H", data[i:i + 2])[0]
        if marker in range(0xC0, 0xC4):
            height, width = unpack(">HH", data[i + 3:i + 7])
            return width, height
        i += length
    raise ValueError(f"{path}: JPEG dimensions not found")


def validate_repo(root: Path, skip_assets: bool = False) -> list[str]:
    import re

    errors: list[str] = []
    required = [
        "gigiloop/SKILL.md", "gigiloop/agents/openai.yaml",
        "gigiloop/references/scoring.md", "gigiloop/references/checkpoint.md",
        "gigiloop/references/verification.md", "gigiloop/references/integrity.md",
        "gigiloop/references/reporting.md", "gigiloop/references/hosts.md",
        "gigiloop/references/runtime.md", "gigiloop/scripts/gigiloop.py",
        "COMPATIBILITY.md", "adapters/codex/AGENTS.md", "adapters/gemini-cli/GEMINI.md",
        ".cursor/rules/gigiloop.mdc", "assets/visual-manifest.json", "assets/BRANDING.md",
        "README.md", "CHANGELOG.md",
    ]
    if not skip_assets:
        required += [
            "assets/gigiloop-logo.jpg", "assets/gigiloop-superbanner.jpg",
            "assets/gigiloop-compatibility.jpg", "gigiloop/assets/gigiloop-logo.jpg",
        ]
    for rel in required:
        if not (root / rel).exists():
            errors.append(f"missing required file: {rel}")

    skill_path = root / "gigiloop/SKILL.md"
    if skill_path.exists():
        text = skill_path.read_text(encoding="utf-8")
        match = re.match(r"^---\n(.*?)\n---\n", text, re.S)
        if not match:
            errors.append("invalid SKILL.md frontmatter")
        else:
            fm = match.group(1)
            if not re.search(r"^name:\s+gigiloop\s*$", fm, re.M):
                errors.append("SKILL.md must declare name: gigiloop")
            dm = re.search(r"^description:\s+(.+)$", fm, re.M)
            if not dm:
                errors.append("SKILL.md missing description")
            else:
                desc = dm.group(1).strip()
                if len(desc) > 1024 or "<" in desc or ">" in desc:
                    errors.append("invalid skill description")
        if len(text.splitlines()) > 500:
            errors.append("SKILL.md exceeds 500 lines")
        for ref in ["scoring.md", "checkpoint.md", "verification.md", "integrity.md",
                    "reporting.md", "hosts.md", "runtime.md"]:
            if f"references/{ref}" not in text:
                errors.append(f"SKILL.md does not reference references/{ref}")
        if "scripts/gigiloop.py" not in text:
            errors.append("SKILL.md does not reference scripts/gigiloop.py")

    metadata = root / "gigiloop/agents/openai.yaml"
    if metadata.exists():
        content = metadata.read_text(encoding="utf-8")
        for expected in [
            'display_name: "GigiLoop"', 'icon_small: "./assets/gigiloop-logo.jpg"',
            'icon_large: "./assets/gigiloop-logo.jpg"', 'brand_color: "#7C5CFF"', '$gigiloop'
        ]:
            if expected not in content:
                errors.append(f"openai.yaml missing: {expected}")

    workflow = root / ".github/workflows/validate-skill.yml"
    if workflow.exists():
        wf = workflow.read_text(encoding="utf-8")
        for required_cmd in [
            "python gigiloop/scripts/gigiloop.py self-test",
            "python gigiloop/scripts/gigiloop.py validate-repo",
            "python gigiloop/scripts/gigiloop.py pack",
        ]:
            if required_cmd not in wf:
                errors.append(f"workflow missing local-first command: {required_cmd}")

    if not skip_assets and (root / "assets/visual-manifest.json").exists():
        expected = {
            "assets/gigiloop-superbanner.jpg": ("9042e8b31e7ef514aeb459bcbf2601a791340cf70f3ef1749f83beb9ef50f157", (1200, 400)),
            "assets/gigiloop-compatibility.jpg": ("67f3733445edfd7992cccba30a1eada1463e5a30f8462dc47e0daeae8eb40eae", (1600, 537)),
            "assets/gigiloop-logo.jpg": ("c07eda5c8557983d757ca48a7988fd3d4b2bb1f80ce8937cd6d27dfa071e5af4", (600, 600)),
            "gigiloop/assets/gigiloop-logo.jpg": ("c07eda5c8557983d757ca48a7988fd3d4b2bb1f80ce8937cd6d27dfa071e5af4", (600, 600)),
        }
        try:
            manifest = json.loads((root / "assets/visual-manifest.json").read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            errors.append(f"invalid visual manifest: {exc}")
            manifest = {"assets": {}}
        for rel, (digest, dimensions) in expected.items():
            path = root / rel
            if not path.exists():
                continue
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != digest:
                errors.append(f"{rel}: SHA-256 mismatch")
            try:
                if jpeg_size(path) != dimensions:
                    errors.append(f"{rel}: dimension mismatch")
            except ValueError as exc:
                errors.append(str(exc))
            entry = manifest.get("assets", {}).get(rel)
            if not entry:
                errors.append(f"manifest missing {rel}")
            elif entry.get("sha256") != digest or (entry.get("width"), entry.get("height")) != dimensions:
                errors.append(f"manifest mismatch for {rel}")

    for rel in [".probe", ".github/workflows/probe-assets.yml"]:
        if (root / rel).exists():
            errors.append(f"temporary artifact must not ship: {rel}")
    return errors


def deterministic_zip(source: Path, out: Path) -> None:
    files = sorted(p for p in source.rglob("*") if p.is_file())
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in files:
            rel = path.relative_to(source.parent).as_posix()
            info = zipfile.ZipInfo(rel, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o644 & 0xFFFF) << 16
            archive.writestr(info, path.read_bytes())
    with zipfile.ZipFile(out) as archive:
        bad = archive.testzip()
        if bad:
            raise RuntimeError(f"corrupt ZIP member: {bad}")
    if out.stat().st_size > 25 * 1024 * 1024:
        raise RuntimeError("packaged skill exceeds 25 MB")


def cmd_doctor(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    state = repository_state(root)
    result = {
        "root": str(root),
        "python": sys.version.split()[0],
        "git_available": bool(shutil.which("git")),
        "git_repo": state["mode"] == "git",
        "github_cli_available": bool(shutil.which("gh")),
        "writable": os.access(root, os.W_OK),
        "checkpoint_exists": (root / CHECKPOINT_REL).exists(),
        "repository": state,
        "execution_policy": "local-first; remote CI optional",
    }
    print(json.dumps(result, indent=2) if args.json else "\n".join(f"{k}: {v}" for k, v in result.items()))
    return 0 if result["writable"] else 2


def cmd_init(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    path = root / CHECKPOINT_REL
    if path.exists() and not args.force:
        raise SystemExit(f"Checkpoint already exists: {path}. Use --force only after reviewing it.")
    data = new_checkpoint(root, args.goal, args.profile, args.max_iterations)
    atomic_json_write(path, data)
    print(f"INITIALIZED {path} run_id={data['run_id']} fingerprint={data['repository']['fingerprint'][:12]}")
    return 0


def cmd_status(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    data = load_checkpoint(root)
    current = repository_state(root)
    drift = current["fingerprint"] != data["repository"]["fingerprint"]
    result = {
        "run_id": data["run_id"], "status": data["status"], "iteration": data["iteration"],
        "profile": data["profile"], "goal": data["goal"], "next_action": data.get("next_action"),
        "phase": data.get("runtime", {}).get("phase"), "repository_drift": drift,
        "heartbeat_at": data.get("runtime", {}).get("heartbeat_at"),
    }
    print(json.dumps(result, indent=2) if args.json else " | ".join(f"{k}={v}" for k, v in result.items()))
    return 2 if drift else 0


def cmd_resume(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    data = load_checkpoint(root)
    drift, data = reconcile_resume(root, data, write=True)
    if drift:
        print(f"RESUME_REBASELINE_REQUIRED generation={data['generation']} next={data.get('next_action')}")
        return 2
    print(f"RESUME_OK generation={data['generation']} iteration={data['iteration']} next={data.get('next_action')}")
    return 0


def cmd_heartbeat(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    data = load_checkpoint(root)
    now = utc_now()
    runtime = data.setdefault("runtime", {})
    runtime["heartbeat_at"] = now
    if args.phase:
        runtime["phase"] = args.phase
    if args.message:
        runtime["last_message"] = args.message
    data["last_updated"] = now
    atomic_json_write(root / CHECKPOINT_REL, data)
    print(f"HEARTBEAT {now} phase={runtime.get('phase')}")
    return 0


def cmd_advance(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    data = load_checkpoint(root)
    if args.iteration is not None:
        data["iteration"] = args.iteration
    elif args.increment:
        data["iteration"] += 1
    if data["iteration"] > data.get("budget", {}).get("max_iterations", 25):
        data["status"] = "budget_exhausted"
    if args.status:
        data["status"] = args.status
    if args.next_action is not None:
        data["next_action"] = args.next_action
    if args.phase:
        data.setdefault("runtime", {})["phase"] = args.phase
    now = utc_now()
    data.setdefault("runtime", {})["heartbeat_at"] = now
    data["repository"] = {**data.get("repository", {}), **repository_state(root)}
    data["last_updated"] = now
    atomic_json_write(root / CHECKPOINT_REL, data)
    print(f"CHECKPOINT status={data['status']} iteration={data['iteration']} phase={data['runtime'].get('phase')} next={data.get('next_action')}")
    return 0


def _terminate(proc: subprocess.Popen[Any], grace: float = 5.0) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=grace)


def cmd_supervise(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    if not args.command:
        raise SystemExit("supervise requires a command after '--'")
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise SystemExit("supervise requires a command after '--'")
    started = time.monotonic()
    restarts = 0
    while True:
        data = load_checkpoint(root)
        if data["status"] in TERMINAL_STATUSES:
            print(f"SUPERVISOR terminal status={data['status']}")
            return 0 if data["status"] == "success" else 2
        drift, _ = reconcile_resume(root, data, write=True)
        if drift:
            print("SUPERVISOR repository drift detected; stale evidence marked before launch")
        print(f"SUPERVISOR launch={restarts + 1} command={command!r}", flush=True)
        proc = subprocess.Popen(command, cwd=str(root))
        stale_restart = False
        while proc.poll() is None:
            time.sleep(max(1.0, args.poll_seconds))
            if args.max_wall_seconds and time.monotonic() - started > args.max_wall_seconds:
                _terminate(proc)
                print("SUPERVISOR wall-clock budget exhausted")
                return 3
            try:
                current = load_checkpoint(root)
            except SystemExit as exc:
                _terminate(proc)
                print(f"SUPERVISOR checkpoint failure: {exc}")
                return 4
            if current["status"] in TERMINAL_STATUSES:
                _terminate(proc)
                print(f"SUPERVISOR terminal status={current['status']}")
                return 0 if current["status"] == "success" else 2
            heartbeat = parse_utc(current.get("runtime", {}).get("heartbeat_at"))
            if args.idle_seconds and heartbeat and time.time() - heartbeat > args.idle_seconds:
                stale_restart = True
                print(f"SUPERVISOR stale heartbeat > {args.idle_seconds}s; restarting", flush=True)
                _terminate(proc)
                break
        exit_code = proc.returncode
        data = load_checkpoint(root)
        data.setdefault("runtime", {})["last_exit_code"] = exit_code
        if data["status"] in TERMINAL_STATUSES:
            atomic_json_write(root / CHECKPOINT_REL, data)
            return 0 if data["status"] == "success" else 2
        restarts += 1
        data["runtime"]["supervisor_restarts"] = restarts
        data["runtime"]["heartbeat_at"] = utc_now()
        data["last_updated"] = utc_now()
        atomic_json_write(root / CHECKPOINT_REL, data)
        if restarts > args.max_restarts:
            print(f"SUPERVISOR restart budget exhausted ({args.max_restarts})")
            return 3
        reason = "stale heartbeat" if stale_restart else f"process exit {exit_code}"
        print(f"SUPERVISOR restart {restarts}/{args.max_restarts} after {reason}", flush=True)
        time.sleep(max(0.0, args.backoff_seconds))


def cmd_validate_repo(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    errors = validate_repo(root, skip_assets=args.skip_assets)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        print(f"VALIDATION_FAILED count={len(errors)}", file=sys.stderr)
        return 1
    print("VALIDATION_OK")
    return 0


def cmd_pack(args: argparse.Namespace) -> int:
    root = find_root(Path(args.root) if args.root else None)
    errors = validate_repo(root, skip_assets=args.skip_assets)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    out = Path(args.output).resolve() if args.output else root / "dist/skill.zip"
    deterministic_zip(root / "gigiloop", out)
    print(f"PACKAGED {out} bytes={out.stat().st_size}")
    return 0


def cmd_self_test(_: argparse.Namespace) -> int:
    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="gigiloop-selftest-") as td:
        root = Path(td)
        if shutil.which("git"):
            run(["git", "init"], root, check=True)
            run(["git", "config", "user.email", "selftest@example.invalid"], root, check=True)
            run(["git", "config", "user.name", "GigiLoop Self Test"], root, check=True)
            (root / "app.txt").write_text("v1\n", encoding="utf-8")
            run(["git", "add", "app.txt"], root, check=True)
            run(["git", "commit", "-m", "baseline"], root, check=True)
            baseline = repository_state(root)["fingerprint"]
            (root / ".gigiloop").mkdir()
            (root / ".gigiloop" / "noise.json").write_text("{}\n", encoding="utf-8")
            if repository_state(root)["fingerprint"] != baseline:
                failures.append("internal .gigiloop state changed repository fingerprint")
            (root / "untracked.txt").write_text("one\n", encoding="utf-8")
            first = repository_state(root)["fingerprint"]
            (root / "untracked.txt").write_text("two\n", encoding="utf-8")
            second = repository_state(root)["fingerprint"]
            if first == second:
                failures.append("untracked content change was not fingerprinted")
            (root / "untracked.txt").unlink()
            cp = new_checkpoint(root, "self test", "balanced", 3)
            atomic_json_write(root / CHECKPOINT_REL, cp)
            drift, _ = reconcile_resume(root, load_checkpoint(root), write=True)
            if drift:
                failures.append("clean resume incorrectly detected drift")
            (root / "app.txt").write_text("v2\n", encoding="utf-8")
            drift, resumed = reconcile_resume(root, load_checkpoint(root), write=False)
            if not drift or not resumed.get("runtime", {}).get("resume_requires_rebaseline"):
                failures.append("repository drift was not detected on resume")
        skill = root / "sample-skill"
        skill.mkdir(exist_ok=True)
        (skill / "SKILL.md").write_text("---\nname: sample\ndescription: test\n---\n", encoding="utf-8")
        out1, out2 = root / "a.zip", root / "b.zip"
        deterministic_zip(skill, out1)
        deterministic_zip(skill, out2)
        if hashlib.sha256(out1.read_bytes()).digest() != hashlib.sha256(out2.read_bytes()).digest():
            failures.append("skill packaging is not deterministic")
        try:
            validate_checkpoint({})
            failures.append("invalid checkpoint unexpectedly validated")
        except ValueError:
            pass
    if failures:
        for failure in failures:
            print(f"SELFTEST_FAIL: {failure}", file=sys.stderr)
        return 1
    print("SELFTEST_OK")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="gigiloop.py", description="GigiLoop deterministic runtime and validator")
    sub = parser.add_subparsers(dest="command_name", required=True)

    doctor = sub.add_parser("doctor", help="inspect local capabilities without mutating the repository")
    doctor.add_argument("--root"); doctor.add_argument("--json", action="store_true"); doctor.set_defaults(func=cmd_doctor)

    init = sub.add_parser("init", help="create a machine-readable resumable checkpoint")
    init.add_argument("--root"); init.add_argument("--goal", required=True)
    init.add_argument("--profile", choices=sorted(VALID_PROFILES), default="balanced")
    init.add_argument("--max-iterations", type=int, default=25); init.add_argument("--force", action="store_true")
    init.set_defaults(func=cmd_init)

    status = sub.add_parser("status", help="show checkpoint status and detect repository drift")
    status.add_argument("--root"); status.add_argument("--json", action="store_true"); status.set_defaults(func=cmd_status)

    resume = sub.add_parser("resume", help="reconcile checkpoint with current repository state")
    resume.add_argument("--root"); resume.set_defaults(func=cmd_resume)

    heartbeat = sub.add_parser("heartbeat", help="refresh liveness for an external supervisor")
    heartbeat.add_argument("--root"); heartbeat.add_argument("--phase", choices=["intake", "work", "verify", "score", "review", "reconcile", "final"])
    heartbeat.add_argument("--message"); heartbeat.set_defaults(func=cmd_heartbeat)

    advance = sub.add_parser("checkpoint", help="atomically update iteration/status/next action")
    advance.add_argument("--root"); advance.add_argument("--iteration", type=int); advance.add_argument("--increment", action="store_true")
    advance.add_argument("--status", choices=sorted(VALID_STATUSES)); advance.add_argument("--next-action")
    advance.add_argument("--phase", choices=["intake", "work", "verify", "score", "review", "reconcile", "final"])
    advance.set_defaults(func=cmd_advance)

    supervise = sub.add_parser("supervise", help="restart an agent command after exit or stale heartbeat")
    supervise.add_argument("--root"); supervise.add_argument("--idle-seconds", type=float, default=900)
    supervise.add_argument("--poll-seconds", type=float, default=5); supervise.add_argument("--max-restarts", type=int, default=10)
    supervise.add_argument("--backoff-seconds", type=float, default=3); supervise.add_argument("--max-wall-seconds", type=float, default=0)
    supervise.add_argument("command", nargs=argparse.REMAINDER); supervise.set_defaults(func=cmd_supervise)

    validate = sub.add_parser("validate-repo", help="run the same deterministic repository checks used by CI")
    validate.add_argument("--root"); validate.add_argument("--skip-assets", action="store_true"); validate.set_defaults(func=cmd_validate_repo)

    pack = sub.add_parser("pack", help="validate and build a deterministic skill.zip")
    pack.add_argument("--root"); pack.add_argument("--output"); pack.add_argument("--skip-assets", action="store_true"); pack.set_defaults(func=cmd_pack)

    selftest = sub.add_parser("self-test", help="exercise fingerprint, resume and packaging invariants")
    selftest.set_defaults(func=cmd_self_test)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except ValueError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print("INTERRUPTED", file=sys.stderr)
        return 130


if __name__ == "__main__":
    raise SystemExit(main())

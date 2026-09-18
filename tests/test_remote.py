import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

REMOTE = Path(__file__).resolve().parents[1] / "gigiloop" / "scripts" / "remote.py"
spec = importlib.util.spec_from_file_location("gigiloop_remote", REMOTE)
remote = importlib.util.module_from_spec(spec)
assert spec.loader
spec.loader.exec_module(remote)


class RemoteTests(unittest.TestCase):
    def envelope(self, **changes):
        value = {
            "schema": remote.SCHEMA, "command_id": "cmd-12345678", "target": "gigipec",
            "action": "run", "goal": "fix login regression", "profile": "balanced",
            "max_iterations": 25, "metadata": {},
        }
        value.update(changes)
        return value

    def config(self, root):
        script = root / "gigiloop.py"; script.write_text("print('stub')\n")
        project = root / "project"; project.mkdir()
        return remote.validate_config({
            "schema": remote.CONFIG_SCHEMA, "control_repo": "CultureDigitali/gigimaster",
            "allowed_actors": ["CultureDigitali"], "gigiloop_script": str(script),
            "state_file": str(root / "state.json"), "poll_seconds": 30,
            "projects": {"gigipec": {
                "path": str(project), "expected_repo": "CultureDigitali/gigipec",
                "agent_command": ["opencode", "run", "--prompt", "{prompt}"],
            }},
        })

    def test_rejects_unknown_fields(self):
        with self.assertRaisesRegex(ValueError, "unknown command fields"):
            remote.validate_envelope(self.envelope(shell="rm -rf /"))

    def test_bounds(self):
        with self.assertRaisesRegex(ValueError, "max_iterations"):
            remote.validate_envelope(self.envelope(max_iterations=101))
        with self.assertRaisesRegex(ValueError, "goal"):
            remote.validate_envelope(self.envelope(goal=""))

    def test_json_fence(self):
        body = "```json\n" + json.dumps(self.envelope()) + "\n```"
        self.assertEqual(remote.extract_json(body)["target"], "gigipec")

    def test_command_is_argv_not_shell(self):
        with tempfile.TemporaryDirectory() as td:
            cfg = self.config(Path(td))
            env = remote.validate_envelope(self.envelope(goal="hello; touch /tmp/pwned"))
            argv = remote.agent_argv(cfg["projects"]["gigipec"], env)
            self.assertEqual(argv[:3], ["opencode", "run", "--prompt"])
            self.assertEqual(len(argv), 4)
            self.assertIn("hello; touch /tmp/pwned", argv[3])

    def test_repo_match(self):
        self.assertTrue(remote.repo_matches("https://github.com/CultureDigitali/gigipec.git", "CultureDigitali/gigipec"))
        self.assertTrue(remote.repo_matches("git@github.com:CultureDigitali/gigipec.git", "CultureDigitali/gigipec"))
        self.assertFalse(remote.repo_matches("https://github.com/evil/gigipec.git", "CultureDigitali/gigipec"))
        self.assertFalse(remote.repo_matches("https://evil.example/CultureDigitali/gigipec.git", "CultureDigitali/gigipec"))
        self.assertFalse(remote.repo_matches("https://evil.example/github.com/CultureDigitali/gigipec.git", "CultureDigitali/gigipec"))

    def test_unallowlisted_author_rejected_first(self):
        with tempfile.TemporaryDirectory() as td:
            cfg = self.config(Path(td)); state = remote.default_state()
            issue = {"number": 1, "author": {"login": "mallory"}, "body": json.dumps(self.envelope())}
            result, detail, env = remote.process(cfg, state, issue)
            self.assertEqual(result, "rejected"); self.assertIsNone(env); self.assertIn("not allowlisted", detail)

    def test_unknown_target_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            cfg = self.config(Path(td)); state = remote.default_state()
            issue = {"number": 1, "author": {"login": "CultureDigitali"}, "body": json.dumps(self.envelope(target="other"))}
            result, detail, _ = remote.process(cfg, state, issue)
            self.assertEqual(result, "rejected"); self.assertIn("not configured", detail)

    def test_duplicate_id_is_idempotent(self):
        with tempfile.TemporaryDirectory() as td:
            cfg = self.config(Path(td)); state = remote.default_state()
            state["processed"]["cmd-12345678"] = {"status": "success"}
            issue = {"number": 1, "author": {"login": "CultureDigitali"}, "body": json.dumps(self.envelope())}
            result, detail, _ = remote.process(cfg, state, issue)
            self.assertEqual(result, "duplicate"); self.assertEqual(detail, "success")

    def test_active_controller_lock_rejects_second_daemon(self):
        with tempfile.TemporaryDirectory() as td:
            state = Path(td) / "state.json"
            lock = state.with_name(state.name + ".lock")
            lock.write_text(f"{os.getpid()}:existing\n", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "another GigiLoop Remote controller"):
                with remote.controller_lock(state):
                    pass

    def test_stale_controller_lock_is_recovered(self):
        with tempfile.TemporaryDirectory() as td:
            state = Path(td) / "state.json"
            lock = state.with_name(state.name + ".lock")
            lock.write_text("not-a-pid:stale\n", encoding="utf-8")
            with remote.controller_lock(state):
                self.assertTrue(lock.exists())
            self.assertFalse(lock.exists())

    def test_claim_is_persisted_before_network_status_or_agent_execution(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); cfg = self.config(root); state = remote.default_state()
            issue = {"number": 7, "author": {"login": "CultureDigitali"}, "body": json.dumps(self.envelope())}
            old_origin, old_comment = remote.git_origin, remote.comment
            remote.git_origin = lambda _root: "https://github.com/CultureDigitali/gigipec.git"
            def fail_after_claim(*_args, **_kwargs):
                raise RuntimeError("simulated crash after durable claim")
            remote.comment = fail_after_claim
            try:
                with self.assertRaisesRegex(RuntimeError, "simulated crash"):
                    remote.process(cfg, state, issue)
            finally:
                remote.git_origin, remote.comment = old_origin, old_comment
            persisted = remote.load_state(Path(cfg["state_file"]))
            self.assertEqual(persisted["processed"]["cmd-12345678"]["status"], "claimed")
            self.assertEqual(persisted["processed"]["cmd-12345678"]["issue_number"], 7)

    def test_state_roundtrip(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "state.json"; data = remote.default_state()
            data["processed"]["cmd-1"] = {"status": "success"}; remote.atomic_write(path, data)
            self.assertEqual(remote.load_state(path)["processed"]["cmd-1"]["status"], "success")


if __name__ == "__main__":
    unittest.main(verbosity=2)

import importlib.util
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import zipfile

RUNTIME = Path(__file__).resolve().parents[1] / 'gigiloop' / 'scripts' / 'gigiloop.py'
spec = importlib.util.spec_from_file_location('gigiloop_runtime', RUNTIME)
gl = importlib.util.module_from_spec(spec)
assert spec.loader
spec.loader.exec_module(gl)


def git(root: Path, *args: str) -> None:
    subprocess.run(['git', *args], cwd=root, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


def make_repo(root: Path) -> None:
    git(root, 'init')
    git(root, 'config', 'user.email', 'tests@example.invalid')
    git(root, 'config', 'user.name', 'Tests')
    (root / 'app.txt').write_text('v1\n', encoding='utf-8')
    git(root, 'add', 'app.txt')
    git(root, 'commit', '-m', 'base')


def has_git() -> bool:
    import shutil
    return shutil.which('git') is not None


def process_exists(pid: int) -> bool:
    if os.name == 'posix':
        stat_path = Path(f'/proc/{pid}/stat')
        if stat_path.exists():
            try:
                if stat_path.read_text().split()[2] == 'Z':
                    return False
            except Exception:
                pass
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


class RuntimeRegressionTests(unittest.TestCase):
    def test_atomic_write_allows_concurrent_writers_without_temp_collision(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / 'checkpoint.json'
            barrier = threading.Barrier(2)
            real_replace = gl.os.replace
            errors = []
            def synchronized_replace(src, dst):
                barrier.wait(timeout=5)
                return real_replace(src, dst)
            gl.os.replace = synchronized_replace
            try:
                def writer(value):
                    try:
                        gl.atomic_write(path, {'value': value})
                    except Exception as exc:
                        errors.append(exc)
                threads = [threading.Thread(target=writer, args=(i,)) for i in (1, 2)]
                for t in threads: t.start()
                for t in threads: t.join(timeout=5)
            finally:
                gl.os.replace = real_replace
            self.assertFalse(errors, errors)

    def test_pack_preserves_executable_bit_for_runtime(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); source = root / 'gigiloop'; scripts = source / 'scripts'; scripts.mkdir(parents=True)
            runtime = scripts / 'gigiloop.py'; runtime.write_text('#!/usr/bin/env python3\nprint("ok")\n'); runtime.chmod(0o755)
            out = root / 'skill.zip'; gl.deterministic_zip(source, out)
            with zipfile.ZipFile(out) as z:
                mode = (z.getinfo('gigiloop/scripts/gigiloop.py').external_attr >> 16) & 0o777
            self.assertEqual(mode, 0o755)

    @unittest.skipUnless(hasattr(os, 'symlink'), 'symlink support required')
    def test_repo_fingerprint_does_not_dereference_external_symlink(self):
        if not has_git(): self.skipTest('git unavailable')
        with tempfile.TemporaryDirectory() as repo_td, tempfile.TemporaryDirectory() as ext_td:
            root = Path(repo_td); make_repo(root)
            external = Path(ext_td) / 'outside.txt'; external.write_text('secret-v1')
            os.symlink(external, root / 'outside-link')
            first = gl.repo_state(root)['fingerprint']; external.write_text('secret-v2'); second = gl.repo_state(root)['fingerprint']
            self.assertEqual(first, second)

    def test_supervisor_wall_budget_sets_terminal_checkpoint(self):
        if not has_git(): self.skipTest('git unavailable')
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_repo(root)
            gl.atomic_write(root / gl.CHECKPOINT, gl.fresh_checkpoint(root, 'budget test', 'balanced', 5))
            args = SimpleNamespace(root=str(root), idle_seconds=0, poll_seconds=0.05, max_restarts=1, backoff_seconds=0, max_wall_seconds=0.1,
                                   command=[sys.executable, '-c', 'import time; time.sleep(5)'])
            self.assertEqual(gl.cmd_supervise(args), 3)
            data = gl.load(root)
            self.assertEqual(data['status'], 'budget_exhausted')
            self.assertEqual(data['runtime'].get('supervisor_stop_reason'), 'wall_clock_budget_exhausted')

    @unittest.skipUnless(os.name == 'posix', 'process group test POSIX only')
    def test_supervised_termination_kills_child_process_group(self):
        self.assertTrue(hasattr(gl, 'spawn_supervised'))
        self.assertTrue(hasattr(gl, 'terminate_tree'))
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); pidfile = root / 'child.pid'
            code = ('import pathlib, subprocess, sys, time; '
                    f'p=subprocess.Popen([sys.executable,"-c","import time; time.sleep(30)"]); pathlib.Path({str(pidfile)!r}).write_text(str(p.pid)); time.sleep(30)')
            proc = gl.spawn_supervised([sys.executable, '-c', code], root)
            deadline = time.time()+5
            while not pidfile.exists() and time.time()<deadline: time.sleep(.05)
            child = int(pidfile.read_text())
            gl.terminate_tree(proc, grace=.2)
            deadline = time.time()+3
            while process_exists(child) and time.time()<deadline: time.sleep(.05)
            self.assertFalse(process_exists(child))

    def test_concurrent_checkpoint_updates_are_serialized(self):
        self.assertTrue(hasattr(gl, 'update_checkpoint'))
        if not has_git(): self.skipTest('git unavailable')
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); gl.atomic_write(root/gl.CHECKPOINT, gl.fresh_checkpoint(root,'concurrency','balanced',50)); errors=[]
            def worker():
                try:
                    def mutate(data):
                        rt=data.setdefault('runtime',{}); rt['counter']=rt.get('counter',0)+1
                    gl.update_checkpoint(root, mutate)
                except Exception as exc: errors.append(exc)
            threads=[threading.Thread(target=worker) for _ in range(12)]
            [t.start() for t in threads]; [t.join(timeout=5) for t in threads]
            self.assertFalse(errors, errors); self.assertEqual(gl.load(root)['runtime']['counter'],12)

    def test_checkpoint_validation_rejects_malformed_repository_fingerprint(self):
        with tempfile.TemporaryDirectory() as td:
            data=gl.fresh_checkpoint(Path(td),'x','balanced',2); data['repository']['fingerprint']='bad'
            with self.assertRaisesRegex(ValueError,'fingerprint'): gl.validate_checkpoint(data)

    def test_checkpoint_validation_rejects_invalid_runtime_phase(self):
        with tempfile.TemporaryDirectory() as td:
            data=gl.fresh_checkpoint(Path(td),'x','balanced',2); data['runtime']['phase']='teleport'
            with self.assertRaisesRegex(ValueError,'phase'): gl.validate_checkpoint(data)

    def test_pack_rejects_symlinks(self):
        if not hasattr(os,'symlink'): self.skipTest('symlink unsupported')
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); source=root/'gigiloop'; source.mkdir(); target=root/'target.txt'; target.write_text('x'); os.symlink(target, source/'link.txt')
            with self.assertRaisesRegex(RuntimeError,'symlink'): gl.deterministic_zip(source, root/'skill.zip')

    def test_supervisor_restart_budget_sets_terminal_checkpoint(self):
        if not has_git(): self.skipTest('git unavailable')
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); gl.atomic_write(root/gl.CHECKPOINT, gl.fresh_checkpoint(root,'restart','balanced',5))
            args=SimpleNamespace(root=str(root), idle_seconds=0, poll_seconds=.05,max_restarts=0,backoff_seconds=0,max_wall_seconds=0,command=[sys.executable,'-c','raise SystemExit(7)'])
            self.assertEqual(gl.cmd_supervise(args),3); data=gl.load(root)
            self.assertEqual(data['status'],'budget_exhausted'); self.assertEqual(data['runtime'].get('supervisor_stop_reason'),'restart_budget_exhausted')

    @unittest.skipUnless(os.name=='posix','POSIX only')
    def test_repo_state_handles_non_utf8_untracked_filename(self):
        if not has_git(): self.skipTest('git unavailable')
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); raw=os.path.join(os.fsencode(root),b'bad-\xff.txt'); fd=os.open(raw,os.O_CREAT|os.O_WRONLY,0o600); os.write(fd,b'x'); os.close(fd)
            self.assertEqual(len(gl.repo_state(root)['fingerprint']),64)

    def test_supervisor_stop_does_not_overwrite_existing_success(self):
        if not has_git(): self.skipTest('git unavailable')
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); data=gl.fresh_checkpoint(root,'race','balanced',5); data['status']='success'; gl.atomic_write(root/gl.CHECKPOINT,data)
            gl.supervisor_stop(root,'budget_exhausted','wall_clock_budget_exhausted',0)
            self.assertEqual(gl.load(root)['status'],'success')

    @unittest.skipUnless(os.name=='posix','POSIX only')
    def test_supervisor_cleans_descendants_after_parent_exits(self):
        if not has_git(): self.skipTest('git unavailable')
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); gl.atomic_write(root/gl.CHECKPOINT,gl.fresh_checkpoint(root,'orphan','balanced',5)); pidfile=root/'orphan.pid'
            code=('import pathlib, subprocess, sys; '+f'p=subprocess.Popen([sys.executable,"-c","import time; time.sleep(30)"]); pathlib.Path({str(pidfile)!r}).write_text(str(p.pid))')
            args=SimpleNamespace(root=str(root),idle_seconds=0,poll_seconds=.05,max_restarts=0,backoff_seconds=0,max_wall_seconds=0,command=[sys.executable,'-c',code])
            self.assertEqual(gl.cmd_supervise(args),3); child=int(pidfile.read_text()); deadline=time.time()+2
            while process_exists(child) and time.time()<deadline: time.sleep(.05)
            self.assertFalse(process_exists(child))

    def test_verify_command_records_passing_local_evidence(self):
        self.assertTrue(hasattr(gl,'cmd_verify'))
        if not has_git(): self.skipTest('git unavailable')
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); gl.atomic_write(root/gl.CHECKPOINT,gl.fresh_checkpoint(root,'verify','balanced',5))
            cmd=f'{shlex.quote(sys.executable)} -c {shlex.quote("print(123)")}'
            args=SimpleNamespace(root=str(root),kind='targeted',tier='T3',timeout_seconds=10.0,check=[cmd])
            self.assertEqual(gl.cmd_verify(args),0); ev=gl.load(root)['current_evidence'][-1]
            self.assertEqual(ev['result'],'pass'); self.assertEqual(ev['freshness'],'current'); self.assertEqual(ev['evidence_tier'],'T3')

    def test_verify_command_marks_evidence_stale_if_check_mutates_repo(self):
        self.assertTrue(hasattr(gl,'cmd_verify'))
        if not has_git(): self.skipTest('git unavailable')
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); make_repo(root); gl.atomic_write(root/gl.CHECKPOINT,gl.fresh_checkpoint(root,'verify drift','balanced',5))
            code='from pathlib import Path; Path("app.txt").write_text("v2\\n", encoding="utf-8")'
            cmd=f'{shlex.quote(sys.executable)} -c {shlex.quote(code)}'
            args=SimpleNamespace(root=str(root),kind='targeted',tier='T3',timeout_seconds=10.0,check=[cmd])
            self.assertEqual(gl.cmd_verify(args),2); ev=gl.load(root)['current_evidence'][-1]
            self.assertEqual(ev['freshness'],'stale'); self.assertIn('repository changed',ev['stale_reason'])


if __name__=='__main__': unittest.main(verbosity=2)

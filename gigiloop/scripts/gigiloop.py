#!/usr/bin/env python3
"""GigiLoop v0.5 deterministic local runtime. Python standard library only."""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid
import zipfile
from struct import unpack

SCHEMA_VERSION = 2
CHECKPOINT = Path('.gigiloop/checkpoint.json')
PROFILES = {'strict', 'balanced', 'fast'}
TERMINAL = {'success', 'blocked', 'budget_exhausted', 'stopped'}
STATUSES = {'active', *TERMINAL}
PHASES = {'intake', 'work', 'verify', 'score', 'review', 'reconcile', 'final'}
EVIDENCE_TIERS = {'T1', 'T2', 'T3', 'T4', 'T5'}
VERIFY_KINDS = {'baseline', 'targeted', 'milestone', 'final'}
LOCK_TIMEOUT_SECONDS = 10.0
LOCK_STALE_SECONDS = 60.0


def now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z')


def run(cmd: list[str], cwd: Path, check: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=str(cwd), text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=check, errors='surrogateescape')


def root_of(start: str | None = None) -> Path:
    root = Path(start or '.').resolve()
    if root.is_file():
        root = root.parent
    if shutil.which('git'):
        p = run(['git', 'rev-parse', '--show-toplevel'], root)
        if p.returncode == 0 and p.stdout.strip():
            return Path(p.stdout.strip()).resolve()
    cur = root
    while cur != cur.parent:
        if (cur / 'gigiloop/SKILL.md').exists() or (cur / '.gigiloop').exists():
            return cur
        cur = cur.parent
    return root


def git_repo(root: Path) -> bool:
    return bool(shutil.which('git')) and run(['git', 'rev-parse', '--is-inside-work-tree'], root).stdout.strip() == 'true'


def digest(parts: list[bytes]) -> str:
    h = hashlib.sha256()
    for part in parts:
        h.update(len(part).to_bytes(8, 'big'))
        h.update(part)
    return h.hexdigest()


def ignored(rel: str) -> bool:
    rel = rel.replace(os.sep, '/')
    return rel in {'.git', '.gigiloop'} or rel.startswith('.git/') or rel.startswith('.gigiloop/')


def fingerprint_bytes(path: Path) -> bytes:
    if path.is_symlink():
        return b'symlink-v1\0' + os.fsencode(os.readlink(path))
    return b'file-v1\0' + path.read_bytes()


def repo_state(root: Path) -> dict:
    if git_repo(root):
        branch = run(['git', 'branch', '--show-current'], root).stdout.strip() or None
        hp = run(['git', 'rev-parse', 'HEAD'], root)
        head = hp.stdout.strip() if hp.returncode == 0 else None
        unstaged = run(['git', 'diff', '--binary', '--no-ext-diff', '--', '.', ':(exclude).gigiloop/**'], root).stdout
        staged = run(['git', 'diff', '--cached', '--binary', '--no-ext-diff', '--', '.', ':(exclude).gigiloop/**'], root).stdout
        other = run(['git', 'ls-files', '--others', '--exclude-standard', '-z'], root).stdout
        untracked, parts = [], [b'git-v3', (branch or '').encode(), (head or '').encode(), unstaged.encode(), staged.encode()]
        for rel in other.split('\0'):
            if not rel or ignored(rel):
                continue
            path = root / rel
            if not path.is_symlink() and not path.is_file():
                continue
            untracked.append(rel)
            parts.extend([rel.encode('utf-8', 'surrogateescape'), fingerprint_bytes(path)])
        return {'mode': 'git', 'branch': branch, 'head_sha': head,
                'dirty': bool(unstaged or staged or untracked), 'untracked': sorted(untracked),
                'fingerprint': digest(parts)}
    parts, count = [b'filesystem-v2'], 0
    for path in sorted(p for p in root.rglob('*') if p.is_file() or p.is_symlink()):
        rel = path.relative_to(root).as_posix()
        if ignored(rel):
            continue
        count += 1
        parts.extend([rel.encode('utf-8', 'surrogateescape'), fingerprint_bytes(path)])
    return {'mode': 'filesystem', 'branch': None, 'head_sha': None, 'dirty': None,
            'untracked': [], 'files': count, 'fingerprint': digest(parts)}


def atomic_write(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(data, indent=2, sort_keys=True) + '\n'
    fd, tmp_name = tempfile.mkstemp(prefix=f'.{path.name}.', suffix='.tmp', dir=path.parent)
    tmp = Path(tmp_name)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.chmod(tmp, 0o600)
        except OSError:
            pass
        os.replace(tmp, path)
    finally:
        try:
            tmp.unlink()
        except FileNotFoundError:
            pass


@contextmanager
def checkpoint_lock(root: Path, timeout: float = LOCK_TIMEOUT_SECONDS, stale_after: float = LOCK_STALE_SECONDS):
    lock_path = root / '.gigiloop/checkpoint.lock'
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    deadline, token = time.monotonic() + max(0.0, timeout), f'{os.getpid()}:{uuid.uuid4()}'
    while True:
        try:
            fd = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(fd, 'w', encoding='utf-8') as handle:
                handle.write(token + '\n'); handle.flush(); os.fsync(handle.fileno())
            break
        except FileExistsError:
            try:
                age = time.time() - lock_path.stat().st_mtime
            except FileNotFoundError:
                continue
            if stale_after > 0 and age > stale_after:
                try:
                    lock_path.unlink(); continue
                except FileNotFoundError:
                    continue
            if time.monotonic() >= deadline:
                raise TimeoutError(f'timed out waiting for checkpoint lock: {lock_path}')
            time.sleep(0.025)
    try:
        yield
    finally:
        try:
            if lock_path.read_text(encoding='utf-8').strip() == token:
                lock_path.unlink()
        except FileNotFoundError:
            pass


def validate_checkpoint(data: dict) -> None:
    if data.get('schema_version') != SCHEMA_VERSION:
        raise ValueError('unsupported checkpoint schema')
    if data.get('status') not in STATUSES:
        raise ValueError('invalid checkpoint status')
    if data.get('profile') not in PROFILES:
        raise ValueError('invalid checkpoint profile')
    if not isinstance(data.get('iteration'), int) or data['iteration'] < 0:
        raise ValueError('iteration must be a non-negative integer')
    if not isinstance(data.get('generation'), int) or data['generation'] < 1:
        raise ValueError('generation must be a positive integer')
    if not data.get('run_id'):
        raise ValueError('run_id is required')
    repo = data.get('repository')
    fp = repo.get('fingerprint') if isinstance(repo, dict) else None
    if not isinstance(fp, str) or len(fp) != 64 or any(c not in '0123456789abcdef' for c in fp):
        raise ValueError('repository fingerprint must be a lowercase SHA-256 hex digest')
    budget = data.get('budget')
    if not isinstance(budget, dict) or not isinstance(budget.get('max_iterations'), int) or budget['max_iterations'] < 1:
        raise ValueError('budget.max_iterations must be a positive integer')
    runtime = data.get('runtime')
    if not isinstance(runtime, dict) or runtime.get('phase') not in PHASES:
        raise ValueError('runtime phase is invalid')


def load(root: Path) -> dict:
    path = root / CHECKPOINT
    if not path.exists():
        raise SystemExit(f'Checkpoint not found: {path}. Run init first.')
    try:
        data = json.loads(path.read_text(encoding='utf-8'))
        validate_checkpoint(data)
        return data
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        raise SystemExit(f'Invalid checkpoint: {exc}') from exc


def fresh_checkpoint(root: Path, goal: str, profile: str, max_iterations: int) -> dict:
    ts = now()
    return {
        'schema_version': SCHEMA_VERSION, 'run_id': str(uuid.uuid4()), 'generation': 1,
        'status': 'active', 'iteration': 0, 'profile': profile, 'goal': goal,
        'scope': {'included': [], 'excluded': []}, 'constraints': [],
        'budget': {'max_iterations': max_iterations, 'wall_clock': None, 'cost_or_token_limit': None},
        'repository': {**repo_state(root), 'protected_local_changes': [], 'instructions_read': []},
        'verification_contract': {'tests': [], 'thresholds': [], 'snapshots_or_golden_files': [],
                                  'static_checks': [], 'manual_acceptance': [], 'approved_exceptions': []},
        'baseline': {'commands': [], 'pre_existing_failures': [], 'unavailable_checks': []},
        'rubric': [], 'current_evidence': [],
        'findings': {'confirmed': [], 'falsified': [], 'hypotheses': []},
        'integrity': {'verifier_changes': [], 'protected_work_conflicts': [], 'destructive_operations': [], 'integrity_blockers': []},
        'progress': {'previous_scores': {}, 'score_delta': None, 'flat_iterations': 0, 'last_material_change': None},
        'runtime': {'phase': 'intake', 'heartbeat_at': ts, 'last_resume_at': ts,
                    'resume_requires_rebaseline': False, 'supervisor_restarts': 0,
                    'last_exit_code': None, 'host': os.environ.get('GIGILOOP_HOST')},
        'next_action': 'establish baseline and verification contract', 'created_at': ts, 'last_updated': ts,
    }


def update_checkpoint(root: Path, mutator, *, refresh_repository: bool = False) -> dict:
    with checkpoint_lock(root):
        data = load(root)
        mutator(data)
        if refresh_repository:
            data['repository'] = {**data['repository'], **repo_state(root)}
        data['last_updated'] = now()
        validate_checkpoint(data)
        atomic_write(root / CHECKPOINT, data)
        return data


def reconcile(root: Path, data: dict, write: bool = True) -> bool:
    current, previous = repo_state(root), data['repository']['fingerprint']
    drift, ts = current['fingerprint'] != previous, now()
    runtime = data.setdefault('runtime', {})
    runtime.update({'last_resume_at': ts, 'heartbeat_at': ts, 'resume_requires_rebaseline': drift})
    if drift:
        stale = 0
        for evidence in data.get('current_evidence', []):
            if evidence.get('freshness') == 'current':
                evidence['freshness'] = 'stale'; evidence['stale_reason'] = 'repository state changed since checkpoint'; stale += 1
        data['generation'] = int(data.get('generation', 1)) + 1
        data.setdefault('resume_events', []).append({'at': ts, 'type': 'repository_drift',
            'previous_fingerprint': previous, 'current_fingerprint': current['fingerprint'], 'evidence_marked_stale': stale})
    data['repository'] = {**data['repository'], **current}; data['last_updated'] = ts
    if write:
        atomic_write(root / CHECKPOINT, data)
    return drift


def reconcile_checkpoint(root: Path) -> tuple[bool, dict]:
    result = {'drift': False}
    def mutate(data: dict) -> None:
        result['drift'] = reconcile(root, data, write=False)
    data = update_checkpoint(root, mutate)
    return bool(result['drift']), data


def jpeg_size(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if data[:2] != b'\xff\xd8':
        raise ValueError(f'{path}: not a JPEG')
    i = 2
    while i < len(data):
        if data[i] != 0xFF:
            i += 1; continue
        while i < len(data) and data[i] == 0xFF:
            i += 1
        if i >= len(data): break
        marker = data[i]; i += 1
        if marker in (0xD8, 0xD9): continue
        if i + 2 > len(data): break
        length = unpack('>H', data[i:i+2])[0]
        if marker in range(0xC0, 0xC4):
            height, width = unpack('>HH', data[i+3:i+7]); return width, height
        i += length
    raise ValueError(f'{path}: JPEG dimensions not found')


def validate_repo(root: Path, skip_assets: bool = False) -> list[str]:
    import re
    errors = []
    required = ['gigiloop/SKILL.md', 'gigiloop/agents/openai.yaml', 'gigiloop/scripts/gigiloop.py',
        'gigiloop/references/scoring.md', 'gigiloop/references/checkpoint.md', 'gigiloop/references/verification.md',
        'gigiloop/references/integrity.md', 'gigiloop/references/reporting.md', 'gigiloop/references/hosts.md',
        'gigiloop/references/runtime.md', 'gigiloop/references/orchestration.md', 'COMPATIBILITY.md', 'README.md',
        'CHANGELOG.md', 'assets/visual-manifest.json', 'assets/BRANDING.md', 'adapters/codex/AGENTS.md',
        'adapters/gemini-cli/GEMINI.md', '.cursor/rules/gigiloop.mdc', '.github/workflows/validate-skill.yml',
        'tests/test_runtime.py']
    if not skip_assets:
        required += ['assets/gigiloop-logo.jpg', 'assets/gigiloop-superbanner.jpg', 'assets/gigiloop-compatibility.jpg', 'gigiloop/assets/gigiloop-logo.jpg']
    for rel in required:
        if not (root / rel).exists(): errors.append(f'missing required file: {rel}')
    skill = root / 'gigiloop/SKILL.md'
    if skill.exists():
        text = skill.read_text(encoding='utf-8')
        m = re.match(r'^---\n(.*?)\n---\n', text, re.S)
        if not m: errors.append('invalid SKILL.md frontmatter')
        else:
            fm = m.group(1)
            if not re.search(r'^name:\s+gigiloop\s*$', fm, re.M): errors.append('expected name: gigiloop')
            dm = re.search(r'^description:\s+(.+)$', fm, re.M)
            if not dm or len(dm.group(1).strip()) > 1024: errors.append('invalid skill description')
        if len(text.splitlines()) > 500: errors.append('SKILL.md exceeds 500 lines')
        for ref in ['scoring.md','checkpoint.md','verification.md','integrity.md','reporting.md','hosts.md','runtime.md','orchestration.md']:
            if f'references/{ref}' not in text: errors.append(f'SKILL.md missing reference {ref}')
        if 'scripts/gigiloop.py' not in text: errors.append('SKILL.md missing runtime reference')
    metadata = root / 'gigiloop/agents/openai.yaml'
    if metadata.exists():
        txt = metadata.read_text(encoding='utf-8')
        for expected in ['display_name: "GigiLoop"','icon_small: "./assets/gigiloop-logo.jpg"','icon_large: "./assets/gigiloop-logo.jpg"','brand_color: "#7C5CFF"','$gigiloop']:
            if expected not in txt: errors.append(f'openai.yaml missing: {expected}')
    wf = root / '.github/workflows/validate-skill.yml'
    if wf.exists():
        txt = wf.read_text(encoding='utf-8')
        for cmd in ['python -m unittest discover -s tests -v','python gigiloop/scripts/gigiloop.py self-test','python gigiloop/scripts/gigiloop.py validate-repo','python gigiloop/scripts/gigiloop.py pack']:
            if cmd not in txt: errors.append(f'workflow missing: {cmd}')
    if not skip_assets and (root/'assets/visual-manifest.json').exists():
        expected = {
          'assets/gigiloop-superbanner.jpg': ('9042e8b31e7ef514aeb459bcbf2601a791340cf70f3ef1749f83beb9ef50f157',(1200,400)),
          'assets/gigiloop-compatibility.jpg': ('67f3733445edfd7992cccba30a1eada1463e5a30f8462dc47e0daeae8eb40eae',(1600,537)),
          'assets/gigiloop-logo.jpg': ('c07eda5c8557983d757ca48a7988fd3d4b2bb1f80ce8937cd6d27dfa071e5af4',(600,600)),
          'gigiloop/assets/gigiloop-logo.jpg': ('c07eda5c8557983d757ca48a7988fd3d4b2bb1f80ce8937cd6d27dfa071e5af4',(600,600))}
        try: manifest = json.loads((root/'assets/visual-manifest.json').read_text(encoding='utf-8'))
        except Exception as exc: errors.append(f'invalid visual manifest: {exc}'); manifest={'assets':{}}
        for rel,(sha,dims) in expected.items():
            path=root/rel
            if not path.exists(): continue
            if hashlib.sha256(path.read_bytes()).hexdigest()!=sha: errors.append(f'{rel}: SHA-256 mismatch')
            try:
                if jpeg_size(path)!=dims: errors.append(f'{rel}: dimension mismatch')
            except ValueError as exc: errors.append(str(exc))
            entry=manifest.get('assets',{}).get(rel)
            if not entry or entry.get('sha256')!=sha or (entry.get('width'),entry.get('height'))!=dims: errors.append(f'manifest mismatch for {rel}')
    return errors


def deterministic_zip(source: Path, out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for path in sorted(p for p in source.rglob('*') if p.is_file() or p.is_symlink()):
            if path.is_symlink(): raise RuntimeError(f'skill package must not contain symlinks: {path}')
            info = zipfile.ZipInfo(path.relative_to(source.parent).as_posix(), (1980,1,1,0,0,0))
            info.compress_type = zipfile.ZIP_DEFLATED; info.create_system = 3
            mode = 0o755 if (path.stat().st_mode & 0o111) else 0o644
            info.external_attr = (mode & 0xffff) << 16
            z.writestr(info, path.read_bytes())
    with zipfile.ZipFile(out) as z:
        if z.testzip(): raise RuntimeError('corrupt skill package')
    if out.stat().st_size > 25*1024*1024: raise RuntimeError('skill package exceeds 25 MB')


def cmd_doctor(a) -> int:
    root = root_of(a.root); state = repo_state(root)
    result = {'root': str(root), 'python': sys.version.split()[0], 'git_available': bool(shutil.which('git')),
              'git_repo': state['mode']=='git', 'github_cli_available': bool(shutil.which('gh')),
              'writable': os.access(root, os.W_OK), 'checkpoint_exists': (root/CHECKPOINT).exists(),
              'repository': state, 'execution_policy': 'local-first; remote CI optional'}
    print(json.dumps(result, indent=2) if a.json else '\n'.join(f'{k}: {v}' for k,v in result.items()))
    return 0 if result['writable'] else 2


def cmd_init(a) -> int:
    root=root_of(a.root); path=root/CHECKPOINT
    if a.max_iterations<1: raise SystemExit('--max-iterations must be >= 1')
    with checkpoint_lock(root):
        if path.exists() and not a.force: raise SystemExit(f'Checkpoint already exists: {path}')
        data=fresh_checkpoint(root,a.goal,a.profile,a.max_iterations); atomic_write(path,data)
    print(f"INITIALIZED {path} run_id={data['run_id']}"); return 0


def cmd_status(a) -> int:
    root=root_of(a.root); data=load(root); drift=repo_state(root)['fingerprint']!=data['repository']['fingerprint']
    result={k:data.get(k) for k in ['run_id','status','iteration','profile','goal','next_action']}
    result.update({'phase':data.get('runtime',{}).get('phase'),'repository_drift':drift,'heartbeat_at':data.get('runtime',{}).get('heartbeat_at')})
    print(json.dumps(result,indent=2) if a.json else ' | '.join(f'{k}={v}' for k,v in result.items())); return 2 if drift else 0


def cmd_resume(a) -> int:
    drift,data=reconcile_checkpoint(root_of(a.root))
    print(('RESUME_REBASELINE_REQUIRED' if drift else 'RESUME_OK')+f" generation={data['generation']} iteration={data['iteration']}")
    return 2 if drift else 0


def cmd_heartbeat(a) -> int:
    root=root_of(a.root); ts=now()
    def mutate(data):
        rt=data.setdefault('runtime',{}); rt['heartbeat_at']=ts
        if a.phase: rt['phase']=a.phase
        if a.message: rt['last_message']=a.message
    update_checkpoint(root,mutate); print(f'HEARTBEAT {ts}'); return 0


def cmd_checkpoint(a) -> int:
    root=root_of(a.root)
    if a.iteration is not None and a.iteration<0: raise SystemExit('--iteration must be >= 0')
    ts=now()
    def mutate(data):
        if a.iteration is not None: data['iteration']=a.iteration
        elif a.increment: data['iteration']+=1
        if a.status: data['status']=a.status
        if data['status']=='active' and data['iteration']>=data.get('budget',{}).get('max_iterations',25): data['status']='budget_exhausted'
        if a.next_action is not None: data['next_action']=a.next_action
        rt=data.setdefault('runtime',{}); rt['heartbeat_at']=ts
        if a.phase: rt['phase']=a.phase
    data=update_checkpoint(root,mutate,refresh_repository=True)
    print(f"CHECKPOINT status={data['status']} iteration={data['iteration']}"); return 0


def parse_ts(value: str|None) -> float|None:
    try: return datetime.fromisoformat((value or '').replace('Z','+00:00')).timestamp()
    except ValueError: return None


def spawn_supervised(command: list[str], root: Path) -> subprocess.Popen:
    kwargs={'cwd':str(root)}
    if os.name=='posix': kwargs['start_new_session']=True
    elif os.name=='nt' and hasattr(subprocess,'CREATE_NEW_PROCESS_GROUP'): kwargs['creationflags']=subprocess.CREATE_NEW_PROCESS_GROUP
    return subprocess.Popen(command,**kwargs)


def terminate_tree(proc: subprocess.Popen, grace: float=5) -> None:
    if os.name=='posix':
        pgid=proc.pid
        try: os.killpg(pgid,signal.SIGTERM)
        except ProcessLookupError: return
        deadline=time.monotonic()+max(0.0,grace)
        while time.monotonic()<deadline:
            try: os.killpg(pgid,0)
            except ProcessLookupError: break
            time.sleep(.025)
        else:
            try: os.killpg(pgid,signal.SIGKILL)
            except ProcessLookupError: pass
        if proc.poll() is None:
            try: proc.wait(timeout=max(.1,grace))
            except subprocess.TimeoutExpired: pass
        return
    if proc.poll() is not None: return
    if os.name=='nt' and shutil.which('taskkill'):
        subprocess.run(['taskkill','/PID',str(proc.pid),'/T','/F'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=False)
    else: proc.terminate()
    try: proc.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        proc.kill()
        try: proc.wait(timeout=grace)
        except subprocess.TimeoutExpired: pass


def supervisor_stop(root: Path, status: str, reason: str, exit_code: int|None=None) -> str:
    ts=now()
    def mutate(data):
        if data['status'] in TERMINAL: return
        data['status']=status; rt=data.setdefault('runtime',{}); rt['supervisor_stop_reason']=reason; rt['heartbeat_at']=ts
        if exit_code is not None: rt['last_exit_code']=exit_code
    return update_checkpoint(root,mutate)['status']


def cmd_supervise(a) -> int:
    root=root_of(a.root); command=list(a.command)
    if command and command[0]=='--': command=command[1:]
    if not command: raise SystemExit("supervise requires a command after '--'")
    started=time.monotonic(); restarts=0
    while True:
        data=load(root)
        if data['status'] in TERMINAL: return 0 if data['status']=='success' else 2
        drift,_=reconcile_checkpoint(root)
        if drift: print('SUPERVISOR repository drift detected; rebaseline required',flush=True)
        proc=spawn_supervised(command,root); stale=False
        while proc.poll() is None:
            time.sleep(max(.05,a.poll_seconds))
            if a.max_wall_seconds and time.monotonic()-started>a.max_wall_seconds:
                terminate_tree(proc); status=supervisor_stop(root,'budget_exhausted','wall_clock_budget_exhausted',proc.returncode)
                return 0 if status=='success' else (3 if status=='budget_exhausted' else 2)
            try: current=load(root)
            except SystemExit as exc:
                terminate_tree(proc); print(f'SUPERVISOR checkpoint failure: {exc}',file=sys.stderr); return 4
            if current['status'] in TERMINAL:
                terminate_tree(proc); return 0 if current['status']=='success' else 2
            beat=parse_ts(current.get('runtime',{}).get('heartbeat_at'))
            if a.idle_seconds and beat and time.time()-beat>a.idle_seconds:
                stale=True; terminate_tree(proc); break
        terminate_tree(proc); restarts+=1
        def record(data):
            rt=data.setdefault('runtime',{}); rt['last_exit_code']=proc.returncode; rt['supervisor_restarts']=restarts; rt['heartbeat_at']=now()
        data=update_checkpoint(root,record)
        if data['status'] in TERMINAL: return 0 if data['status']=='success' else 2
        if restarts>a.max_restarts:
            status=supervisor_stop(root,'budget_exhausted','restart_budget_exhausted',proc.returncode)
            return 0 if status=='success' else (3 if status=='budget_exhausted' else 2)
        print(f"SUPERVISOR restart {restarts}/{a.max_restarts} after {'stale heartbeat' if stale else 'process exit'}",flush=True)
        time.sleep(max(0,a.backoff_seconds))


def cmd_verify(a) -> int:
    root=root_of(a.root)
    if a.timeout_seconds<=0: raise SystemExit('--timeout-seconds must be > 0')
    checks=[]
    for raw in a.check or []:
        argv=shlex.split(raw)
        if not argv: raise SystemExit('--check must not be empty')
        checks.append((raw,argv))
    if not checks: raise SystemExit('verify requires at least one --check')
    reconcile_checkpoint(root); initial=repo_state(root); iteration=load(root)['iteration']; records=[]; failed=False; mutated=False
    for raw,argv in checks:
        started=now()
        try:
            proc=subprocess.run(argv,cwd=str(root),text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=a.timeout_seconds,errors='surrogateescape')
            code=proc.returncode; stdout=proc.stdout or ''; stderr=proc.stderr or ''; result='pass' if code==0 else 'fail'
        except subprocess.TimeoutExpired as exc:
            code=124; stdout=exc.stdout or ''; stderr=(exc.stderr or '')+f'\nTIMEOUT after {a.timeout_seconds}s'; result='fail'
        current=repo_state(root); mutated=mutated or current['fingerprint']!=initial['fingerprint']; failed=failed or result=='fail'
        combined=(str(stdout)+('\n' if stdout and stderr else '')+str(stderr)).strip()
        records.append({'id':f'evidence-{uuid.uuid4()}','iteration':iteration,'code_state':current['fingerprint'],'method':'command','command':raw,
            'scope':'local','result':result,'exit_status':code,'evidence_tier':a.tier,'freshness':'current','classification':a.kind,
            'started_at':started,'completed_at':now(),'output_tail':combined[-2000:]})
        print(f'VERIFY {result.upper()} [{a.kind}/{a.tier}] {raw}')
        if combined: print(combined[-4000:])
    final=repo_state(root); mutated=mutated or final['fingerprint']!=initial['fingerprint']
    if mutated:
        for r in records: r['freshness']='stale'; r['stale_reason']='repository changed while verification commands were running'
    def persist(data):
        data.setdefault('current_evidence',[]).extend(records); data['repository']={**data['repository'],**final}
        rt=data.setdefault('runtime',{}); rt['last_verification_at']=now(); rt['last_verification_result']='stale' if mutated else ('fail' if failed else 'pass')
    update_checkpoint(root,persist)
    if mutated: print('VERIFY_REBASELINE_REQUIRED repository changed during verification',file=sys.stderr); return 2
    return 1 if failed else 0


def cmd_validate(a) -> int:
    errors=validate_repo(root_of(a.root),a.skip_assets)
    for e in errors: print('ERROR:',e,file=sys.stderr)
    print('VALIDATION_OK' if not errors else f'VALIDATION_FAILED count={len(errors)}'); return 0 if not errors else 1


def cmd_pack(a) -> int:
    root=root_of(a.root); errors=validate_repo(root,a.skip_assets)
    if errors:
        for e in errors: print('ERROR:',e,file=sys.stderr)
        return 1
    out=Path(a.output).resolve() if a.output else root/'dist/skill.zip'; deterministic_zip(root/'gigiloop',out)
    print(f'PACKAGED {out} bytes={out.stat().st_size}'); return 0


def cmd_self_test(_a) -> int:
    failures=[]
    with tempfile.TemporaryDirectory(prefix='gigiloop-test-') as td:
        root=Path(td)
        if shutil.which('git'):
            run(['git','init'],root,True); run(['git','config','user.email','selftest@example.invalid'],root,True); run(['git','config','user.name','Self Test'],root,True)
            (root/'app.txt').write_text('v1\n'); run(['git','add','app.txt'],root,True); run(['git','commit','-m','base'],root,True)
            baseline=repo_state(root)['fingerprint']; (root/'.gigiloop').mkdir(); (root/'.gigiloop/x').write_text('x')
            if repo_state(root)['fingerprint']!=baseline: failures.append('internal state changed fingerprint')
            (root/'u.txt').write_text('1'); fp1=repo_state(root)['fingerprint']; (root/'u.txt').write_text('2'); fp2=repo_state(root)['fingerprint']
            if fp1==fp2: failures.append('untracked content not fingerprinted')
            (root/'u.txt').unlink(); cp=fresh_checkpoint(root,'test','balanced',1); atomic_write(root/CHECKPOINT,cp)
            if reconcile(root,load(root)): failures.append('clean resume detected drift')
            (root/'app.txt').write_text('v2'); cp=load(root); cp['current_evidence']=[{'freshness':'current'}]
            if not reconcile(root,cp,False) or cp['current_evidence'][0]['freshness']!='stale': failures.append('drift did not stale evidence')
        skill=root/'sample'; skill.mkdir(exist_ok=True); script=skill/'run.py'; script.write_text('#!/usr/bin/env python3\n'); script.chmod(0o755)
        z1,z2=root/'a.zip',root/'b.zip'; deterministic_zip(skill,z1); deterministic_zip(skill,z2)
        if z1.read_bytes()!=z2.read_bytes(): failures.append('package not deterministic')
        with zipfile.ZipFile(z1) as z:
            if ((z.getinfo('sample/run.py').external_attr>>16)&0o777)!=0o755: failures.append('package lost executable bit')
        try: validate_checkpoint({}); failures.append('invalid checkpoint accepted')
        except ValueError: pass
    for f in failures: print('SELFTEST_FAIL:',f,file=sys.stderr)
    print('SELFTEST_OK' if not failures else f'SELFTEST_FAILED count={len(failures)}'); return 0 if not failures else 1


def parser() -> argparse.ArgumentParser:
    p=argparse.ArgumentParser(description='GigiLoop deterministic runtime'); s=p.add_subparsers(dest='cmd',required=True)
    q=s.add_parser('doctor'); q.add_argument('--root'); q.add_argument('--json',action='store_true'); q.set_defaults(fn=cmd_doctor)
    q=s.add_parser('init'); q.add_argument('--root'); q.add_argument('--goal',required=True); q.add_argument('--profile',choices=sorted(PROFILES),default='balanced'); q.add_argument('--max-iterations',type=int,default=25); q.add_argument('--force',action='store_true'); q.set_defaults(fn=cmd_init)
    q=s.add_parser('status'); q.add_argument('--root'); q.add_argument('--json',action='store_true'); q.set_defaults(fn=cmd_status)
    q=s.add_parser('resume'); q.add_argument('--root'); q.set_defaults(fn=cmd_resume)
    q=s.add_parser('heartbeat'); q.add_argument('--root'); q.add_argument('--phase',choices=sorted(PHASES)); q.add_argument('--message'); q.set_defaults(fn=cmd_heartbeat)
    q=s.add_parser('checkpoint'); q.add_argument('--root'); q.add_argument('--iteration',type=int); q.add_argument('--increment',action='store_true'); q.add_argument('--status',choices=sorted(STATUSES)); q.add_argument('--next-action'); q.add_argument('--phase',choices=sorted(PHASES)); q.set_defaults(fn=cmd_checkpoint)
    q=s.add_parser('supervise'); q.add_argument('--root'); q.add_argument('--idle-seconds',type=float,default=900); q.add_argument('--poll-seconds',type=float,default=5); q.add_argument('--max-restarts',type=int,default=10); q.add_argument('--backoff-seconds',type=float,default=3); q.add_argument('--max-wall-seconds',type=float,default=0); q.add_argument('command',nargs=argparse.REMAINDER); q.set_defaults(fn=cmd_supervise)
    q=s.add_parser('verify'); q.add_argument('--root'); q.add_argument('--kind',choices=sorted(VERIFY_KINDS),default='targeted'); q.add_argument('--tier',choices=sorted(EVIDENCE_TIERS),default='T3'); q.add_argument('--timeout-seconds',type=float,default=900); q.add_argument('--check',action='append',required=True); q.set_defaults(fn=cmd_verify)
    q=s.add_parser('validate-repo'); q.add_argument('--root'); q.add_argument('--skip-assets',action='store_true'); q.set_defaults(fn=cmd_validate)
    q=s.add_parser('pack'); q.add_argument('--root'); q.add_argument('--output'); q.add_argument('--skip-assets',action='store_true'); q.set_defaults(fn=cmd_pack)
    q=s.add_parser('self-test'); q.set_defaults(fn=cmd_self_test)
    return p


if __name__=='__main__':
    args=parser().parse_args()
    try: raise SystemExit(int(args.fn(args)))
    except (ValueError, TimeoutError) as exc:
        print('ERROR:',exc,file=sys.stderr); raise SystemExit(2)

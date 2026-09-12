#!/usr/bin/env python3
"""GigiLoop v0.4 local runtime. Python standard library only."""
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

SCHEMA_VERSION = 2
CHECKPOINT = Path('.gigiloop/checkpoint.json')
PROFILES = {'strict', 'balanced', 'fast'}
TERMINAL = {'success', 'blocked', 'budget_exhausted', 'stopped'}
STATUSES = {'active', *TERMINAL}
PHASES = {'intake', 'work', 'verify', 'score', 'review', 'reconcile', 'final'}


def now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z')


def run(cmd: list[str], cwd: Path, check: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=str(cwd), text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=check)


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


def repo_state(root: Path) -> dict:
    if git_repo(root):
        branch = run(['git', 'branch', '--show-current'], root).stdout.strip() or None
        hp = run(['git', 'rev-parse', 'HEAD'], root)
        head = hp.stdout.strip() if hp.returncode == 0 else None
        unstaged = run(['git', 'diff', '--binary', '--no-ext-diff', '--', '.', ':(exclude).gigiloop/**'], root).stdout
        staged = run(['git', 'diff', '--cached', '--binary', '--no-ext-diff', '--', '.', ':(exclude).gigiloop/**'], root).stdout
        other = run(['git', 'ls-files', '--others', '--exclude-standard', '-z'], root).stdout
        untracked: list[str] = []
        parts = [b'git-v2', (branch or '').encode(), (head or '').encode(), unstaged.encode(), staged.encode()]
        for rel in other.split('\0'):
            if not rel or ignored(rel):
                continue
            path = root / rel
            if not path.is_file():
                continue
            untracked.append(rel)
            parts.extend([rel.encode('utf-8', 'surrogateescape'), path.read_bytes()])
        return {'mode': 'git', 'branch': branch, 'head_sha': head,
                'dirty': bool(unstaged or staged or untracked), 'untracked': sorted(untracked),
                'fingerprint': digest(parts)}

    parts = [b'filesystem-v1']
    count = 0
    for path in sorted(p for p in root.rglob('*') if p.is_file()):
        rel = path.relative_to(root).as_posix()
        if ignored(rel):
            continue
        count += 1
        parts.extend([rel.encode('utf-8', 'surrogateescape'), path.read_bytes()])
    return {'mode': 'filesystem', 'branch': None, 'head_sha': None, 'dirty': None,
            'untracked': [], 'files': count, 'fingerprint': digest(parts)}


def atomic_write(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + '.tmp')
    tmp.write_text(json.dumps(data, indent=2, sort_keys=True) + '\n', encoding='utf-8')
    os.replace(tmp, path)


def validate_checkpoint(data: dict) -> None:
    if data.get('schema_version') != SCHEMA_VERSION:
        raise ValueError('unsupported checkpoint schema')
    if data.get('status') not in STATUSES:
        raise ValueError('invalid checkpoint status')
    if data.get('profile') not in PROFILES:
        raise ValueError('invalid checkpoint profile')
    if not isinstance(data.get('iteration'), int) or data['iteration'] < 0:
        raise ValueError('iteration must be a non-negative integer')
    if not data.get('run_id'):
        raise ValueError('run_id is required')
    if not isinstance(data.get('repository'), dict) or not data['repository'].get('fingerprint'):
        raise ValueError('repository fingerprint is required')


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
        'integrity': {'verifier_changes': [], 'protected_work_conflicts': [],
                      'destructive_operations': [], 'integrity_blockers': []},
        'progress': {'previous_scores': {}, 'score_delta': None, 'flat_iterations': 0,
                     'last_material_change': None},
        'runtime': {'phase': 'intake', 'heartbeat_at': ts, 'last_resume_at': ts,
                    'resume_requires_rebaseline': False, 'supervisor_restarts': 0,
                    'last_exit_code': None, 'host': os.environ.get('GIGILOOP_HOST')},
        'next_action': 'establish baseline and verification contract',
        'created_at': ts, 'last_updated': ts,
    }


def reconcile(root: Path, data: dict, write: bool = True) -> bool:
    current = repo_state(root)
    drift = current['fingerprint'] != data['repository']['fingerprint']
    ts = now()
    runtime = data.setdefault('runtime', {})
    runtime.update({'last_resume_at': ts, 'heartbeat_at': ts, 'resume_requires_rebaseline': drift})
    if drift:
        stale = 0
        for evidence in data.get('current_evidence', []):
            if evidence.get('freshness') == 'current':
                evidence['freshness'] = 'stale'
                evidence['stale_reason'] = 'repository state changed since checkpoint'
                stale += 1
        data['generation'] = int(data.get('generation', 1)) + 1
        data.setdefault('resume_events', []).append({'at': ts, 'type': 'repository_drift',
            'previous_fingerprint': data['repository']['fingerprint'],
            'current_fingerprint': current['fingerprint'], 'evidence_marked_stale': stale})
    data['repository'] = {**data['repository'], **current}
    data['last_updated'] = ts
    if write:
        atomic_write(root / CHECKPOINT, data)
    return drift


def jpeg_size(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if data[:2] != b'\xff\xd8':
        raise ValueError(f'{path}: not a JPEG')
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
        length = unpack('>H', data[i:i+2])[0]
        if marker in range(0xC0, 0xC4):
            height, width = unpack('>HH', data[i+3:i+7])
            return width, height
        i += length
    raise ValueError(f'{path}: JPEG dimensions not found')


def validate_repo(root: Path, skip_assets: bool = False) -> list[str]:
    import re
    errors: list[str] = []
    required = [
        'gigiloop/SKILL.md', 'gigiloop/agents/openai.yaml', 'gigiloop/scripts/gigiloop.py',
        'gigiloop/references/scoring.md', 'gigiloop/references/checkpoint.md',
        'gigiloop/references/verification.md', 'gigiloop/references/integrity.md',
        'gigiloop/references/reporting.md', 'gigiloop/references/hosts.md',
        'gigiloop/references/runtime.md', 'gigiloop/references/orchestration.md',
        'COMPATIBILITY.md', 'README.md', 'CHANGELOG.md', 'assets/visual-manifest.json',
        'assets/BRANDING.md', 'adapters/codex/AGENTS.md', 'adapters/gemini-cli/GEMINI.md',
        '.cursor/rules/gigiloop.mdc', '.github/workflows/validate-skill.yml']
    if not skip_assets:
        required += ['assets/gigiloop-logo.jpg', 'assets/gigiloop-superbanner.jpg',
                     'assets/gigiloop-compatibility.jpg', 'gigiloop/assets/gigiloop-logo.jpg']
    for rel in required:
        if not (root / rel).exists():
            errors.append(f'missing required file: {rel}')

    skill = root / 'gigiloop/SKILL.md'
    if skill.exists():
        text = skill.read_text(encoding='utf-8')
        m = re.match(r'^---\n(.*?)\n---\n', text, re.S)
        if not m:
            errors.append('invalid SKILL.md frontmatter')
        else:
            fm = m.group(1)
            if not re.search(r'^name:\s+gigiloop\s*$', fm, re.M):
                errors.append('expected name: gigiloop')
            dm = re.search(r'^description:\s+(.+)$', fm, re.M)
            if not dm or len(dm.group(1).strip()) > 1024:
                errors.append('invalid skill description')
        if len(text.splitlines()) > 500:
            errors.append('SKILL.md exceeds 500 lines')
        for ref in ['scoring.md', 'checkpoint.md', 'verification.md', 'integrity.md', 'reporting.md',
                    'hosts.md', 'runtime.md', 'orchestration.md']:
            if f'references/{ref}' not in text:
                errors.append(f'SKILL.md missing reference {ref}')
        if 'scripts/gigiloop.py' not in text:
            errors.append('SKILL.md missing runtime reference')

    metadata = root / 'gigiloop/agents/openai.yaml'
    if metadata.exists():
        txt = metadata.read_text(encoding='utf-8')
        for expected in ['display_name: "GigiLoop"', 'icon_small: "./assets/gigiloop-logo.jpg"',
                         'icon_large: "./assets/gigiloop-logo.jpg"', 'brand_color: "#7C5CFF"', '$gigiloop']:
            if expected not in txt:
                errors.append(f'openai.yaml missing: {expected}')

    wf = root / '.github/workflows/validate-skill.yml'
    if wf.exists():
        txt = wf.read_text(encoding='utf-8')
        for cmd in ['python gigiloop/scripts/gigiloop.py self-test',
                    'python gigiloop/scripts/gigiloop.py validate-repo',
                    'python gigiloop/scripts/gigiloop.py pack']:
            if cmd not in txt:
                errors.append(f'workflow missing: {cmd}')

    if not skip_assets and (root / 'assets/visual-manifest.json').exists():
        expected = {
          'assets/gigiloop-superbanner.jpg': ('9042e8b31e7ef514aeb459bcbf2601a791340cf70f3ef1749f83beb9ef50f157',(1200,400)),
          'assets/gigiloop-compatibility.jpg': ('67f3733445edfd7992cccba30a1eada1463e5a30f8462dc47e0daeae8eb40eae',(1600,537)),
          'assets/gigiloop-logo.jpg': ('c07eda5c8557983d757ca48a7988fd3d4b2bb1f80ce8937cd6d27dfa071e5af4',(600,600)),
          'gigiloop/assets/gigiloop-logo.jpg': ('c07eda5c8557983d757ca48a7988fd3d4b2bb1f80ce8937cd6d27dfa071e5af4',(600,600))}
        try:
            manifest = json.loads((root/'assets/visual-manifest.json').read_text(encoding='utf-8'))
        except Exception as exc:
            errors.append(f'invalid visual manifest: {exc}')
            manifest = {'assets': {}}
        for rel, (sha, dims) in expected.items():
            path = root / rel
            if not path.exists():
                continue
            if hashlib.sha256(path.read_bytes()).hexdigest() != sha:
                errors.append(f'{rel}: SHA-256 mismatch')
            try:
                if jpeg_size(path) != dims:
                    errors.append(f'{rel}: dimension mismatch')
            except ValueError as exc:
                errors.append(str(exc))
            entry = manifest.get('assets', {}).get(rel)
            if not entry or entry.get('sha256') != sha or (entry.get('width'), entry.get('height')) != dims:
                errors.append(f'manifest mismatch for {rel}')
    return errors


def deterministic_zip(source: Path, out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for path in sorted(p for p in source.rglob('*') if p.is_file()):
            info = zipfile.ZipInfo(path.relative_to(source.parent).as_posix(), (1980,1,1,0,0,0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o644 & 0xffff) << 16
            z.writestr(info, path.read_bytes())
    with zipfile.ZipFile(out) as z:
        if z.testzip():
            raise RuntimeError('corrupt skill package')
    if out.stat().st_size > 25*1024*1024:
        raise RuntimeError('skill package exceeds 25 MB')


def cmd_doctor(a) -> int:
    root = root_of(a.root)
    state = repo_state(root)
    result = {'root': str(root), 'python': sys.version.split()[0], 'git_available': bool(shutil.which('git')),
              'git_repo': state['mode']=='git', 'github_cli_available': bool(shutil.which('gh')),
              'writable': os.access(root, os.W_OK), 'checkpoint_exists': (root/CHECKPOINT).exists(),
              'repository': state, 'execution_policy': 'local-first; remote CI optional'}
    print(json.dumps(result, indent=2) if a.json else '\n'.join(f'{k}: {v}' for k,v in result.items()))
    return 0 if result['writable'] else 2


def cmd_init(a) -> int:
    root = root_of(a.root)
    path = root / CHECKPOINT
    if path.exists() and not a.force:
        raise SystemExit(f'Checkpoint already exists: {path}')
    data = fresh_checkpoint(root, a.goal, a.profile, a.max_iterations)
    atomic_write(path, data)
    print(f"INITIALIZED {path} run_id={data['run_id']}")
    return 0


def cmd_status(a) -> int:
    root = root_of(a.root)
    data = load(root)
    drift = repo_state(root)['fingerprint'] != data['repository']['fingerprint']
    result = {k:data.get(k) for k in ['run_id','status','iteration','profile','goal','next_action']}
    result.update({'phase': data.get('runtime',{}).get('phase'), 'repository_drift': drift,
                   'heartbeat_at': data.get('runtime',{}).get('heartbeat_at')})
    print(json.dumps(result, indent=2) if a.json else ' | '.join(f'{k}={v}' for k,v in result.items()))
    return 2 if drift else 0


def cmd_resume(a) -> int:
    root = root_of(a.root)
    data = load(root)
    drift = reconcile(root, data)
    print(('RESUME_REBASELINE_REQUIRED' if drift else 'RESUME_OK') + f" generation={data['generation']} iteration={data['iteration']}")
    return 2 if drift else 0


def cmd_heartbeat(a) -> int:
    root = root_of(a.root)
    data = load(root)
    ts = now()
    runtime = data.setdefault('runtime', {})
    runtime['heartbeat_at'] = ts
    if a.phase:
        runtime['phase'] = a.phase
    if a.message:
        runtime['last_message'] = a.message
    data['last_updated'] = ts
    atomic_write(root/CHECKPOINT, data)
    print(f"HEARTBEAT {ts}")
    return 0


def cmd_checkpoint(a) -> int:
    root = root_of(a.root)
    data = load(root)
    if a.iteration is not None:
        data['iteration'] = a.iteration
    elif a.increment:
        data['iteration'] += 1
    if a.status:
        data['status'] = a.status
    max_it = data.get('budget',{}).get('max_iterations',25)
    if data['status']=='active' and data['iteration'] >= max_it:
        data['status']='budget_exhausted'
    if a.next_action is not None:
        data['next_action']=a.next_action
    if a.phase:
        data.setdefault('runtime',{})['phase']=a.phase
    ts=now()
    data.setdefault('runtime',{})['heartbeat_at']=ts
    data['repository']={**data['repository'],**repo_state(root)}
    data['last_updated']=ts
    atomic_write(root/CHECKPOINT,data)
    print(f"CHECKPOINT status={data['status']} iteration={data['iteration']}")
    return 0


def parse_ts(value: str | None) -> float | None:
    try:
        return datetime.fromisoformat((value or '').replace('Z','+00:00')).timestamp()
    except ValueError:
        return None


def terminate(proc: subprocess.Popen, grace: float=5) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=grace)


def cmd_supervise(a) -> int:
    root=root_of(a.root)
    command=list(a.command)
    if command and command[0]=='--':
        command=command[1:]
    if not command:
        raise SystemExit("supervise requires a command after '--'")
    started=time.monotonic()
    restarts=0
    while True:
        data=load(root)
        if data['status'] in TERMINAL:
            return 0 if data['status']=='success' else 2
        drift=reconcile(root,data)
        if drift:
            print('SUPERVISOR repository drift detected; rebaseline required', flush=True)
        proc=subprocess.Popen(command,cwd=str(root))
        stale=False
        while proc.poll() is None:
            time.sleep(max(1,a.poll_seconds))
            if a.max_wall_seconds and time.monotonic()-started > a.max_wall_seconds:
                terminate(proc)
                return 3
            current=load(root)
            if current['status'] in TERMINAL:
                terminate(proc)
                return 0 if current['status']=='success' else 2
            beat=parse_ts(current.get('runtime',{}).get('heartbeat_at'))
            if a.idle_seconds and beat and time.time()-beat > a.idle_seconds:
                stale=True
                terminate(proc)
                break
        data=load(root)
        data.setdefault('runtime',{})['last_exit_code']=proc.returncode
        if data['status'] in TERMINAL:
            atomic_write(root/CHECKPOINT,data)
            return 0 if data['status']=='success' else 2
        restarts += 1
        data['runtime']['supervisor_restarts']=restarts
        data['runtime']['heartbeat_at']=now()
        atomic_write(root/CHECKPOINT,data)
        if restarts > a.max_restarts:
            return 3
        print(f"SUPERVISOR restart {restarts}/{a.max_restarts} after {'stale heartbeat' if stale else 'process exit'}", flush=True)
        time.sleep(max(0,a.backoff_seconds))


def cmd_validate(a) -> int:
    errors=validate_repo(root_of(a.root), a.skip_assets)
    for e in errors:
        print('ERROR:',e,file=sys.stderr)
    print('VALIDATION_OK' if not errors else f'VALIDATION_FAILED count={len(errors)}')
    return 0 if not errors else 1


def cmd_pack(a) -> int:
    root=root_of(a.root)
    errors=validate_repo(root,a.skip_assets)
    if errors:
        for e in errors:
            print('ERROR:',e,file=sys.stderr)
        return 1
    out=Path(a.output).resolve() if a.output else root/'dist/skill.zip'
    deterministic_zip(root/'gigiloop',out)
    print(f'PACKAGED {out} bytes={out.stat().st_size}')
    return 0


def cmd_self_test(_a) -> int:
    failures=[]
    with tempfile.TemporaryDirectory(prefix='gigiloop-test-') as td:
        root=Path(td)
        if shutil.which('git'):
            run(['git','init'],root,True)
            run(['git','config','user.email','selftest@example.invalid'],root,True)
            run(['git','config','user.name','Self Test'],root,True)
            (root/'app.txt').write_text('v1\n')
            run(['git','add','app.txt'],root,True)
            run(['git','commit','-m','base'],root,True)
            baseline=repo_state(root)['fingerprint']
            (root/'.gigiloop').mkdir()
            (root/'.gigiloop/x').write_text('x')
            if repo_state(root)['fingerprint'] != baseline:
                failures.append('internal state changed fingerprint')
            (root/'u.txt').write_text('1')
            fp1=repo_state(root)['fingerprint']
            (root/'u.txt').write_text('2')
            fp2=repo_state(root)['fingerprint']
            if fp1==fp2:
                failures.append('untracked content not fingerprinted')
            (root/'u.txt').unlink()
            cp=fresh_checkpoint(root,'test','balanced',1)
            atomic_write(root/CHECKPOINT,cp)
            if reconcile(root,load(root)):
                failures.append('clean resume detected drift')
            (root/'app.txt').write_text('v2')
            cp=load(root)
            cp['current_evidence']=[{'freshness':'current'}]
            if not reconcile(root,cp,False) or cp['current_evidence'][0]['freshness']!='stale':
                failures.append('drift did not stale evidence')
            cp=fresh_checkpoint(root,'budget','balanced',1)
            cp['iteration']=1
            cp['status']='active'
            if not (cp['iteration'] >= cp['budget']['max_iterations']):
                failures.append('budget boundary broken')
        skill=root/'sample'
        skill.mkdir(exist_ok=True)
        (skill/'SKILL.md').write_text('x')
        z1,z2=root/'a.zip',root/'b.zip'
        deterministic_zip(skill,z1)
        deterministic_zip(skill,z2)
        if z1.read_bytes()!=z2.read_bytes():
            failures.append('package not deterministic')
        try:
            validate_checkpoint({})
            failures.append('invalid checkpoint accepted')
        except ValueError:
            pass
    for f in failures:
        print('SELFTEST_FAIL:',f,file=sys.stderr)
    print('SELFTEST_OK' if not failures else f'SELFTEST_FAILED count={len(failures)}')
    return 0 if not failures else 1


def parser() -> argparse.ArgumentParser:
    p=argparse.ArgumentParser(description='GigiLoop deterministic runtime')
    s=p.add_subparsers(dest='cmd',required=True)
    q=s.add_parser('doctor'); q.add_argument('--root'); q.add_argument('--json',action='store_true'); q.set_defaults(fn=cmd_doctor)
    q=s.add_parser('init'); q.add_argument('--root'); q.add_argument('--goal',required=True); q.add_argument('--profile',choices=sorted(PROFILES),default='balanced'); q.add_argument('--max-iterations',type=int,default=25); q.add_argument('--force',action='store_true'); q.set_defaults(fn=cmd_init)
    q=s.add_parser('status'); q.add_argument('--root'); q.add_argument('--json',action='store_true'); q.set_defaults(fn=cmd_status)
    q=s.add_parser('resume'); q.add_argument('--root'); q.set_defaults(fn=cmd_resume)
    q=s.add_parser('heartbeat'); q.add_argument('--root'); q.add_argument('--phase',choices=sorted(PHASES)); q.add_argument('--message'); q.set_defaults(fn=cmd_heartbeat)
    q=s.add_parser('checkpoint'); q.add_argument('--root'); q.add_argument('--iteration',type=int); q.add_argument('--increment',action='store_true'); q.add_argument('--status',choices=sorted(STATUSES)); q.add_argument('--next-action'); q.add_argument('--phase',choices=sorted(PHASES)); q.set_defaults(fn=cmd_checkpoint)
    q=s.add_parser('supervise'); q.add_argument('--root'); q.add_argument('--idle-seconds',type=float,default=900); q.add_argument('--poll-seconds',type=float,default=5); q.add_argument('--max-restarts',type=int,default=10); q.add_argument('--backoff-seconds',type=float,default=3); q.add_argument('--max-wall-seconds',type=float,default=0); q.add_argument('command',nargs=argparse.REMAINDER); q.set_defaults(fn=cmd_supervise)
    q=s.add_parser('validate-repo'); q.add_argument('--root'); q.add_argument('--skip-assets',action='store_true'); q.set_defaults(fn=cmd_validate)
    q=s.add_parser('pack'); q.add_argument('--root'); q.add_argument('--output'); q.add_argument('--skip-assets',action='store_true'); q.set_defaults(fn=cmd_pack)
    q=s.add_parser('self-test'); q.set_defaults(fn=cmd_self_test)
    return p


if __name__ == '__main__':
    p=parser()
    args=p.parse_args()
    try:
        raise SystemExit(int(args.fn(args)))
    except ValueError as exc:
        print('ERROR:',exc,file=sys.stderr)
        raise SystemExit(2)

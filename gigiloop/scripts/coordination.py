#!/usr/bin/env python3
"""GigiLoop durable worker coordination runtime. Python standard library only.

This module provides host-neutral durable worker identity, a leased inbox with
receipt replay, reusable sleeping workers, a finish boundary, and compact
handoff snapshots. It deliberately does not automate any provider UI.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import tempfile
import time
import uuid

SCHEMA_VERSION = 1
STATE = Path('.gigiloop/coordination.json')
CHECKPOINT = Path('.gigiloop/checkpoint.json')
LOCK_TIMEOUT_SECONDS = 10.0
LOCK_STALE_SECONDS = 60.0
WORKER_STATUSES = {'working', 'sleeping', 'failed', 'finished'}
MESSAGE_STATUSES = {'queued', 'claimed', 'acked'}
MESSAGE_KINDS = {'immediate', 'after_turn'}


def now_dt() -> datetime:
    return datetime.now(timezone.utc).replace(microsecond=0)


def iso(value: datetime) -> str:
    return value.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z')


def now() -> str:
    return iso(now_dt())


def parse_ts(value: str | None) -> datetime | None:
    try:
        return datetime.fromisoformat((value or '').replace('Z', '+00:00')).astimezone(timezone.utc)
    except (TypeError, ValueError):
        return None


def root_of(start: str | None = None) -> Path:
    root = Path(start or '.').resolve()
    if root.is_file():
        root = root.parent
    cur = root
    while True:
        if (cur / CHECKPOINT).exists() or (cur / 'gigiloop/SKILL.md').exists():
            return cur
        if cur == cur.parent:
            return root
        cur = cur.parent


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
def coordination_lock(root: Path, timeout: float = LOCK_TIMEOUT_SECONDS, stale_after: float = LOCK_STALE_SECONDS):
    lock_path = root / '.gigiloop/coordination.lock'
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + max(0.0, timeout)
    token = f'{os.getpid()}:{uuid.uuid4()}'
    while True:
        try:
            fd = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(fd, 'w', encoding='utf-8') as handle:
                handle.write(token + '\n')
                handle.flush()
                os.fsync(handle.fileno())
            break
        except FileExistsError:
            try:
                age = time.time() - lock_path.stat().st_mtime
            except FileNotFoundError:
                continue
            if stale_after > 0 and age > stale_after:
                try:
                    lock_path.unlink()
                    continue
                except FileNotFoundError:
                    continue
            if time.monotonic() >= deadline:
                raise TimeoutError(f'timed out waiting for coordination lock: {lock_path}')
            time.sleep(0.025)
    try:
        yield
    finally:
        try:
            if lock_path.read_text(encoding='utf-8').strip() == token:
                lock_path.unlink()
        except FileNotFoundError:
            pass


def read_checkpoint(root: Path) -> dict:
    path = root / CHECKPOINT
    if not path.exists():
        raise SystemExit(f'Checkpoint not found: {path}. Run gigiloop.py init first.')
    try:
        data = json.loads(path.read_text(encoding='utf-8'))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f'Invalid checkpoint: {exc}') from exc
    if not data.get('run_id'):
        raise SystemExit('Invalid checkpoint: run_id is required')
    return data


def fresh_state(checkpoint: dict, max_active_workers: int = 2) -> dict:
    if max_active_workers < 1:
        raise ValueError('max_active_workers must be >= 1')
    ts = now()
    return {
        'schema_version': SCHEMA_VERSION,
        'run_id': checkpoint['run_id'],
        'checkpoint_generation': checkpoint.get('generation', 1),
        'context_generation': 1,
        'max_active_workers': max_active_workers,
        'workers': {},
        'inbox': [],
        'handoffs': [],
        'events': [],
        'sequence': 0,
        'created_at': ts,
        'updated_at': ts,
    }


def validate_state(data: dict) -> None:
    if data.get('schema_version') != SCHEMA_VERSION:
        raise ValueError('unsupported coordination schema')
    if not data.get('run_id'):
        raise ValueError('coordination run_id is required')
    if not isinstance(data.get('context_generation'), int) or data['context_generation'] < 1:
        raise ValueError('context_generation must be a positive integer')
    if not isinstance(data.get('max_active_workers'), int) or data['max_active_workers'] < 1:
        raise ValueError('max_active_workers must be a positive integer')
    workers = data.get('workers')
    if not isinstance(workers, dict):
        raise ValueError('workers must be an object')
    for worker_id, worker in workers.items():
        if not worker_id or not isinstance(worker, dict) or worker.get('status') not in WORKER_STATUSES:
            raise ValueError(f'invalid worker record: {worker_id!r}')
    inbox = data.get('inbox')
    if not isinstance(inbox, list):
        raise ValueError('inbox must be an array')
    message_ids: set[str] = set()
    receipts: set[str] = set()
    for message in inbox:
        if not isinstance(message, dict) or not message.get('id'):
            raise ValueError('invalid inbox message')
        if message['id'] in message_ids:
            raise ValueError(f'duplicate inbox message id: {message["id"]}')
        message_ids.add(message['id'])
        if message.get('status') not in MESSAGE_STATUSES:
            raise ValueError(f'invalid message status: {message.get("status")}')
        if message.get('kind') not in MESSAGE_KINDS:
            raise ValueError(f'invalid message kind: {message.get("kind")}')
        receipt = message.get('receipt')
        if receipt:
            if receipt in receipts:
                raise ValueError(f'duplicate receipt: {receipt}')
            receipts.add(receipt)


def load(root: Path, *, reconcile_checkpoint: bool = True) -> dict:
    path = root / STATE
    if not path.exists():
        raise SystemExit(f'Coordination state not found: {path}. Run coordination.py init first.')
    try:
        data = json.loads(path.read_text(encoding='utf-8'))
        validate_state(data)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        raise SystemExit(f'Invalid coordination state: {exc}') from exc
    if reconcile_checkpoint:
        checkpoint = read_checkpoint(root)
        if checkpoint['run_id'] != data['run_id']:
            raise SystemExit('Coordination state belongs to a different GigiLoop run; initialize it again for this checkpoint.')
    return data


def update(root: Path, mutator) -> dict:
    with coordination_lock(root):
        data = load(root)
        mutator(data)
        data['checkpoint_generation'] = read_checkpoint(root).get('generation', data.get('checkpoint_generation', 1))
        data['updated_at'] = now()
        validate_state(data)
        atomic_write(root / STATE, data)
        return data


def append_event(data: dict, event_type: str, **fields) -> None:
    event = {'at': now(), 'type': event_type, **fields}
    data.setdefault('events', []).append(event)
    if len(data['events']) > 500:
        data['events'] = data['events'][-500:]


def active_worker_count(data: dict) -> int:
    return sum(1 for worker in data.get('workers', {}).values() if worker.get('status') == 'working')


def expire_claims(data: dict, at: datetime | None = None) -> int:
    current = at or now_dt()
    expired = 0
    for message in data.get('inbox', []):
        if message.get('status') != 'claimed':
            continue
        lease_until = parse_ts(message.get('lease_until'))
        if lease_until is None or lease_until > current:
            continue
        append_event(data, 'message_lease_expired', message_id=message['id'], claimed_by=message.get('claimed_by'))
        message.update({
            'status': 'queued',
            'claimed_at': None,
            'claimed_by': None,
            'lease_until': None,
            'receipt': None,
        })
        expired += 1
    return expired


def eligible_messages(data: dict, target: str, kind: str | None = None) -> list[dict]:
    candidates = []
    for message in data.get('inbox', []):
        if message.get('status') != 'queued':
            continue
        if message.get('target') not in {target, '*'}:
            continue
        if kind and message.get('kind') != kind:
            continue
        candidates.append(message)
    return sorted(candidates, key=lambda item: (0 if item.get('kind') == 'immediate' else 1, item.get('sequence', 0)))


def outstanding_claim(data: dict, worker_id: str) -> dict | None:
    for message in data.get('inbox', []):
        if message.get('status') == 'claimed' and message.get('claimed_by') == worker_id:
            return message
    return None


def claim_record(data: dict, message: dict, worker_id: str, lease_seconds: float) -> dict:
    if lease_seconds <= 0:
        raise ValueError('lease_seconds must be > 0')
    ts = now_dt()
    receipt = f'rcpt-{uuid.uuid4()}'
    message.update({
        'status': 'claimed',
        'claimed_at': iso(ts),
        'claimed_by': worker_id,
        'lease_until': iso(ts + timedelta(seconds=lease_seconds)),
        'receipt': receipt,
        'attempts': int(message.get('attempts', 0)) + 1,
    })
    append_event(data, 'message_claimed', message_id=message['id'], worker_id=worker_id, receipt=receipt)
    return message


def public_message(message: dict | None) -> dict | None:
    if message is None:
        return None
    return {key: message.get(key) for key in (
        'id', 'target', 'kind', 'body', 'status', 'receipt', 'lease_until', 'attempts', 'dedupe_key'
    )}


def ack_record(data: dict, worker_id: str, receipt: str) -> dict:
    expire_claims(data)
    for message in data.get('inbox', []):
        if message.get('receipt') != receipt:
            continue
        if message.get('status') != 'claimed':
            raise ValueError('receipt is no longer claimable')
        if message.get('claimed_by') != worker_id:
            raise ValueError('receipt belongs to another worker')
        message['status'] = 'acked'
        message['acked_at'] = now()
        append_event(data, 'message_acked', message_id=message['id'], worker_id=worker_id, receipt=receipt)
        return message
    raise ValueError('unknown or expired receipt')


def require_worker(data: dict, worker_id: str, *, working: bool = False) -> dict:
    worker = data.get('workers', {}).get(worker_id)
    if worker is None:
        raise ValueError(f'unknown worker: {worker_id}')
    if working and worker.get('status') != 'working':
        raise ValueError(f'worker {worker_id} is not working')
    return worker


def cmd_init(a) -> int:
    root = root_of(a.root)
    checkpoint = read_checkpoint(root)
    path = root / STATE
    with coordination_lock(root):
        if path.exists() and not a.force:
            raise SystemExit(f'Coordination state already exists: {path}')
        data = fresh_state(checkpoint, a.max_active_workers)
        atomic_write(path, data)
    print(f'COORDINATION_INITIALIZED {path} run_id={data["run_id"]}')
    return 0


def cmd_status(a) -> int:
    root = root_of(a.root)
    expired = {'count': 0}
    def mutate(data):
        expired['count'] = expire_claims(data)
    data = update(root, mutate)
    counts = {status: 0 for status in WORKER_STATUSES}
    for worker in data['workers'].values():
        counts[worker['status']] += 1
    result = {
        'run_id': data['run_id'],
        'context_generation': data['context_generation'],
        'max_active_workers': data['max_active_workers'],
        'workers': counts,
        'queued_messages': sum(1 for item in data['inbox'] if item['status'] == 'queued'),
        'claimed_messages': sum(1 for item in data['inbox'] if item['status'] == 'claimed'),
        'acked_messages': sum(1 for item in data['inbox'] if item['status'] == 'acked'),
        'handoffs': len(data['handoffs']),
        'expired_claims_requeued': expired['count'],
    }
    if a.json:
        print(json.dumps(result, indent=2))
    else:
        print(' | '.join(f'{key}={value}' for key, value in result.items()))
    return 0


def cmd_worker_start(a) -> int:
    root = root_of(a.root)
    outcome: dict = {}
    def mutate(data):
        expire_claims(data)
        workers = data['workers']
        worker = workers.get(a.worker_id)
        if worker and worker.get('status') == 'finished':
            raise ValueError(f'worker {a.worker_id} is permanently finished')
        if worker and worker.get('status') == 'working':
            if a.role:
                worker['role'] = a.role
            if a.session_ref:
                worker['session_ref'] = a.session_ref
            worker['updated_at'] = now()
            outcome.update(worker)
            return
        if active_worker_count(data) >= data['max_active_workers']:
            raise ValueError('active worker limit reached')
        ts = now()
        if worker:
            worker.update({
                'status': 'working',
                'role': a.role or worker.get('role') or 'worker',
                'session_ref': a.session_ref or worker.get('session_ref'),
                'updated_at': ts,
                'last_started_at': ts,
                'revivals': int(worker.get('revivals', 0)) + 1,
            })
            append_event(data, 'worker_revived', worker_id=a.worker_id)
        else:
            worker = {
                'id': a.worker_id,
                'role': a.role or 'worker',
                'status': 'working',
                'session_ref': a.session_ref,
                'created_at': ts,
                'updated_at': ts,
                'last_started_at': ts,
                'last_result': None,
                'turns_completed': 0,
                'revivals': 0,
            }
            workers[a.worker_id] = worker
            append_event(data, 'worker_created', worker_id=a.worker_id, role=worker['role'])
        outcome.update(worker)
    update(root, mutate)
    print(json.dumps(outcome, indent=2))
    return 0


def requeue_worker_claim(data: dict, worker_id: str, reason: str) -> None:
    claim = outstanding_claim(data, worker_id)
    if not claim:
        return
    append_event(data, 'message_requeued', message_id=claim['id'], worker_id=worker_id, reason=reason)
    claim.update({'status': 'queued', 'claimed_at': None, 'claimed_by': None, 'lease_until': None, 'receipt': None})


def cmd_worker_sleep(a) -> int:
    root = root_of(a.root)
    def mutate(data):
        worker = require_worker(data, a.worker_id)
        if worker['status'] == 'finished':
            raise ValueError('finished worker cannot sleep')
        requeue_worker_claim(data, a.worker_id, 'worker_sleep')
        worker['status'] = 'sleeping'
        worker['updated_at'] = now()
        if a.result is not None:
            worker['last_result'] = a.result
        append_event(data, 'worker_sleeping', worker_id=a.worker_id)
    data = update(root, mutate)
    print(f'WORKER_SLEEPING {a.worker_id} active={active_worker_count(data)}')
    return 0


def cmd_worker_fail(a) -> int:
    root = root_of(a.root)
    def mutate(data):
        worker = require_worker(data, a.worker_id)
        if worker['status'] == 'finished':
            raise ValueError('finished worker cannot fail')
        requeue_worker_claim(data, a.worker_id, 'worker_failed')
        worker['status'] = 'failed'
        worker['updated_at'] = now()
        worker['last_result'] = a.result
        append_event(data, 'worker_failed', worker_id=a.worker_id)
    update(root, mutate)
    print(f'WORKER_FAILED {a.worker_id}')
    return 0


def cmd_send(a) -> int:
    root = root_of(a.root)
    outcome: dict = {}
    def mutate(data):
        expire_claims(data)
        if a.target not in {'prime', '*'} and a.target not in data['workers']:
            raise ValueError(f'unknown target worker: {a.target}')
        if a.dedupe_key:
            for existing in data['inbox']:
                if (existing.get('target') == a.target and existing.get('dedupe_key') == a.dedupe_key
                        and existing.get('status') in {'queued', 'claimed'}):
                    outcome.update(existing)
                    return
        data['sequence'] = int(data.get('sequence', 0)) + 1
        message = {
            'id': f'msg-{data["sequence"]:06d}-{uuid.uuid4().hex[:8]}',
            'sequence': data['sequence'],
            'target': a.target,
            'kind': a.kind,
            'body': a.message,
            'dedupe_key': a.dedupe_key,
            'status': 'queued',
            'created_at': now(),
            'claimed_at': None,
            'claimed_by': None,
            'lease_until': None,
            'receipt': None,
            'attempts': 0,
            'acked_at': None,
        }
        data['inbox'].append(message)
        append_event(data, 'message_queued', message_id=message['id'], target=a.target, kind=a.kind)
        outcome.update(message)
    update(root, mutate)
    print(json.dumps(public_message(outcome), indent=2))
    return 0


def cmd_claim(a) -> int:
    root = root_of(a.root)
    outcome: dict = {'message': None}
    def mutate(data):
        expire_claims(data)
        if a.worker_id != 'prime':
            require_worker(data, a.worker_id, working=True)
        existing = outstanding_claim(data, a.worker_id)
        if existing:
            outcome['message'] = existing
            return
        candidates = eligible_messages(data, a.worker_id, a.kind)
        if candidates:
            outcome['message'] = claim_record(data, candidates[0], a.worker_id, a.lease_seconds)
    update(root, mutate)
    if outcome['message'] is None:
        print('NO_MESSAGE')
        return 1
    print(json.dumps(public_message(outcome['message']), indent=2))
    return 0


def cmd_ack(a) -> int:
    root = root_of(a.root)
    outcome: dict = {}
    def mutate(data):
        if a.worker_id != 'prime':
            require_worker(data, a.worker_id)
        outcome.update(ack_record(data, a.worker_id, a.receipt))
    update(root, mutate)
    print(f'ACKED {outcome["id"]} receipt={a.receipt}')
    return 0


def cmd_finish(a) -> int:
    root = root_of(a.root)
    outcome: dict = {'status': None, 'next_message': None}
    def mutate(data):
        expire_claims(data)
        worker = require_worker(data, a.worker_id, working=True)
        claim = outstanding_claim(data, a.worker_id)
        if claim:
            if not a.receipt:
                raise ValueError('worker has an unacknowledged message; pass --receipt to finish atomically')
            if claim.get('receipt') != a.receipt:
                raise ValueError('finish receipt does not match the worker current claim')
            ack_record(data, a.worker_id, a.receipt)
        elif a.receipt:
            raise ValueError('worker has no active claim for the supplied receipt')
        worker['last_result'] = a.result
        worker['turns_completed'] = int(worker.get('turns_completed', 0)) + 1
        worker['updated_at'] = now()
        if a.permanent:
            worker['status'] = 'finished'
            worker['finished_at'] = now()
            append_event(data, 'worker_finished', worker_id=a.worker_id, permanent=True)
            outcome['status'] = 'finished'
            return
        candidates = eligible_messages(data, a.worker_id)
        if candidates:
            next_message = claim_record(data, candidates[0], a.worker_id, a.lease_seconds)
            worker['status'] = 'working'
            outcome['status'] = 'continue'
            outcome['next_message'] = next_message
            append_event(data, 'worker_continues', worker_id=a.worker_id, message_id=next_message['id'])
        else:
            worker['status'] = 'sleeping'
            worker['sleeping_since'] = now()
            outcome['status'] = 'sleeping'
            append_event(data, 'worker_finished', worker_id=a.worker_id, permanent=False)
    update(root, mutate)
    print(json.dumps({'status': outcome['status'], 'next_message': public_message(outcome['next_message'])}, indent=2))
    return 0


def cmd_handoff(a) -> int:
    root = root_of(a.root)
    checkpoint = read_checkpoint(root)
    outcome: dict = {}
    def mutate(data):
        expire_claims(data)
        current_generation = int(data.get('context_generation', 1))
        data['context_generation'] = current_generation + 1
        handoff = {
            'id': f'handoff-{uuid.uuid4()}',
            'at': now(),
            'reason': a.reason,
            'from_context_generation': current_generation,
            'to_context_generation': data['context_generation'],
            'summary': a.summary,
            'next_action': a.next_action,
            'run_id': data['run_id'],
            'checkpoint_generation': checkpoint.get('generation', 1),
            'repository_fingerprint': (checkpoint.get('repository') or {}).get('fingerprint'),
            'iteration': checkpoint.get('iteration'),
            'worker_snapshot': {
                worker_id: {
                    'role': worker.get('role'),
                    'status': worker.get('status'),
                    'session_ref': worker.get('session_ref'),
                    'turns_completed': worker.get('turns_completed', 0),
                }
                for worker_id, worker in data['workers'].items()
            },
            'pending_message_ids': [item['id'] for item in data['inbox'] if item['status'] in {'queued', 'claimed'}],
        }
        data['handoffs'].append(handoff)
        if len(data['handoffs']) > 100:
            data['handoffs'] = data['handoffs'][-100:]
        append_event(data, 'context_handoff', handoff_id=handoff['id'], reason=a.reason)
        outcome.update(handoff)
    update(root, mutate)
    print(json.dumps(outcome, indent=2))
    return 0


def cmd_self_test(_a) -> int:
    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix='gigiloop-coordination-test-') as td:
        root = Path(td)
        (root / '.gigiloop').mkdir()
        checkpoint = {
            'run_id': 'self-test-run',
            'generation': 1,
            'iteration': 0,
            'repository': {'fingerprint': '0' * 64},
        }
        atomic_write(root / CHECKPOINT, checkpoint)
        atomic_write(root / STATE, fresh_state(checkpoint, 2))

        def start_worker(data):
            ts = now()
            data['workers']['w1'] = {'id': 'w1', 'role': 'builder', 'status': 'working', 'session_ref': 'session-1',
                                     'created_at': ts, 'updated_at': ts, 'turns_completed': 0, 'revivals': 0}
        update(root, start_worker)

        def queue_once(data):
            data['sequence'] += 1
            data['inbox'].append({'id': 'm1', 'sequence': data['sequence'], 'target': 'w1', 'kind': 'after_turn', 'body': 'next',
                                  'dedupe_key': 'd1', 'status': 'queued', 'created_at': now(), 'claimed_at': None,
                                  'claimed_by': None, 'lease_until': None, 'receipt': None, 'attempts': 0, 'acked_at': None})
        update(root, queue_once)

        claimed: dict = {}
        def claim_once(data):
            claimed.update(claim_record(data, eligible_messages(data, 'w1')[0], 'w1', 60))
        update(root, claim_once)
        replay = load(root)['inbox'][0]
        if replay.get('receipt') != claimed.get('receipt'):
            failures.append('receipt not durable')

        def finish_once(data):
            ack_record(data, 'w1', claimed['receipt'])
            worker = data['workers']['w1']
            worker['status'] = 'sleeping'
            worker['turns_completed'] = 1
        update(root, finish_once)
        state = load(root)
        if state['workers']['w1']['status'] != 'sleeping' or state['inbox'][0]['status'] != 'acked':
            failures.append('finish boundary state not durable')

        run_id = state['run_id']
        state['context_generation'] += 1
        atomic_write(root / STATE, state)
        if load(root)['run_id'] != run_id:
            failures.append('context handoff changed durable run identity')

    for failure in failures:
        print('COORDINATION_SELFTEST_FAIL:', failure)
    print('COORDINATION_SELFTEST_OK' if not failures else f'COORDINATION_SELFTEST_FAILED count={len(failures)}')
    return 0 if not failures else 1


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description='GigiLoop durable worker coordination runtime')
    s = p.add_subparsers(dest='cmd', required=True)

    q = s.add_parser('init')
    q.add_argument('--root')
    q.add_argument('--max-active-workers', type=int, default=2)
    q.add_argument('--force', action='store_true')
    q.set_defaults(fn=cmd_init)

    q = s.add_parser('status')
    q.add_argument('--root')
    q.add_argument('--json', action='store_true')
    q.set_defaults(fn=cmd_status)

    q = s.add_parser('worker-start')
    q.add_argument('--root')
    q.add_argument('--id', dest='worker_id', required=True)
    q.add_argument('--role')
    q.add_argument('--session-ref')
    q.set_defaults(fn=cmd_worker_start)

    q = s.add_parser('worker-sleep')
    q.add_argument('--root')
    q.add_argument('--id', dest='worker_id', required=True)
    q.add_argument('--result')
    q.set_defaults(fn=cmd_worker_sleep)

    q = s.add_parser('worker-fail')
    q.add_argument('--root')
    q.add_argument('--id', dest='worker_id', required=True)
    q.add_argument('--result', required=True)
    q.set_defaults(fn=cmd_worker_fail)

    q = s.add_parser('send')
    q.add_argument('--root')
    q.add_argument('--target', required=True)
    q.add_argument('--kind', choices=sorted(MESSAGE_KINDS), default='after_turn')
    q.add_argument('--message', required=True)
    q.add_argument('--dedupe-key')
    q.set_defaults(fn=cmd_send)

    q = s.add_parser('claim')
    q.add_argument('--root')
    q.add_argument('--id', dest='worker_id', required=True)
    q.add_argument('--kind', choices=sorted(MESSAGE_KINDS))
    q.add_argument('--lease-seconds', type=float, default=300)
    q.set_defaults(fn=cmd_claim)

    q = s.add_parser('ack')
    q.add_argument('--root')
    q.add_argument('--id', dest='worker_id', required=True)
    q.add_argument('--receipt', required=True)
    q.set_defaults(fn=cmd_ack)

    q = s.add_parser('finish')
    q.add_argument('--root')
    q.add_argument('--id', dest='worker_id', required=True)
    q.add_argument('--result', required=True)
    q.add_argument('--receipt')
    q.add_argument('--lease-seconds', type=float, default=300)
    q.add_argument('--permanent', action='store_true')
    q.set_defaults(fn=cmd_finish)

    q = s.add_parser('handoff')
    q.add_argument('--root')
    q.add_argument('--summary', required=True)
    q.add_argument('--next-action', required=True)
    q.add_argument('--reason', choices=['compact', 'context_reset', 'manual', 'host_restart'], default='manual')
    q.set_defaults(fn=cmd_handoff)

    q = s.add_parser('self-test')
    q.set_defaults(fn=cmd_self_test)
    return p


if __name__ == '__main__':
    args = parser().parse_args()
    try:
        raise SystemExit(int(args.fn(args)))
    except (ValueError, TimeoutError) as exc:
        print('ERROR:', exc, file=os.sys.stderr)
        raise SystemExit(2)

import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest

COORDINATION = Path(__file__).resolve().parents[1] / 'gigiloop' / 'scripts' / 'coordination.py'
spec = importlib.util.spec_from_file_location('gigiloop_coordination', COORDINATION)
coord = importlib.util.module_from_spec(spec)
assert spec.loader
spec.loader.exec_module(coord)


def make_root(root: Path, run_id: str = 'run-1', max_workers: int = 2) -> None:
    (root / '.gigiloop').mkdir(parents=True, exist_ok=True)
    checkpoint = {
        'run_id': run_id,
        'generation': 1,
        'iteration': 0,
        'repository': {'fingerprint': 'a' * 64},
    }
    coord.atomic_write(root / coord.CHECKPOINT, checkpoint)
    coord.atomic_write(root / coord.STATE, coord.fresh_state(checkpoint, max_workers))


def start_worker(root: Path, worker_id: str = 'w1', role: str = 'builder', session_ref: str = 'session-1') -> None:
    args = SimpleNamespace(root=str(root), worker_id=worker_id, role=role, session_ref=session_ref)
    assert coord.cmd_worker_start(args) == 0


def send(root: Path, target: str = 'w1', message: str = 'task', kind: str = 'after_turn', dedupe_key=None) -> None:
    args = SimpleNamespace(root=str(root), target=target, message=message, kind=kind, dedupe_key=dedupe_key)
    assert coord.cmd_send(args) == 0


class CoordinationTests(unittest.TestCase):
    def test_sleeping_worker_reuses_identity_and_session_ref(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root)
            coord.cmd_worker_sleep(SimpleNamespace(root=str(root), worker_id='w1', result='done'))
            coord.cmd_worker_start(SimpleNamespace(root=str(root), worker_id='w1', role=None, session_ref=None))
            worker = coord.load(root)['workers']['w1']
            self.assertEqual(worker['status'], 'working')
            self.assertEqual(worker['session_ref'], 'session-1')
            self.assertEqual(worker['revivals'], 1)

    def test_active_worker_limit_counts_only_working_workers(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root, max_workers=1); start_worker(root, 'w1')
            with self.assertRaisesRegex(ValueError, 'active worker limit'):
                coord.cmd_worker_start(SimpleNamespace(root=str(root), worker_id='w2', role='verifier', session_ref='s2'))
            coord.cmd_worker_sleep(SimpleNamespace(root=str(root), worker_id='w1', result=None))
            self.assertEqual(coord.cmd_worker_start(SimpleNamespace(root=str(root), worker_id='w2', role='verifier', session_ref='s2')), 0)

    def test_dedupe_key_makes_send_idempotent_until_ack(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root)
            send(root, dedupe_key='task-42')
            send(root, message='duplicate payload ignored', dedupe_key='task-42')
            state = coord.load(root)
            self.assertEqual(len(state['inbox']), 1)
            self.assertEqual(state['inbox'][0]['body'], 'task')

    def test_claim_replays_same_receipt_until_ack_or_expiry(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root); send(root)
            args = SimpleNamespace(root=str(root), worker_id='w1', kind=None, lease_seconds=300)
            self.assertEqual(coord.cmd_claim(args), 0)
            first = coord.load(root)['inbox'][0].copy()
            self.assertEqual(coord.cmd_claim(args), 0)
            second = coord.load(root)['inbox'][0]
            self.assertEqual(first['receipt'], second['receipt'])
            self.assertEqual(second['attempts'], 1)

    def test_expired_claim_is_requeued_for_recovery(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root); send(root)
            def claim_then_expire(data):
                message = coord.claim_record(data, coord.eligible_messages(data, 'w1')[0], 'w1', 300)
                message['lease_until'] = '2000-01-01T00:00:00Z'
            coord.update(root, claim_then_expire)
            changed = {'count': 0}
            def expire(data):
                changed['count'] = coord.expire_claims(data)
            coord.update(root, expire)
            message = coord.load(root)['inbox'][0]
            self.assertEqual(changed['count'], 1)
            self.assertEqual(message['status'], 'queued')
            self.assertIsNone(message['receipt'])

    def test_finish_boundary_acks_current_and_claims_queued_followup(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root)
            send(root, message='first', kind='immediate')
            coord.cmd_claim(SimpleNamespace(root=str(root), worker_id='w1', kind=None, lease_seconds=300))
            first_receipt = coord.load(root)['inbox'][0]['receipt']
            send(root, message='follow-up', kind='after_turn')
            args = SimpleNamespace(root=str(root), worker_id='w1', result='first done', receipt=first_receipt,
                                   lease_seconds=300, permanent=False)
            self.assertEqual(coord.cmd_finish(args), 0)
            state = coord.load(root)
            self.assertEqual(state['inbox'][0]['status'], 'acked')
            self.assertEqual(state['inbox'][1]['status'], 'claimed')
            self.assertEqual(state['inbox'][1]['claimed_by'], 'w1')
            self.assertEqual(state['workers']['w1']['status'], 'working')
            self.assertEqual(state['workers']['w1']['turns_completed'], 1)

    def test_finish_without_followup_puts_worker_to_sleep(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root)
            args = SimpleNamespace(root=str(root), worker_id='w1', result='done', receipt=None,
                                   lease_seconds=300, permanent=False)
            self.assertEqual(coord.cmd_finish(args), 0)
            self.assertEqual(coord.load(root)['workers']['w1']['status'], 'sleeping')

    def test_handoff_preserves_run_identity_and_snapshots_workers_and_queue(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root); send(root, message='pending')
            args = SimpleNamespace(root=str(root), summary='context nearly full', next_action='continue pending task', reason='compact')
            self.assertEqual(coord.cmd_handoff(args), 0)
            state = coord.load(root)
            self.assertEqual(state['run_id'], 'run-1')
            self.assertEqual(state['context_generation'], 2)
            handoff = state['handoffs'][-1]
            self.assertEqual(handoff['run_id'], 'run-1')
            self.assertEqual(handoff['worker_snapshot']['w1']['session_ref'], 'session-1')
            self.assertEqual(handoff['pending_message_ids'], [state['inbox'][0]['id']])

    def test_permanent_finish_rejects_orphaning_targeted_messages(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root); start_worker(root); send(root, target='w1', message='still pending')
            args = SimpleNamespace(root=str(root), worker_id='w1', result='done', receipt=None,
                                   lease_seconds=300, permanent=True)
            with self.assertRaisesRegex(ValueError, 'queued targeted messages'):
                coord.cmd_finish(args)
            state = coord.load(root)
            self.assertEqual(state['workers']['w1']['status'], 'working')
            self.assertEqual(state['inbox'][0]['status'], 'queued')

    def test_update_rejects_checkpoint_run_change_mid_mutation(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root)
            def mutate(data):
                checkpoint = json.loads((root / coord.CHECKPOINT).read_text(encoding='utf-8'))
                checkpoint['run_id'] = 'run-2'
                coord.atomic_write(root / coord.CHECKPOINT, checkpoint)
                data['sequence'] = 99
            with self.assertRaisesRegex(ValueError, 'checkpoint run changed'):
                coord.update(root, mutate)
            raw = json.loads((root / coord.STATE).read_text(encoding='utf-8'))
            self.assertEqual(raw['run_id'], 'run-1')
            self.assertEqual(raw['sequence'], 0)

    def test_coordination_refuses_checkpoint_from_another_run(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); make_root(root)
            checkpoint = json.loads((root / coord.CHECKPOINT).read_text(encoding='utf-8'))
            checkpoint['run_id'] = 'run-2'
            coord.atomic_write(root / coord.CHECKPOINT, checkpoint)
            with self.assertRaisesRegex(SystemExit, 'different GigiLoop run'):
                coord.load(root)


if __name__ == '__main__':
    unittest.main(verbosity=2)

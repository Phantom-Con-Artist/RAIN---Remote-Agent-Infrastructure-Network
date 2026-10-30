"""Provider conversations must resume only their own RAIN direct/group thread."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ADAPTER = Path(__file__).resolve().parents[1] / 'src' / 'chat_adapter.py'
FAKE = r'''#!/usr/bin/env python3
import json,os,sys,uuid
provider=os.environ['TEST_PROVIDER']
flag={'codex_chat':'resume','claude_chat':'--resume','agy_chat':'--conversation','muse_chat':'--session-id'}[provider]
thread=sys.argv[sys.argv.index(flag)+1] if flag in sys.argv else str(uuid.uuid4())
if provider=='codex_chat':
    sys.stdin.read()
    print(json.dumps({'type':'thread.started','thread_id':thread}))
    print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':thread}}))
elif provider=='claude_chat':
    sys.stdin.read()
    print(json.dumps({'session_id':thread,'result':thread}))
elif provider=='agy_chat':
    print(json.dumps({'status':'SUCCESS','conversation_id':thread,'response':thread}))
else:
    print(json.dumps({'payload_type':'run.terminal.completed','payload':{'terminal':'completed','text':thread}}))
'''

class AdapterIsolationTests(unittest.TestCase):
    def test_each_provider_keeps_direct_and_group_threads_separate(self):
        with tempfile.TemporaryDirectory(prefix='rain-thread-test-') as folder:
            cli = Path(folder) / 'provider'
            cli.write_text(FAKE)
            cli.chmod(0o700)
            for provider in ['codex_chat', 'claude_chat', 'agy_chat', 'muse_chat']:
                with self.subTest(provider=provider):
                    frames = [{'conversation_id': c, 'text': 'ordinary message'} for c in ['group', 'direct', 'group', 'direct']]
                    result = subprocess.run(['python3', str(ADAPTER), provider, str(cli)],
                        input=''.join(json.dumps(f)+'\n' for f in frames), text=True, capture_output=True,
                        env={**os.environ, 'TEST_PROVIDER': provider}, timeout=10, check=True)
                    replies = [e for line in result.stdout.splitlines() if (e := json.loads(line))['rain_chat']=='reply']
                    self.assertEqual([e['conversation_id'] for e in replies], ['group', 'direct', 'group', 'direct'])
                    self.assertEqual(replies[0]['text'], replies[2]['text'])
                    self.assertEqual(replies[1]['text'], replies[3]['text'])
                    self.assertNotEqual(replies[0]['text'], replies[1]['text'])

if __name__ == '__main__':
    unittest.main()

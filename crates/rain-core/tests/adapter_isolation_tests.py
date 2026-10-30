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

    def test_human_direction_interrupts_autonomous_turn_and_drops_stale_queue(self):
        import time
        with tempfile.TemporaryDirectory(prefix='rain-priority-') as folder:
            root = Path(folder)
            cli = root/'provider'
            cli.write_text("#!/usr/bin/env python3\nimport sys,json,time,os\nprompt=sys.argv[sys.argv.index('--print')+1]\nif prompt=='slow-agent':\n open(os.environ['STARTED'],'w').write('yes')\n time.sleep(20)\nprint(json.dumps({'status':'SUCCESS','response':prompt}))\n")
            cli.chmod(0o700)
            bridge = subprocess.Popen(['python3',str(ADAPTER),'agy_chat',str(cli)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env={**os.environ,'STARTED':str(root/'started')})
            def send(text,origin,run):
                bridge.stdin.write(json.dumps({'conversation_id':'channel','text':text,'origin':origin,'run_id':run})+'\n')
                bridge.stdin.flush()
            try:
                send('slow-agent','agent','old')
                deadline=time.monotonic()+5
                while not (root/'started').exists():
                    self.assertLess(time.monotonic(),deadline)
                    time.sleep(.02)
                send('queued-old-agent','agent','old')
                send('urgent-human','human','new')
                bridge.stdin.close();bridge.stdin=None
                output,errors=bridge.communicate(timeout=6)
                replies=[json.loads(line) for line in output.splitlines() if json.loads(line)['rain_chat']=='reply']
                self.assertEqual([(r['text'],r.get('run_id')) for r in replies],[('urgent-human','new')],errors)
            finally:
                if bridge.poll() is None:
                    bridge.terminate();bridge.wait(timeout=5)

if __name__ == '__main__':
    unittest.main()

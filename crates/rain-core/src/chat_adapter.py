"""Persistent stdin bridge for installed agent CLIs. No credentials or approval bypasses.
Codex: https://learn.chatgpt.com/docs/non-interactive-mode
Claude: https://code.claude.com/docs/en/cli-usage
Agy/Muse flags and event formats are checked against their installed CLI help/output.
"""
import json
import subprocess
import sys
import tempfile
import uuid

provider, executable = sys.argv[1:3]
session = None
muse_session = str(uuid.uuid4())

def emit(kind, **data):
    print(json.dumps(dict(rain_chat=kind, **data)), flush=True)

for raw in sys.stdin:
    prompt_file = None
    try:
        request = json.loads(raw)
        prompt = request['text']
        conversation = request['conversation_id']
        emit('activity', state='working')
        if provider == 'codex_chat':
            args = [executable, 'exec'] + (['resume', session] if session else []) + ['--json', '-']
            input_text = prompt
        elif provider == 'claude_chat':
            args = [executable, '-p', '--output-format', 'json'] + (['--resume', session] if session else [])
            input_text = prompt
        elif provider == 'agy_chat':
            args = [executable, '--print', prompt, '--output-format', 'json'] + (['--conversation', session] if session else [])
            input_text = ''
        elif provider == 'muse_chat':
            prompt_file = tempfile.NamedTemporaryFile(mode='w+', suffix='.txt', encoding='utf-8')
            prompt_file.write(prompt)
            prompt_file.flush()
            args = [executable, 'exec', '--json', '--session-id', muse_session, '--prompt-file', prompt_file.name]
            input_text = ''
        else:
            raise ValueError('Unsupported chat provider')
        # stderr goes to a file so long diagnostic output cannot deadlock stdout parsing.
        with tempfile.TemporaryFile(mode='w+') as errors:
            child = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, text=True)
            # Concurrent pipe writing handles long prompts while the CLI starts streaming output.
            import threading
            def write_input():
                try:
                    child.stdin.write(input_text)
                    child.stdin.close()
                except (BrokenPipeError, OSError):
                    pass
            writer = threading.Thread(target=write_input, daemon=True)
            writer.start()
            replies = []
            failure = None
            for line in child.stdout:
                try:
                    event = json.loads(line)
                except ValueError:
                    continue
                if provider == 'codex_chat':
                    if event.get('type') == 'thread.started': session = event.get('thread_id')
                    item = event.get('item', {})
                    if event.get('type') == 'item.completed' and item.get('type') == 'agent_message': replies.append(item.get('text', ''))
                    if event.get('type') in ('error', 'turn.failed'): failure = event.get('message') or str(event.get('error', 'Provider turn failed'))
                elif provider == 'claude_chat':
                    session = event.get('session_id', session)
                    if event.get('is_error'): failure = event.get('result', 'Claude turn failed')
                    elif 'result' in event: replies.append(event['result'])
                elif provider == 'agy_chat':
                    session = event.get('conversation_id', session)
                    if event.get('status') not in (None, 'SUCCESS'): failure = event.get('response') or str(event)
                    elif 'response' in event: replies.append(event['response'])
                elif provider == 'muse_chat':
                    if event.get('payload_type') == 'run.terminal.completed':
                        payload = event.get('payload', {})
                        if payload.get('terminal') == 'completed': replies.append(payload.get('text', ''))
                        else: failure = payload.get('reason') or 'Muse turn failed'
            code = child.wait()
            writer.join(timeout=1)
            if code or failure:
                errors.seek(0)
                detail = errors.read()[-4000:]
                raise RuntimeError(failure or detail or f'CLI exited with code {code}')
            reply = '\n\n'.join(text for text in replies if text.strip()).strip()
            if not reply:
                raise RuntimeError('CLI returned no supported assistant response; inspect its native terminal or authentication setup')
            emit('reply', conversation_id=conversation, text=reply)
    except Exception as error:
        emit('reply', conversation_id=request.get('conversation_id', '') if 'request' in locals() else '', text=f'Agent error: {error}')
    finally:
        if prompt_file is not None:
            prompt_file.close()
        emit('activity', state='online')

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
import os
import signal
import threading
import time
from collections import deque

provider, executable = sys.argv[1:3]
sessions = {}
muse_sessions = {}

emit_lock = threading.Lock()
def emit(kind, **data):
    with emit_lock:
        print(json.dumps(dict(rain_chat=kind, **data)), flush=True)

pending = deque()
condition = threading.Condition()
input_closed = False
active_request = None
active_child = None

def interrupt(child):
    if child is None or child.poll() is not None:
        return
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    def force_if_needed():
        time.sleep(1)
        if child.poll() is None:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    threading.Thread(target=force_if_needed, daemon=True).start()

def on_stop(signum, frame):
    interrupt(active_child)
    raise SystemExit(0)

signal.signal(signal.SIGTERM, on_stop)
signal.signal(signal.SIGINT, on_stop)

def read_requests():
    global input_closed
    for raw in sys.stdin:
        try:
            frame = json.loads(raw)
        except ValueError:
            continue
        with condition:
            conversation = frame.get('conversation_id')
            cancel = frame.get('control') in ('cancel', 'cancel_auto', 'reset')
            cancel_all = frame.get('control') in ('cancel','reset')
            human = frame.get('origin') == 'human'
            if cancel or human:
                pending_copy = [item for item in pending if not (item.get('conversation_id') == conversation and (cancel_all or item.get('origin') == 'agent' or (frame.get('run_id') and item.get('run_id') != frame.get('run_id'))))]
                for item in pending:
                    if item not in pending_copy and item.get('dispatch_id'):
                        emit('turn', dispatch_id=item['dispatch_id'], state='cancelled')
                pending.clear()
                pending.extend(pending_copy)
                if active_request and active_request.get('conversation_id') == conversation and (cancel_all or active_request.get('origin') == 'agent'):
                    active_request['_cancelled'] = True
                    interrupt(active_child)
            if frame.get('control') == 'reset':
                pending.appendleft(frame)
            if not cancel:
                if human:
                    index = next((i for i,item in enumerate(pending) if item.get('origin') == 'agent'), len(pending))
                    pending.insert(index, frame)
                else:
                    pending.append(frame)
            condition.notify()
    with condition:
        input_closed = True
        condition.notify()

threading.Thread(target=read_requests, daemon=True).start()
while True:
    with condition:
        while not pending and not input_closed:
            condition.wait()
        if not pending:
            break
        request = pending.popleft()
        active_request = request
    if request.get('control') == 'reset':
        sessions.pop(request.get('conversation_id'), None)
        muse_sessions.pop(request.get('conversation_id'), None)
        with condition:
            active_request = None
        continue
    prompt_file = None
    conversation = None
    session = None
    turn_outcome = "completed"
    turn_error = None
    try:
        prompt = request['text']
        conversation = request['conversation_id']
        session = sessions.get(conversation)
        emit('activity', state='working')
        emit('turn', dispatch_id=request.get('dispatch_id'), state='running')
        if provider == 'codex_chat':
            args = [executable, 'exec'] + (['resume', session] if session else []) + ['--json', '-']
            input_text = prompt
        elif provider == 'claude_chat':
            args = [executable, '-p', '--output-format', 'json'] + (['--resume', session] if session else [])
            input_text = prompt
        elif provider == 'agy_chat':
            args = [executable, '--print', prompt, '--output-format', 'stream-json'] + (['--conversation', session] if session else [])
            input_text = ''
        elif provider == 'muse_chat':
            prompt_file = tempfile.NamedTemporaryFile(mode='w+', suffix='.txt', encoding='utf-8')
            prompt_file.write(prompt)
            prompt_file.flush()
            args = [executable, 'exec', '--json', '--session-id', muse_sessions.setdefault(conversation, str(uuid.uuid4())), '--prompt-file', prompt_file.name]
            input_text = ''
        else:
            raise ValueError('Unsupported chat provider')
        options = request.get('options') or {}
        if options.get('model'):
            args += ['--model', options['model']]
        if options.get('effort'):
            if provider == 'muse_chat':
                args += ['--reasoning-effort', options['effort']]
            elif provider in ('agy_chat', 'claude_chat'):
                args += ['--effort', options['effort']]
        # stderr goes to a file so long diagnostic output cannot deadlock stdout parsing.
        with tempfile.TemporaryFile(mode='w+') as errors:
            child = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, text=True, start_new_session=True)
            with condition:
                active_child = child
                if request.get('_cancelled'):
                    interrupt(child)
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
                # Display only observable tool metadata, never reasoning, tool output, or text deltas.
                step = None
                if provider == 'agy_chat':
                    update = event.get('step_update') or {}
                    tool = update.get('tool_name')
                    if isinstance(tool, str):
                        params = (update.get('tool_info') or {}).get('parameters') or {}
                        path = next((params[k] for k in ('Path','path','file_path','FilePath') if isinstance(params.get(k), str)), None)
                        step = f"{tool} · {update.get('state', 'observed')}" + (f" · {path[:300]}" if path else '')
                elif provider == 'codex_chat':
                    item = event.get('item') or {}
                    if item.get('type') in ('command_execution','file_change','mcp_tool_call','web_search'):
                        step = f"{item['type']} · {event.get('type', 'observed')}"
                        if item.get('type') == 'file_change':
                            paths = [c.get('path','') for c in item.get('changes',[]) if isinstance(c,dict)]
                            step += ' · ' + ', '.join(paths)[:300]
                elif provider == 'muse_chat':
                    kind = event.get('payload_type', '')
                    if kind in ('task.lifecycle.started','task.lifecycle.completed','task.lifecycle.rejected','task.lifecycle.scheduled','tool.result'):
                        step = kind.replace('.', ' · ')
                if step:
                    emit('trace', dispatch_id=request.get('dispatch_id'), step=step[:500])
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
                    event = event.get('result') if isinstance(event.get('result'), dict) else event
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
            if request.get('_cancelled'):
                continue
            if code or failure:
                errors.seek(0)
                detail = errors.read()[-4000:]
                raise RuntimeError(failure or detail or f'CLI exited with code {code}')
            reply = '\n\n'.join(text for text in replies if text.strip()).strip()
            if not reply:
                raise RuntimeError('CLI returned no supported assistant response; inspect its native terminal or authentication setup')
            emit('reply', conversation_id=conversation, run_id=request.get('run_id'), text=reply)
    except Exception as error:
        turn_outcome="failed"
        turn_error=str(error)
        if not request.get('_cancelled'):
            emit('reply', conversation_id=request.get('conversation_id', ''), run_id=request.get('run_id'), text=f'Agent error: {error}')
    finally:
        emit('turn', dispatch_id=request.get('dispatch_id'), state='cancelled' if request.get('_cancelled') else turn_outcome, error=turn_error)
        with condition:
            active_request = None
            active_child = None
        if conversation is not None and session:
            sessions[conversation] = session
        if prompt_file is not None:
            prompt_file.close()
        emit('activity', state='online')

"""Read-only discovery and explicit file operations; JSON requests over stdin."""
import base64
import hashlib
import mimetypes
import tempfile
import uuid
import glob
import datetime
import json
import os
import shutil
import sys
import stat
import subprocess
import shlex
import platform
import re

LIMIT = 5 * 1024 * 1024

def artifact_folder(req):
    key = str(uuid.UUID(req['artifact_id']))
    root = os.path.abspath(os.path.expanduser(req.get('cache_root') or os.environ.get('RAIN_ARTIFACT_CACHE') or '~/.local/share/rain/artifacts'))
    folder = os.path.join(root, key)
    os.makedirs(folder, mode=0o700, exist_ok=True)
    return folder

def regular_file(path):
    path = os.path.realpath(os.path.expanduser(path))
    if not stat.S_ISREG(os.stat(path).st_mode):
        raise ValueError('Select a regular file')
    return path

def digest_file(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()

def staged_path(req):
    folder = artifact_folder(req)
    name = os.path.basename(req['name'].replace('\\', '/'))
    if not name or name in ('.', '..'):
        raise ValueError('Invalid filename')
    return os.path.join(folder, name)

def finish_stage(req, destination):
    if os.path.isfile(destination) and os.path.getsize(destination) == req['size_bytes'] and digest_file(destination) == req['sha256']:
        return dict(path=destination, sha256=req['sha256'], size_bytes=req['size_bytes'])
    part = destination + '.part'
    if os.path.getsize(part) != req['size_bytes'] or digest_file(part) != req['sha256']:
        raise ValueError('Transfer verification failed; incomplete file kept for retry')
    os.replace(part, destination)
    return dict(path=destination, sha256=req['sha256'], size_bytes=req['size_bytes'])

def run(req):
    action = req['action']
    if action in ('cli_catalog', 'cli_command'):
        command = req['command']
        provider = req.get('provider', '')
        cwd = os.path.expanduser(req.get('cwd') or '~')
        root = subprocess.run([command, '--help'], cwd=cwd, capture_output=True, text=True, timeout=15)
        help_text = root.stdout + root.stderr
        flag_text = help_text
        if provider in ('codex_chat', 'muse_chat'):
            headless = subprocess.run([command, 'exec', '--help'], cwd=cwd, capture_output=True, text=True, timeout=15)
            flag_text = headless.stdout + headless.stderr
        block = re.split(r'(?:Available subcommands:|Commands:)', help_text, maxsplit=1)
        names = []
        if len(block) > 1:
            section = re.split(r'\n(?:Options:|Arguments:|Safety)', block[1], maxsplit=1)[0]
            names = re.findall(r'^  ([a-z][a-z-]+)\s{2,}', section, re.M)
        permitted = {'models', 'agents', 'agent', 'changelog', 'plugins', 'plugin', 'mcp', 'skills', 'usage', 'model-profile', 'trace', 'export', 'schema'}
        names = [name for name in names if name in permitted]
        efforts = []
        if '--reasoning-effort' in flag_text:
            efforts = ['none','minimal','low','medium','high','xhigh','max','ultra']
        elif '--effort' in flag_text:
            efforts = ['low','medium','high','xhigh','max'] if provider == 'agy_chat' else ['low','medium','high','max']
        if action == 'cli_command':
            if req['subcommand'] not in names:
                raise ValueError('Subcommand is not available in this installed CLI')
            arguments = shlex.split(req.get('arguments') or '')
            # Interactive account/transport setup remains in the native terminal.
            if any(arg in ('login','serve','auth') for arg in arguments):
                raise ValueError('Use Native terminal for interactive login or transport setup')
            result = subprocess.run([command, req['subcommand']] + arguments, cwd=cwd, capture_output=True, text=True, timeout=45)
            output = result.stdout + result.stderr
            if result.returncode:
                raise ValueError(output[-4000:] or 'Native command failed')
            return dict(output=(output[:32000] + ('\n[Output truncated at 32,000 characters; use the native terminal for the full result.]' if len(output)>32000 else '')) or 'Command completed without output')
        return dict(help=help_text[:32000], supports_model='--model' in flag_text, efforts=efforts, commands=names, native_slash='--disable-slash-commands' in flag_text)
    if action == 'resources':
        memory = {}
        try:
            with open('/proc/meminfo') as file:
                memory = {line.split(':')[0]: int(line.split()[1])*1024 for line in file}
        except (OSError, ValueError):
            pass
        return dict(hostname=platform.node(), platform=platform.system(), cpu_cores=os.cpu_count(), ram_total_bytes=memory.get('MemTotal'), ram_available_bytes=memory.get('MemAvailable'), home=os.path.expanduser('~'))
    if action == 'materialize':
        source = regular_file(req['path'])
        if digest_file(source) != req['sha256']:
            raise ValueError('Source hash changed before final delivery')
        destination = os.path.abspath(os.path.expanduser(req['destination']))
        if os.path.isdir(destination):
            destination = os.path.join(destination, os.path.basename(source))
        os.makedirs(os.path.dirname(destination), exist_ok=True)
        temporary = destination + '.rain-' + str(uuid.uuid4())
        with open(source, 'rb') as original, open(temporary, 'xb') as output:
            os.chmod(temporary, 0o600)
            shutil.copyfileobj(original, output, 1024*1024)
            output.flush()
            os.fsync(output.fileno())
        if digest_file(temporary) != req['sha256']:
            os.unlink(temporary)
            raise ValueError('Final delivery hash mismatch')
        os.replace(temporary, destination)
        return dict(path=destination, sha256=req['sha256'])
    if action == 'snapshot':
        source = regular_file(req['path'])
        destination = os.path.join(artifact_folder(req), os.path.basename(source))
        if source != destination:
            with open(source, 'rb') as original, open(destination + '.part', 'wb') as copy:
                os.chmod(destination + '.part', 0o600)
                shutil.copyfileobj(original, copy, 1024 * 1024)
                copy.flush()
                os.fsync(copy.fileno())
            os.replace(destination + '.part', destination)
        return dict(path=destination, name=os.path.basename(source), size_bytes=os.path.getsize(destination), sha256=digest_file(destination), mime_type=mimetypes.guess_type(source)[0] or 'application/octet-stream')
    if action == 'chunk':
        path = regular_file(req['path'])
        offset, length = int(req.get('offset', 0)), int(req.get('length', 1024*1024))
        if offset < 0 or not 0 <= length <= 4*1024*1024:
            raise ValueError('Invalid file range')
        with open(path, 'rb') as file:
            file.seek(offset)
            data = file.read(length)
        return dict(content_base64=base64.b64encode(data).decode('ascii'), sha256=hashlib.sha256(data).hexdigest(), size_bytes=len(data))
    if action == 'stage':
        destination = staged_path(req)
        if os.path.isfile(destination) and os.path.getsize(destination) == req['size_bytes'] and digest_file(destination) == req['sha256']:
            return dict(path=destination, offset=req['size_bytes'], complete=True)
        part = destination + '.part'
        if not os.path.exists(part):
            try:
                with open(part, 'xb') as file:
                    os.chmod(part, 0o600)
            except FileExistsError:
                pass
        # A fully received but corrupt partial is restarted; interrupted valid prefixes resume.
        if os.path.getsize(part) == req['size_bytes'] and digest_file(part) != req['sha256']:
            with open(part, 'wb'):
                pass
        # Resume only at a fully written chunk boundary; a crashed write is overwritten.
        offset = min(os.path.getsize(part), req['size_bytes']) // (1024*1024) * (1024*1024)
        return dict(path=destination, offset=offset, complete=False)
    if action == 'write_chunk':
        destination = staged_path(req)
        data = base64.b64decode(req['content_base64'], validate=True)
        offset = int(req['offset'])
        if len(data) > 1024*1024 or offset < 0 or offset + len(data) > req['size_bytes'] or hashlib.sha256(data).hexdigest() != req['chunk_sha256']:
            raise ValueError('Invalid transfer chunk')
        with open(destination + '.part', 'r+b') as file:
            if offset > os.fstat(file.fileno()).st_size:
                raise ValueError('Transfer would leave a gap')
            file.seek(offset)
            file.write(data)
            file.flush()
            os.fsync(file.fileno())
        return dict(offset=offset+len(data))
    if action == 'finish_stage':
        return finish_stage(req, staged_path(req))
    if action == 'receive_stream':
        destination = staged_path(req)
        remaining = int(req['size_bytes'])
        with open(destination + '.part', 'wb') as file:
            os.chmod(destination + '.part', 0o600)
            while remaining:
                chunk = sys.stdin.buffer.read(min(1024*1024, remaining))
                if not chunk:
                    raise ValueError('Peer stream interrupted')
                file.write(chunk)
                remaining -= len(chunk)
            file.flush()
            os.fsync(file.fileno())
        return finish_stage(req, destination)
    if action == 'peer_copy':
        source = regular_file(req['path'])
        target = req['ssh_target']
        if not target or target.startswith('-') or '\0' in target:
            raise ValueError('Invalid peer target')
        remote = 'python3 -c ' + shlex.quote(req['receiver_code'])
        child = subprocess.Popen([req.get('ssh_binary') or 'ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=5','-T','--',target,remote], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            metadata = {k:req[k] for k in ['artifact_id','name','size_bytes','sha256']}
            metadata['action'] = 'receive_stream'
            child.stdin.write(json.dumps(metadata).encode()+b'\n')
            with open(source, 'rb') as file:
                shutil.copyfileobj(file, child.stdin, 1024*1024)
            child.stdin.close()
            response = child.stdout.read()
            errors = child.stderr.read()
            if child.wait(timeout=300) != 0:
                raise ValueError('Peer SSH copy failed: '+errors.decode(errors='replace')[-500:])
            result = json.loads(response)
            if 'error' in result:
                raise ValueError(result['error'])
            return result['data']
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()
    if action == 'scan':
        paths = os.environ.get('PATH', '').split(os.pathsep)
        paths += [os.path.expanduser(p) for p in ['~/.local/bin', '~/.npm-global/bin', '~/.cargo/bin', '~/bin', '~/.bun/bin']]
        paths += glob.glob(os.path.expanduser('~/.nvm/versions/node/*/bin'))
        paths += [os.path.expanduser('~/.local/share/mise/shims')]
        executables = {}
        for directory in dict.fromkeys(paths):
            if not directory:
                continue
            try:
                for item in os.scandir(directory):
                    if len(executables) >= 5000:
                        break
                    if item.is_file() and os.access(item.path, os.X_OK):
                        executables.setdefault(item.name, os.path.abspath(item.path))
            except OSError:
                continue
        known = {'agy': 'Agy', 'muse': 'Muse', 'codex': 'Codex', 'claude': 'Claude Code', 'gemini': 'Gemini CLI', 'aider': 'Aider', 'opencode': 'OpenCode', 'goose': 'Goose', 'copilot': 'Copilot CLI', 'amp': 'Amp', 'qwen': 'Qwen Code', 'cline': 'Cline', 'kiro-cli': 'Kiro CLI'}
        return {'agents': [{'name': label, 'command': executables[name], 'chat_provider': {'codex':'codex_chat','claude':'claude_chat','agy':'agy_chat','muse':'muse_chat'}.get(name)} for name, label in known.items() if name in executables], 'executables': [{'name': name, 'command': path} for name, path in sorted(executables.items())], 'shell': shutil.which('bash') or shutil.which('sh'), 'home': os.path.expanduser('~')}
    if action == 'import':
        encoded = req.get('content_base64', '')
        if len(encoded) > (LIMIT + 2) // 3 * 4:
            raise ValueError('Attachment exceeds 5 MB limit')
        data = base64.b64decode(encoded, validate=True)
        if len(data) > LIMIT or len(data) != req['size_bytes']:
            raise ValueError('Attachment size mismatch or limit exceeded')
        digest = hashlib.sha256(data).hexdigest()
        if req.get('sha256') and req['sha256'] != digest:
            raise ValueError('Attachment SHA-256 mismatch')
        name = os.path.basename(req['name'].replace('\\', '/'))
        if not name or name in ('.', '..'):
            raise ValueError('Invalid attachment name')
        folder = tempfile.mkdtemp(prefix='rain-inbox-')
        destination = os.path.join(folder, name)
        with open(destination, 'xb') as file:
            os.chmod(destination, 0o600)
            file.write(data)
        return dict(path=destination, sha256=digest, size_bytes=len(data))
    path = os.path.abspath(os.path.expanduser(req.get('path') or '~'))
    if action == 'list':
        entries = []
        parent = os.path.dirname(path)
        if parent != path:
            entries.append(dict(name='..', path=parent, is_dir=True, size_bytes=0, modified_time=None, is_editable=False))
        with os.scandir(path) as iterator:
            for item in iterator:
                if len(entries) >= 10000:
                    raise ValueError('Directory exceeds 10,000 entries; open a smaller directory')
                try:
                    stat = item.stat()
                    entries.append(dict(name=item.name, path=item.path, is_dir=item.is_dir(), size_bytes=stat.st_size, modified_time=datetime.datetime.fromtimestamp(stat.st_mtime, datetime.timezone.utc).isoformat(), is_editable=item.is_file() and stat.st_size <= LIMIT))
                except OSError:
                    continue
        return sorted(entries, key=lambda e: (e['name'] != '..', not e['is_dir'], e['name'].lower()))
    if action == 'export':
        if req.get('workspace_root'):
            root = os.path.realpath(req['workspace_root'])
            path = os.path.realpath(os.path.join(root, req['path']))
            if os.path.commonpath([root, path]) != root:
                raise ValueError('Shared file must stay inside the sending workspace')
            if not os.path.isfile(path):
                raise ValueError('Shared artifact must be a regular file')
        with open(path, 'rb') as file:
            data = file.read(LIMIT + 1)
        if len(data) > LIMIT:
            raise ValueError('Attachment exceeds 5 MB limit')
        return dict(id=str(uuid.uuid4()), name=os.path.basename(path), mime_type=mimetypes.guess_type(path)[0] or 'application/octet-stream', size_bytes=len(data), preview_content=None, data_base64=base64.b64encode(data).decode('ascii'), sha256=hashlib.sha256(data).hexdigest())
    if action == 'read':
        with open(path, 'rb') as file:
            data = file.read(LIMIT + 1)
        if len(data) > LIMIT:
            raise ValueError('File exceeds 5 MB editor limit')
        try:
            content = data.decode('utf-8')
            binary = '\0' in content
        except UnicodeDecodeError:
            content, binary = '', True
        return dict(path=path, content='' if binary else content, size_bytes=len(data), is_binary=binary)
    if action == 'write':
        content = req['content'].encode('utf-8')
        if len(content) > LIMIT:
            raise ValueError('File exceeds 5 MB editor limit')
        # Explicit save follows the selected file's existing permissions.
        with open(path, 'wb') as file:
            file.write(content)
        return 'File saved'
    raise ValueError('Unknown host operation')

try:
    print(json.dumps({'data': run(json.loads(sys.stdin.buffer.readline()))}, ensure_ascii=True))
except Exception as error:
    print(json.dumps({'error': str(error)}))
    sys.exit(1)

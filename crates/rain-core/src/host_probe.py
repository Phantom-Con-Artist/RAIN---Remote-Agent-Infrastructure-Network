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

LIMIT = 5 * 1024 * 1024

def run(req):
    action = req['action']
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
    print(json.dumps({'data': run(json.load(sys.stdin))}, ensure_ascii=True))
except Exception as error:
    print(json.dumps({'error': str(error)}))
    sys.exit(1)

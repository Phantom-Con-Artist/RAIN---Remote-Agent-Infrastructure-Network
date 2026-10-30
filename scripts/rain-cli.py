#!/usr/bin/env python3
"""Small, dependency-free lifecycle commands for a source checkout of RAIN."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import urllib.request
import webbrowser
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = Path(os.environ.get('RAIN_HOME', ROOT / '.rain')).expanduser().resolve()
CONFIG = RUNTIME / 'config.json'
PROCESS = RUNTIME / 'server.json'
LOG = RUNTIME / 'server.log'
BINARY = ROOT / 'target' / 'debug' / ('rain-daemon.exe' if os.name == 'nt' else 'rain-daemon')


def settings():
    if CONFIG.exists():
        return json.loads(CONFIG.read_text())
    return {'port': 7890, 'state_path': str(RUNTIME / 'state.json')}


def initialize(skip_build=False):
    RUNTIME.mkdir(parents=True, exist_ok=True)
    if not CONFIG.exists():
        CONFIG.write_text(json.dumps(settings(), indent=2)+'\n')
    if not skip_build:
        for command in ['cargo', 'node', 'npm']:
            if not shutil.which(command):
                raise RuntimeError(f'{command} is required. Install it, then run rain initialize again.')
        subprocess.run(['cargo', 'build', '-p', 'rain-daemon'], cwd=ROOT, check=True)
        client = ROOT / 'apps' / 'rain-client'
        if not (client / 'node_modules').exists():
            subprocess.run(['npm', 'ci'], cwd=client, check=True)
        subprocess.run(['npm', 'run', 'build'], cwd=client, check=True)
    print(f'RAIN initialized in {RUNTIME}')


def identity(pid):
    if sys.platform.startswith('linux'):
        try:
            fields=Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
            return fields[19] if fields[0]!='Z' else None
        except (OSError, IndexError):
            return None
    try:
        os.kill(pid, 0)
        return str(pid)
    except OSError:
        return None


def process_record():
    try:
        record = json.loads(PROCESS.read_text())
        if identity(record['pid']) == record['identity']:
            return record
    except (OSError, ValueError, KeyError):
        pass
    return None


def url(config):
    return f"http://127.0.0.1:{config['port']}"


def client_address(address=None):
    address = address or settings().get('client_hub')
    if not address:
        raise RuntimeError('Choose your server: rain-client start --hub https://your-private-hub')
    parsed = urlsplit(address)
    if parsed.username or parsed.password or parsed.query or parsed.fragment or parsed.path not in ('', '/'):
        raise RuntimeError('Hub URL must be an origin without credentials, path, query or fragment')
    if not parsed.hostname or parsed.scheme not in ('http', 'https'):
        raise RuntimeError('Hub URL must use HTTPS (HTTP is allowed only on loopback)')
    if parsed.scheme != 'https' and parsed.hostname not in ('localhost', '127.0.0.1', '::1'):
        raise RuntimeError('Remote hubs require HTTPS through your private gateway')
    return address.rstrip('/')


def connect_client(address=None, no_open=False):
    address = client_address(address)
    config = settings()
    config['client_hub'] = address
    RUNTIME.mkdir(parents=True, exist_ok=True)
    CONFIG.write_text(json.dumps(config, indent=2)+'\n')
    print(f'RAIN remote client: {address}')
    if not no_open:
        webbrowser.open(address)


def healthy(config):
    try:
        with urllib.request.urlopen(url(config)+'/health', timeout=1) as response:
            body=json.load(response)
            return response.status==200 and body.get('success') is True and isinstance(body.get('data'),dict) and 'server_time' in body['data']
    except (OSError,ValueError):
        return False


def start(port=None, foreground=False, parent=None, name=None):
    config = settings()
    if port is not None:
        if not 1 <= port <= 65535: raise RuntimeError('Port must be between 1 and 65535')
        config['port'] = port
    if parent is not None: config['parent_hub'] = parent
    if name is not None: config['node_name'] = name
    RUNTIME.mkdir(parents=True, exist_ok=True)
    if process_record():
        record = process_record()
        print(f"RAIN server already running: {record['url']} (PID {record['pid']})")
        return record['url']
    if healthy(config):
        print(f'RAIN is already available at {url(config)}; this command will not replace that process.')
        return url(config)
    if not BINARY.exists() or not (ROOT / 'apps/rain-client/dist/index.html').exists():
        initialize()
    environment = dict(os.environ, RAIN_PORT=str(config['port']), RAIN_STATE_PATH=config['state_path'], RAIN_SPOKE_ID_FILE=str(RUNTIME/'device-id'))
    if config.get('parent_hub'): environment['RAIN_PARENT_HUB'] = config['parent_hub']
    if config.get('node_name'): environment['RAIN_NODE_NAME'] = config['node_name']
    environment.setdefault('RAIN_ADVERTISE_URL',url(config))
    if foreground:
        os.chdir(ROOT)
        os.execve(str(BINARY), [str(BINARY)], environment)
    with LOG.open('ab') as output:
        child = subprocess.Popen([str(BINARY)], cwd=ROOT, env=environment, stdin=subprocess.DEVNULL, stdout=output, stderr=output, start_new_session=os.name!='nt')
    record = {'pid': child.pid, 'identity': identity(child.pid), 'url': url(config), 'port': config['port']}
    PROCESS.write_text(json.dumps(record)+'\n')
    for _ in range(100):
        if child.poll() is not None:
            PROCESS.unlink(missing_ok=True)
            raise RuntimeError(f'Server exited. Inspect {LOG}')
        if healthy(config):
            CONFIG.write_text(json.dumps(config, indent=2)+'\n')
            print(f"RAIN server running: {url(config)} (PID {child.pid})")
            return url(config)
        time.sleep(.1)
    raise RuntimeError(f'Server did not become ready in 10 seconds. Inspect {LOG}; rain-server status shows the process.')


def stop():
    record = process_record()
    if not record:
        print('No RAIN server started by these commands is running.')
        return
    os.kill(record['pid'], signal.SIGTERM)
    for _ in range(100):
        if identity(record['pid']) != record['identity']:
            PROCESS.unlink(missing_ok=True)
            print('RAIN server stopped.')
            return
        # Linux zombie processes no longer serve requests and are waiting for their parent to reap them.
        if sys.platform.startswith('linux'):
            try:
                if Path(f"/proc/{record['pid']}/stat").read_text().rsplit(')',1)[1].split()[0] == 'Z':
                    PROCESS.unlink(missing_ok=True)
                    print('RAIN server stopped.')
                    return
            except OSError:
                pass
        time.sleep(.1)
    raise RuntimeError('Server has not exited; its PID record is retained. Inspect rain-server logs.')


def install(prefix):
    destination = Path(prefix).expanduser().resolve()
    destination.mkdir(parents=True, exist_ok=True)
    names=['rain','rain-server','rain-client']
    for name in names:
        target=destination/name
        if (target.exists() or target.is_symlink()) and not (target.is_symlink() and target.resolve()==ROOT/'bin'/name):
            raise RuntimeError(f'{target} already exists. Choose another --prefix; no file was overwritten.')
    for name in names:
        target=destination/name
        if not target.is_symlink():target.symlink_to(ROOT/'bin'/name)
    print(f'Installed rain, rain-server and rain-client in {destination}')
    print('Next: rain initialize, then rain-client start')
    if str(destination) not in os.environ.get('PATH', '').split(os.pathsep):
        print(f'Add {destination} to PATH to use the command names.')


def monitor(kind, address, watch=False, as_json=False):
    address = (address or url(settings())).rstrip('/')
    def tree(node, prefix='', last=True):
        marker = '[online]' if node['online'] else '[unreachable]'
        print(f"{prefix}{'`-- ' if last else '|-- '}{node['name']} ({node['role']}) {marker}")
        children = node.get('children', [])
        for i, child in enumerate(children):
            tree(child, prefix+('    ' if last else '|   '), i==len(children)-1)
    while True:
        try:
            with urllib.request.urlopen(address+'/api/v1/network/topology', timeout=25) as response:
                body = json.load(response)
            if not body.get('success'):
                raise RuntimeError(body.get('error','Network status unavailable'))
            data = body['data']
            if watch and sys.stdout.isatty():
                print('\033[2J\033[H', end='')
            if as_json:
                print(json.dumps(data if kind!='flow' else {'messages':data.get('message_flows',[]),'files':data['transfers']}, indent=2))
            elif kind=='flow':
                print('RAIN DATAFLOW  |  '+address)
                print('AGENT MESSAGE DELIVERY')
                for turn in data.get('message_flows',[]):
                    print(f"{turn['source_name']} -> {turn['target_name']}  {turn['status']}  {turn['target_machine'] or 'hub'}")
                print('FILE TRANSFERS')
                print('SOURCE -> TARGET        ROUTE        STATE         STAGED / TOTAL')
                for transfer in data['transfers']:
                    print(f"{transfer['source_machine'] or 'hub'} -> {transfer['target_machine'] or 'hub'}  {transfer['route']}  {transfer['status']}  {transfer.get('transferred_bytes',0)} / {transfer['bytes']}")
                    if transfer.get('fallback_reason'): print('  Fallback: '+transfer['fallback_reason'])
                    if transfer.get('error'): print('  Error: '+transfer['error'])
                if not data['transfers']: print('No file transfers have been recorded.')
            else:
                print('RAIN NETWORK  |  '+address+'  |  '+data['model'])
                if data.get('parent_path'): print('Parent path: '+' -> '.join(data['parent_path']))
                tree(data['root'])
                for host in data['execution_hosts']: print(f"SSH {host['alias']}: {host['status']}")
            sys.stdout.flush()
        except (OSError, ValueError) as error:
            if not watch: raise RuntimeError(f'Cannot fetch network state: {error}')
            print(f'Parent/host unavailable; retrying: {error}', file=sys.stderr)
        if not watch: return
        time.sleep(3)


def main():
    role = globals().get('RAIN_COMMAND', Path(sys.argv[0]).name)
    parser = argparse.ArgumentParser(prog=role, description='RAIN client and server lifecycle commands')
    sub = parser.add_subparsers(dest='command', required=True)
    for name in ['initialize', 'init']:
        sub.add_parser(name).add_argument('--skip-build', action='store_true', help='Only initialize configuration')
    launch = sub.add_parser('start')
    launch.add_argument('--port', type=int)
    launch.add_argument('--hub', help='Existing private hub URL (rain-client only)')
    launch.add_argument('--parent', help='Parent hub URL; this server runs as a subhub')
    launch.add_argument('--name', help='Name shown in the topology')
    launch.add_argument('--foreground', action='store_true')
    launch.add_argument('--no-open', action='store_true', help='Print the client URL without opening a browser')
    for name in ['stop', 'status', 'restart', 'logs']:
        sub.add_parser(name)
    setup = sub.add_parser('install')
    setup.add_argument('--prefix', default=str(Path('~/.local/bin').expanduser()))
    for name in ['network','topology','hubs','flow','dataflow']:
        view=sub.add_parser(name)
        view.add_argument('--watch',action='store_true')
        view.add_argument('--json',action='store_true')
        view.add_argument('--url',help='Inspect a dedicated hub or subhub URL')
    spoke = sub.add_parser('spoke')
    spoke.add_argument('--hub', required=True)
    spoke.add_argument('--name', default=os.uname().nodename if hasattr(os, 'uname') else 'RAIN node')
    spoke.add_argument('--pin')
    args = parser.parse_args()
    if not getattr(args,'json',False):
        source=(ROOT/'crates/rain-core/src/branding.rs').read_text()
        banner=source.split('r#\"',1)[1].split('\"#;',1)[0]
        print(banner)
    if role == 'rain-client':
        if args.command != 'start':
            raise RuntimeError('rain-client only connects to a hub. Use rain-server for server lifecycle commands.')
        if args.port or args.parent or args.name or args.foreground:
            raise RuntimeError('Server options belong to rain-server; clients only use --hub and --no-open')
        connect_client(args.hub, args.no_open)
        return
    if args.command in ('network','topology','hubs','flow','dataflow'):
        monitor('flow' if args.command in ('flow','dataflow') else 'network',args.url,args.watch,args.json)
    elif args.command in ('initialize', 'init'):
        initialize(args.skip_build)
    elif args.command == 'install':
        install(args.prefix)
    elif args.command in ('start', 'restart'):
        if args.command == 'restart':
            stop()
        address = start(getattr(args, 'port', None), getattr(args, 'foreground', False),getattr(args,'parent',None),getattr(args,'name',None))
        if role != 'rain-server' and not getattr(args, 'no_open', False):
            if not webbrowser.open(address):
                print(f'Open {address} in your browser.')
    elif args.command == 'stop':
        stop()
    elif args.command == 'status':
        record = process_record()
        print(f"Running: {record['url']} (PID {record['pid']})" if record else f'RAIN available at {url(settings())} (not managed by these commands)' if healthy(settings()) else 'No managed RAIN server is running.')
    elif args.command == 'logs':
        print(LOG.read_text(errors='replace')[-20000:] if LOG.exists() else 'No server log yet.')
    elif args.command == 'spoke':
        if not BINARY.exists():
            initialize()
        RUNTIME.mkdir(parents=True, exist_ok=True)
        arguments = [str(BINARY), '--spoke', '--hub-url', args.hub, '--name', args.name]
        if args.pin:
            arguments += ['--pin', args.pin]
        environment = dict(os.environ, RAIN_SPOKE_ID_FILE=str(RUNTIME/'device-id'))
        os.chdir(ROOT)
        os.execve(str(BINARY), arguments, environment)

if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f'RAIN: {error}', file=sys.stderr)
        sys.exit(1)

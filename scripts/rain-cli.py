#!/usr/bin/env python3
"""Small, dependency-free lifecycle commands for a source checkout of RAIN."""
import argparse
import json
import hashlib
import secrets
import uuid
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
REGISTRY = Path(os.environ.get('RAIN_REGISTRY_PATH',ROOT/'.rain/infrastructures.json')).expanduser().resolve()
def profile_registry():
    if REGISTRY.exists():return json.loads(REGISTRY.read_text())
    return {'active':None,'profiles':{}}
def selected_runtime():
    if os.environ.get('RAIN_HOME'):return Path(os.environ['RAIN_HOME']).expanduser().resolve()
    profiles=profile_registry();active=profiles.get('active')
    return Path(profiles['profiles'][active]['directory']) if active in profiles.get('profiles',{}) else ROOT/'.rain'
RUNTIME = selected_runtime()

CONFIG = RUNTIME / 'config.json'
PROCESS = RUNTIME / 'server.json'
LOG = RUNTIME / 'server.log'
BINARY = ROOT / 'target' / 'debug' / ('rain-daemon.exe' if os.name == 'nt' else 'rain-daemon')


def settings():
    if CONFIG.exists():
        return json.loads(CONFIG.read_text())
    return {'port': 7890, 'state_path': str(RUNTIME / 'state.json')}


def private_json(path,value):
    path.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    temp=path.with_suffix('.tmp')
    fd=os.open(temp,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
    with os.fdopen(fd,'w') as file:json.dump(value,file,indent=2);file.write('\n');file.flush();os.fsync(file.fileno())
    os.replace(temp,path)


def ensure_infrastructure(directory,name=None,key_file=None):
    directory=Path(directory).expanduser().resolve();directory.mkdir(parents=True,exist_ok=True,mode=0o700)
    metadata=directory/'infrastructure.json';key_path=directory/'admin.key'
    if metadata.exists():
        if key_file:raise RuntimeError('This infrastructure already exists. Importing a replacement key requires explicit key rotation in the admin UI.')
        if not key_path.exists():raise RuntimeError('Saved infrastructure is missing its admin key. Restore it from your private backup; initialization will not replace it.')
        return json.loads(metadata.read_text())
    name=name or (input('Name your infrastructure: ').strip() if sys.stdin.isatty() else None)
    if not name:raise RuntimeError('Initialize once with rain initialize --name YOUR_NAME. This creates the infrastructure and its private admin key.')
    if '\n' in name or len(name)>100:raise RuntimeError('Choose a single-line name of at most 100 characters')
    if not key_path.exists():
        key=Path(key_file).expanduser().read_text().strip() if key_file else secrets.token_hex(32)
        if len(key)!=64 or any(c not in '0123456789abcdefABCDEF' for c in key):raise RuntimeError('An imported admin key must be a 64-character hexadecimal RAIN key')
        fd=os.open(key_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
        with os.fdopen(fd,'w') as file:file.write(key);file.flush();os.fsync(file.fileno())
    value={'id':str(uuid.uuid4()),'name':name,'created_at':int(time.time())}
    private_json(metadata,value)
    print(f'Infrastructure saved: {name} ({directory})')
    print(f'Admin key file: {key_path}')
    if sys.stdout.isatty():print('Your sysadmin key (keep private):\n'+key_path.read_text().strip())
    return value


def infrastructures(arguments):
    parser=argparse.ArgumentParser(prog='rain infrastructure')
    commands=parser.add_subparsers(dest='action',required=True)
    commands.add_parser('list')
    create=commands.add_parser('create');create.add_argument('name');create.add_argument('--directory',required=True);create.add_argument('--admin-key-file')
    use=commands.add_parser('use');use.add_argument('name')
    args=parser.parse_args(arguments);registry=profile_registry()
    if args.action=='create':
        if args.name in registry['profiles']:raise RuntimeError('That infrastructure name is already saved')
        directory=str(Path(args.directory).expanduser().resolve());meta=ensure_infrastructure(directory,args.name,args.admin_key_file)
        private_json(Path(directory)/'config.json',{'port':7890,'state_path':str(Path(directory)/'state.json'),'node_name':meta['name']})
        registry['profiles'][args.name]={'id':meta['id'],'directory':directory};registry['active']=args.name;private_json(REGISTRY,registry)
        print('Ready. Start it with rain-server start --share --allow-program python3')
    elif args.action=='use':
        if args.name not in registry['profiles']:raise RuntimeError('Saved infrastructure not found')
        directory=Path(registry['profiles'][args.name]['directory'])
        if not (directory/'infrastructure.json').exists() or not (directory/'admin.key').exists():raise RuntimeError('This saved infrastructure or its key is missing; restore its directory first')
        registry['active']=args.name;private_json(REGISTRY,registry);print('Selected infrastructure: '+args.name)
    else:
        for name,profile in registry['profiles'].items():print(('* ' if registry['active']==name else '  ')+name+'  '+profile['directory'])
        if not registry['profiles']:print('No named profiles. Create one with rain infrastructure create NAME --directory PATH')


def initialize(skip_build=False, name=None, key_file=None):
    ensure_infrastructure(RUNTIME,name,key_file)
    RUNTIME.mkdir(parents=True, exist_ok=True)
    config=settings();config.setdefault('node_name',json.loads((RUNTIME/'infrastructure.json').read_text())['name']);private_json(CONFIG,config)
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
    first_admin_start = not (Path(config['state_path']).parent/'admin.key').exists()
    if not (Path(config['state_path']).parent/'infrastructure.json').exists():ensure_infrastructure(Path(config['state_path']).parent,name)
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
        raise RuntimeError(f'Another hub is already available at {url(config)}. Choose a different --port; saved infrastructure state will not be attached to another server.')
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
            key_path=Path(config['state_path']).parent/'admin.key'
            if first_admin_start:
                print(f'Admin key file: {key_path}')
                if sys.stdout.isatty():
                    print('Your sysadmin key (full control; keep private):')
                    print(key_path.read_text().strip())
                else:
                    print('Show your key in the terminal: rain-server admin-key --show')
            return url(config)
        time.sleep(.1)
    raise RuntimeError(f'Server did not become ready in 10 seconds. Inspect {LOG}; rain-server status shows the process.')


def start_shared_worker(programs):
    record=process_record()
    if not record:raise RuntimeError('Start the managed server before enabling its worker')
    worker=record.get('worker')
    if worker and identity(worker['pid'])==worker['identity']:
        print('Hub resource sharing is already running.')
        return
    environment=dict(os.environ,RAIN_HOME=str(RUNTIME))
    arguments=[sys.executable,str(ROOT/'bin/rain-server'),'share']
    for program in programs:arguments+=['--allow-program',program]
    with (RUNTIME/'worker.log').open('ab') as output:
        child=subprocess.Popen(arguments,cwd=ROOT,env=environment,stdin=subprocess.DEVNULL,stdout=output,stderr=output,start_new_session=os.name!='nt')
    record['worker']={'pid':child.pid,'identity':identity(child.pid)}
    PROCESS.write_text(json.dumps(record)+'\n')
    time.sleep(.2)
    if child.poll() is not None:raise RuntimeError('Resource sharing failed to start. Inspect '+str(RUNTIME/'worker.log'))
    print('Hub resource sharing is active. Manage it from Network → Cloud.')


def stop():
    record = process_record()
    if not record:
        print('No RAIN server started by these commands is running.')
        return
    worker=record.get('worker')
    if worker and identity(worker['pid'])==worker['identity']:
        os.kill(worker['pid'],signal.SIGTERM)
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
    print('Next: rain initialize, rain-server start, then rain-client start --hub http://127.0.0.1:7890')
    if str(destination) not in os.environ.get('PATH', '').split(os.pathsep):
        print(f'Add {destination} to PATH to use the command names.')


def local_admin_cookie(address):
    parsed=urlsplit(address)
    if parsed.hostname not in ('localhost','127.0.0.1','::1'):
        raise RuntimeError('Remote CLI monitoring requires an authenticated admin client; open the HTTPS admin UI.')
    key=(Path(settings()['state_path']).parent/'admin.key').read_text().strip()
    request=urllib.request.Request(address+'/api/v1/admin/login',data=json.dumps({'key':key}).encode(),headers={'Content-Type':'application/json'})
    with urllib.request.urlopen(request,timeout=10) as response:return response.headers['Set-Cookie'].split(';',1)[0]


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
            with urllib.request.urlopen(urllib.request.Request(address+'/api/v1/network/topology',headers={'Cookie':local_admin_cookie(address)}), timeout=25) as response:
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
    if role != 'rain-client' and len(sys.argv)>1 and sys.argv[1]=='infrastructure':
        infrastructures(sys.argv[2:]);return
    if role != 'rain-client' and len(sys.argv)>1 and sys.argv[1]=='admin-key':
        path=Path(settings()['state_path']).parent/'admin.key'
        print(path)
        if '--show' in sys.argv[2:]:
            print(path.read_text().strip())
        else:
            print('Show the key: rain-server admin-key --show. This key grants full sysadmin access.')
        return
    if role != 'rain-client' and len(sys.argv)>1 and sys.argv[1]=='share':
        share = argparse.ArgumentParser(prog=role+' share',description='Share this hub through its outbound task worker')
        share.add_argument('--workspace',default=str(RUNTIME/'shared'))
        share.add_argument('--allow-program',action='append',default=[])
        share.add_argument('--once',action='store_true')
        options=share.parse_args(sys.argv[2:])
        import importlib.util
        spec=importlib.util.spec_from_file_location('rain_worker',ROOT/'scripts/rain-worker.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        address=url(settings())
        worker=module.Worker(address,RUNTIME/'hub-worker',options.workspace,options.allow_program)
        worker.lock_identity()
        if not worker.token:
            request=urllib.request.Request(address+'/api/v1/cloud/invites',data=json.dumps({'name':'This hub'}).encode(),headers={'Content-Type':'application/json'})
            request.add_header('Cookie',local_admin_cookie(address))
            with urllib.request.urlopen(request,timeout=10) as response:invitation=json.load(response)['data']
            worker.enroll(invitation['token'])
        worker.run(options.once)
        return
    # Dedicated scripts keep worker execution and deployment secrets out of the client.
    if role != 'rain-client' and len(sys.argv)>1 and sys.argv[1] in ('worker','cloud-setup','agent'):
        script = {'worker':'rain-worker.py','cloud-setup':'rain-cloud-setup.py','agent':'rain-agent.py'}[sys.argv[1]]
        os.execv(sys.executable,[sys.executable,str(ROOT/'scripts'/script),*sys.argv[2:]])
    parser = argparse.ArgumentParser(prog=role, description='RAIN client and server lifecycle commands')
    sub = parser.add_subparsers(dest='command', required=True)
    for name in ['initialize', 'init']:
        init=sub.add_parser(name)
        init.add_argument('--skip-build', action='store_true', help='Only initialize configuration')
        init.add_argument('--name',help='Name this infrastructure and establish its initial authentication')
        init.add_argument('--admin-key-file',help='Import a private RAIN admin key during first initialization')
    launch = sub.add_parser('start')
    launch.add_argument('--port', type=int)
    launch.add_argument('--share',action='store_true',help='Start a managed local resource worker')
    launch.add_argument('--allow-program',action='append',default=[],help='Explicitly permit a program on the local worker')
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
        if args.port or args.parent or args.name or args.foreground or args.share or args.allow_program:
            raise RuntimeError('Server options belong to rain-server; clients only use --hub and --no-open')
        connect_client(args.hub, args.no_open)
        return
    if args.command in ('network','topology','hubs','flow','dataflow'):
        monitor('flow' if args.command in ('flow','dataflow') else 'network',args.url,args.watch,args.json)
    elif args.command in ('initialize', 'init'):
        initialize(args.skip_build,args.name,args.admin_key_file)
    elif args.command == 'install':
        install(args.prefix)
    elif args.command in ('start', 'restart'):
        if args.command == 'restart':
            stop()
        address = start(getattr(args, 'port', None), getattr(args, 'foreground', False),getattr(args,'parent',None),getattr(args,'name',None))
        if getattr(args,'share',False):start_shared_worker(args.allow_program)
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
    except KeyboardInterrupt:
        pass
    except (RuntimeError, OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f'RAIN: {error}', file=sys.stderr)
        sys.exit(1)

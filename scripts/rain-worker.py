#!/usr/bin/env python3
"""Outbound-only RAIN worker. No SSH daemon, inbound port, or owner password."""
import argparse
import secrets
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import ssl

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,req,fp,code,msg,headers,newurl):return None

class Worker:
    def __init__(self, hub, home, workspace, programs, ca=None, agent_runtime_paths=None):
        self.hub=hub.rstrip('/')
        parsed=urllib.parse.urlsplit(self.hub)
        if parsed.scheme not in ('http','https') or not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment or parsed.path not in ('','/'):
            raise ValueError('Use a hub origin without credentials or paths')
        if parsed.scheme!='https' and parsed.hostname not in ('localhost','127.0.0.1','::1'):
            raise ValueError('Remote workers require HTTPS')
        self.home=Path(home).expanduser().resolve();self.home.mkdir(parents=True,exist_ok=True,mode=0o700)
        self.workspace=Path(workspace).expanduser().resolve();self.workspace.mkdir(parents=True,exist_ok=True)
        self.programs={}
        for program in programs:
            executable=shutil.which(program)
            if not executable: raise ValueError(f'Allowed program is not installed: {program}')
            self.programs[program]=str(Path(executable).resolve())
        self.ca=Path(ca).expanduser().resolve() if ca else None
        self.agent_runtime_paths=[Path(p).expanduser().resolve() for p in (agent_runtime_paths or [])]
        self.context=ssl.create_default_context(cafile=ca)
        self.opener=urllib.request.build_opener(urllib.request.HTTPSHandler(context=self.context),NoRedirect())
        self.owner_path=self.home/'owner.key'
        if not self.owner_path.exists():
            fd=os.open(self.owner_path,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
            with os.fdopen(fd,'w') as f:f.write(secrets.token_hex(32))
        self.owner_key=self.owner_path.read_text().strip()
        self.config_path=self.home/'worker.json';self.journal=self.home/'current-task.json';self.token=''
        if self.config_path.exists():
            config=json.loads(self.config_path.read_text())
            if config['hub']!=self.hub:raise ValueError('This worker identity belongs to another hub; choose another --home')
            self.token=config['token']
    def atomic(self,path,value):
        temp=path.with_suffix('.tmp');fd=os.open(temp,os.O_CREAT|os.O_TRUNC|os.O_WRONLY,0o600)
        with os.fdopen(fd,'w') as f:json.dump(value,f);f.flush();os.fsync(f.fileno())
        os.replace(temp,path)
    def request(self,path,value=None,lease=None,raw=None):
        headers={}
        if self.token:headers['X-Rain-Worker-Token']=self.token
        if lease:headers['X-Rain-Task-Lease']=lease
        if raw is not None:
            headers['Content-Type']='application/octet-stream';data=raw
        elif value is not None:
            headers['Content-Type']='application/json';data=json.dumps(value).encode()
        else:data=None
        req=urllib.request.Request(self.hub+'/api/v1/cloud/worker/'+path,headers=headers,data=data)
        with self.opener.open(req,timeout=10) as response:body=json.load(response)
        if not body.get('success'):raise RuntimeError(body.get('error','Hub rejected request'))
        return body['data']
    def enroll(self,invite):
        if self.token:return
        pending=self.home/'pending-credential'
        if not pending.exists():
            fd=os.open(pending,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
            with os.fdopen(fd,'w') as file:file.write(secrets.token_hex(32))
        candidate=pending.read_text().strip()
        try:
            identity=self.request('enroll',{'invite':invite,'worker_token':candidate,'owner_key_hash':hashlib.sha256(self.owner_key.encode()).hexdigest()})
        except (OSError,RuntimeError):
            # A lost response must not lose the private identity created before enrollment.
            self.token=candidate
            try:identity=self.request('identity');identity['token']=candidate
            except Exception:self.token='';raise
        self.atomic(self.config_path,dict(identity,hub=self.hub));self.token=identity['token']
        pending.unlink(missing_ok=True)
        print('Worker enrolled. Its saved identity will be reused on restart.',flush=True)
    def resources(self):
        memory=None;available=None
        try:
            values={line.split(':')[0]:int(line.split()[1])*1024 for line in Path('/proc/meminfo').read_text().splitlines()}
            memory=values.get('MemTotal');available=values.get('MemAvailable')
        except (OSError,ValueError,IndexError):pass
        disk=shutil.disk_usage(self.workspace)
        return {'hostname':socket.gethostname(),'platform':sys.platform,'cpu_cores':os.cpu_count(),'ram_total_bytes':memory,'ram_available_bytes':available,'disk_free_bytes':disk.free,'disk_total_bytes':disk.total,'load_average':list(os.getloadavg()) if hasattr(os,'getloadavg') else None,'shared_folder':str(self.workspace)}
    def shared(self,path):
        lexical=self.workspace/path
        cursor=lexical
        while cursor!=self.workspace and cursor!=cursor.parent:
            if cursor.is_symlink() or getattr(cursor,'is_junction',lambda:False)():raise ValueError('Symlink paths are not exposed through shared-folder access')
            cursor=cursor.parent
        candidate=lexical.resolve()
        if candidate!=self.workspace and self.workspace not in candidate.parents:raise ValueError('Path must stay inside the shared folder')
        if candidate==self.home or self.home in candidate.parents:raise ValueError('Worker credentials and task state are never exposed through shared folders')
        if candidate in [self.home.parent/'admin.key',self.home.parent/'cloud.json',self.home.parent/'state.json',self.home.parent/'admin-audit.jsonl']:raise ValueError('Infrastructure credentials and records are private to the sysadmin')
        return candidate
    def download(self,task,info,directory):
        name=Path(info['name']).name
        target=directory/(info['id']+'-'+name)
        req=urllib.request.Request(self.hub+f"/api/v1/cloud/worker/tasks/{task['id']}/inputs/{info['id']}",headers={'X-Rain-Worker-Token':self.token,'X-Rain-Task-Lease':task['lease']})
        size=0;digest=hashlib.sha256()
        with self.opener.open(req,timeout=10) as response,target.open('xb') as file:
            while chunk:=response.read(262144):
                size+=len(chunk)
                if size>64*1024*1024:raise ValueError('Input exceeds 64 MB')
                digest.update(chunk);file.write(chunk)
        if size!=info['size_bytes'] or digest.hexdigest()!=info['sha256']:raise ValueError('Input checksum mismatch')
        return target
    def upload(self,task,path):
        if path.is_symlink() or not path.is_file():raise ValueError('Output must be a regular file')
        if path.stat().st_size>64*1024*1024:raise ValueError('Output exceeds 64 MB')
        data=path.read_bytes()
        result=self.request(f"tasks/{task['id']}/outputs?name="+urllib.parse.quote(path.name),lease=task['lease'],raw=data)
        if result['sha256']!=hashlib.sha256(data).hexdigest():raise ValueError('Output checksum mismatch')
        return result
    @staticmethod
    def terminate(child):
        if os.name=='posix':
            try:os.killpg(child.pid,signal.SIGTERM)
            except ProcessLookupError:pass
        elif child.poll() is None:child.terminate()
        try:child.wait(timeout=3)
        except subprocess.TimeoutExpired:
            if os.name=='posix':
                try:os.killpg(child.pid,signal.SIGKILL)
                except ProcessLookupError:pass
            else:child.kill()
            child.wait()
    def execute(self,task,files):
        folder=self.home/'tasks'/task['id'];folder.mkdir(parents=True,exist_ok=True)
        inputs=folder/'inputs';outputs=folder/'outputs';inputs.mkdir(exist_ok=True);outputs.mkdir(exist_ok=True)
        downloaded=[self.download(task,f,inputs) for f in files]
        kind=task['kind']
        if kind=='list_files':
            directory=self.shared(task['path'])
            if not directory.is_dir():raise ValueError('Shared directory not found')
            entries=[];listing_bytes=0;truncated=False
            children=sorted(directory.iterdir(),key=lambda p:p.name)
            for entry in children[:1000]:
                if entry.is_symlink():continue
                try:self.shared(str(entry.relative_to(self.workspace)))
                except ValueError:continue
                if not entry.is_dir() and not entry.is_file():continue
                stat=entry.stat();item={'name':entry.name,'is_dir':entry.is_dir(),'size_bytes':stat.st_size,'path':str(entry.relative_to(self.workspace))};listing_bytes+=len(json.dumps(item).encode())
                if listing_bytes>180000:truncated=True;break
                entries.append(item)
            return {'success':True,'files':entries,'truncated':truncated or len(children)>1000}
        if kind=='fetch_file':
            path=self.shared(task['path']);output=self.upload(task,path)
            return {'success':True,'file_id':output['id']}
        if kind=='put_file':
            target=self.shared(task['path'])
            if target==self.workspace or not target.parent.is_dir():raise ValueError('Choose a filename in an existing shared directory')
            # Refuse overwrites. The owner can choose another filename deliberately.
            with target.open('xb') as output,downloaded[0].open('rb') as source:shutil.copyfileobj(source,output)
            return {'success':True,'path':str(target.relative_to(self.workspace))}
        if kind!='command':raise ValueError('Unknown task kind')
        program=self.programs.get(task['argv'][0])
        if not program:raise ValueError('Program is not allowed by this worker')
        environment={k:v for k,v in os.environ.items() if not k.startswith('RAIN_')}
        environment.update(RAIN_INPUT_DIR=str(inputs),RAIN_OUTPUT_DIR=str(outputs),RAIN_SHARED_DIR=str(self.workspace))
        if task.get('agent_id'):
            if not task.get('agent_token'):raise ValueError('Managed agent credential is unavailable; create a new agent identity')
            environment.update(RAIN_AGENT_TOKEN=task['agent_token'],RAIN_AGENT_ID=task['agent_id'],RAIN_HUB_URL=self.hub)
            if self.ca:environment['SSL_CERT_FILE']=str(self.ca)
        command=[program,*task['argv'][1:]]
        cwd=folder
        if task.get('agent_id'):
            if not sys.platform.startswith('linux') or not shutil.which('bwrap'):
                raise ValueError('Managed agents require Linux bubblewrap isolation on this worker. No unrestricted fallback is allowed.')
            command=['bwrap','--unshare-user','--unshare-pid','--die-with-parent','--cap-drop','ALL','--proc','/proc','--dev','/dev','--tmpfs','/tmp','--bind',str(folder),'/work','--chdir','/work']
            for runtime in ['/usr','/bin','/lib','/lib64','/etc/resolv.conf','/etc/hosts','/etc/nsswitch.conf','/etc/passwd','/etc/ssl/certs','/etc/pki']:
                if Path(runtime).exists():command+=['--ro-bind',runtime,runtime]
            for runtime in self.agent_runtime_paths:
                if not runtime.exists():raise ValueError('An explicitly configured agent runtime path is missing')
                if runtime==self.home or self.home in runtime.parents or runtime in self.home.parents:raise ValueError('Agent runtime mounts cannot expose worker credentials')
                command+=['--ro-bind',str(runtime),str(runtime)]
            if self.ca:command+=['--ro-bind',str(self.ca),'/work/hub-root.crt'];environment['SSL_CERT_FILE']='/work/hub-root.crt'
            command+=['--',program,*task['argv'][1:]]
            environment.update(RAIN_INPUT_DIR='/work/inputs',RAIN_OUTPUT_DIR='/work/outputs',XDG_CONFIG_HOME='/work/config',XDG_CACHE_HOME='/work/cache',XDG_DATA_HOME='/work/data')
            environment.pop('RAIN_SHARED_DIR',None)
        child=None
        with (folder/'stdout').open('wb') as stdout,(folder/'stderr').open('wb') as stderr:
            try:
                child=subprocess.Popen(command,cwd=cwd,env=environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=os.name=='posix')
                deadline=time.monotonic()+task['timeout_seconds'];renewed=time.monotonic();check=time.monotonic()
                while child.poll() is None:
                    if time.monotonic()>deadline:raise TimeoutError('Task exceeded its execution timeout')
                    if stdout.tell()+stderr.tell()>2*1024*1024:raise ValueError('Task output exceeded 2 MB')
                    if time.monotonic()-check>=3:
                        try:self.request(f"tasks/{task['id']}/renew",{},task['lease']);renewed=time.monotonic()
                        except urllib.error.HTTPError as e:
                            if e.code in (400,401,403,409):raise RuntimeError('Task cancelled, worker revoked, or lease interrupted') from e
                        except (OSError,RuntimeError):
                            if time.monotonic()-renewed>20:raise RuntimeError('Lost hub connection; task stopped before lease expiry')
                        check=time.monotonic()
                    time.sleep(.15)
            finally:
                if child:self.terminate(child)
        stdout_text=(folder/'stdout').read_bytes()[:65536].decode(errors='replace');stderr_text=(folder/'stderr').read_bytes()[:65536].decode(errors='replace')
        stdout_text=stdout_text.encode()[:65536].decode(errors='ignore');stderr_text=stderr_text.encode()[:65536].decode(errors='ignore')
        output_files=[p for p in outputs.iterdir() if p.is_file() and not p.is_symlink()]
        if len(output_files)>20:raise ValueError('Task produced more than 20 output files')
        for path in output_files:self.upload(task,path)
        return {'success':child.returncode==0,'exit_code':child.returncode,'stdout':stdout_text,'stderr':stderr_text}
    def lock_identity(self):
        self.identity_lock=os.fdopen(os.open(self.home/'worker.lock',os.O_CREAT|os.O_RDWR,0o600),'r+')
        self.identity_lock.seek(0);self.identity_lock.write('1');self.identity_lock.flush()
        if os.name=='posix':
            import fcntl
            try:fcntl.flock(self.identity_lock.fileno(),fcntl.LOCK_EX|fcntl.LOCK_NB)
            except BlockingIOError:raise RuntimeError('A worker using this saved identity is already running')
        else:
            import msvcrt
            try:msvcrt.locking(self.identity_lock.fileno(),msvcrt.LK_NBLCK,1)
            except OSError:raise RuntimeError('A worker using this saved identity is already running')
    def run(self,once=False):
        def stopping(_signal,_frame):raise KeyboardInterrupt()
        signal.signal(signal.SIGTERM,stopping)
        if not self.token:raise ValueError('Enroll first using --invite or --invite-file')
        delay=1
        while True:
            try:
                if self.journal.exists():
                    current=json.loads(self.journal.read_text());task=current['task']
                    result=current.get('result',{'success':False,'error':'Worker restarted during execution; inspect effects before retrying'})
                    try:self.request(f"tasks/{task['id']}/finish",result,task['lease'])
                    except urllib.error.HTTPError as e:
                        if e.code not in (400,401,403,409):raise
                    self.journal.unlink()
                polled=self.request('poll',{'resources':self.resources(),'programs':list(self.programs)})
                task=polled.get('task');delay=1
                if task:
                    self.atomic(self.journal,{'task':task})
                    try:result=self.execute(task,polled.get('input_files',[]))
                    except Exception as e:result={'success':False,'error':str(e)}
                    self.atomic(self.journal,{'task':task,'result':result})
                    print(f"Task {task['id']}: {'completed' if result['success'] else 'failed'}",flush=True)
                    continue
                if once:return
                time.sleep(2)
            except urllib.error.HTTPError as e:
                if e.code in (401,403):raise RuntimeError('Worker access was revoked or the private connection denied access') from e
                print(f'Hub request failed ({e.code}); reconnecting',flush=True)
                time.sleep(delay);delay=min(30,delay*2)
            except (OSError,RuntimeError) as e:
                print(f'Hub unavailable; reconnecting ({type(e).__name__})',flush=True)
                if once:raise
                time.sleep(delay);delay=min(30,delay*2)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hub',required=True);parser.add_argument('--home',required=True);parser.add_argument('--workspace',required=True)
    parser.add_argument('--invite');parser.add_argument('--invite-file');parser.add_argument('--ca',help='Trusted private hub CA certificate')
    parser.add_argument('--agent-runtime-path',action='append',default=[],help='Explicit read-only runtime/config mount for managed agents; never mount credential/state roots');parser.add_argument('--allow-program',action='append',default=[]);parser.add_argument('--once',action='store_true')
    args=parser.parse_args()
    if args.invite and args.invite_file:parser.error('Use --invite or --invite-file, not both')
    def stopping(_signal,_frame):raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM,stopping)
    worker=Worker(args.hub,args.home,args.workspace,args.allow_program,args.ca,args.agent_runtime_path)
    worker.lock_identity()
    invite=Path(args.invite_file).read_text().strip() if args.invite_file else args.invite
    if invite:worker.enroll(invite)
    worker.run(args.once)
if __name__=='__main__':
    try:main()
    except KeyboardInterrupt:pass
    except (ValueError,RuntimeError,OSError) as error:print(f'RAIN worker: {error}',file=sys.stderr);sys.exit(1)

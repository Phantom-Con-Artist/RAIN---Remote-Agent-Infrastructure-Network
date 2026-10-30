#!/usr/bin/env python3
"""Real hub + two outbound workers: enrollment, scopes, task/file sharing and recovery."""
import hashlib,json,secrets,os,pathlib,socket,subprocess,sys,tempfile,time,urllib.request,urllib.error,uuid
ROOT=pathlib.Path(__file__).resolve().parents[1]
checks=[]
admin_cookie=''
def freeport():
 with socket.socket() as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]
def wait(predicate,label,timeout=45):
 deadline=time.monotonic()+timeout
 while time.monotonic()<deadline:
  try:
   value=predicate()
   if value:return value
  except (OSError,KeyError):pass
  time.sleep(.2)
 raise AssertionError('Timed out: '+label)
def passed(label):checks.append(label);print('PASS '+label,flush=True)
with tempfile.TemporaryDirectory(prefix='rain-cloud-e2e-') as temp:
 d=pathlib.Path(temp);port=freeport();hub=f'http://127.0.0.1:{port}';procs=[];logs=[]
 env={k:v for k,v in os.environ.items() if not k.startswith('RAIN_')};env.update(RAIN_PORT=str(port),RAIN_STATE_PATH=str(d/'hub/state.json'))
 def start_hub():
  global admin_cookie
  log=(d/'hub.log').open('ab');logs.append(log);p=subprocess.Popen([str(ROOT/'target/debug/rain-daemon')],cwd=ROOT,env=env,stdout=log,stderr=log);procs.append(p);wait(lambda:call('/health'),'hub ready');key=(d/'hub/admin.key').read_text().strip();req=urllib.request.Request(hub+'/api/v1/admin/login',data=json.dumps({'key':key}).encode(),headers={'Content-Type':'application/json'});
  with urllib.request.urlopen(req) as response:admin_cookie=response.headers['Set-Cookie'].split(';',1)[0]
  return p
 def request(path,data=None,headers=None,method=None):
  h=dict(headers or {});
  if not any(path.startswith('/api/v1/cloud/'+role+'/') for role in ['worker','agent','owner']) and path!='/api/v1/admin/login':h['Cookie']=admin_cookie
  raw=data if isinstance(data,bytes) else json.dumps(data).encode() if data is not None else None
  if data is not None and not isinstance(data,bytes):h=dict(h,**{'Content-Type':'application/json'})
  req=urllib.request.Request(hub+path,data=raw,headers=h,method=method)
  with urllib.request.urlopen(req,timeout=10) as r:
   content=r.read();return json.loads(content) if 'application/json' in r.headers.get('Content-Type','') else content
 def call(path,data=None,headers=None,method=None):
  value=request(path,data,headers,method)
  if isinstance(value,bytes):return value
  assert value['success'],value
  return value['data']
 def reject(path,data=None,headers=None,method=None,codes=(400,401,403,409)):
  try:call(path,data,headers,method)
  except urllib.error.HTTPError as e:assert e.code in codes,(path,e.code)
  else:raise AssertionError('Unauthorized operation accepted: '+path)
 def submit(title,**kwargs):return call('/api/v1/cloud/tasks',dict(title=title,request_id=str(uuid.uuid4()),**kwargs))
 def task(id):return next(t for t in call('/api/v1/cloud')['tasks'] if t['id']==id)
 def done(id):return wait(lambda:(t if t['status'] in ('completed','failed','cancelled','interrupted') else None) if (t:=task(id)) else None,'task '+id)
 def worker(name):
  invite=call('/api/v1/cloud/invites',{'name':name})
  home=d/name;shared=d/(name+'-shared');shared.mkdir();(shared/'hello.txt').write_text(name+' shared file')
  log=(d/(name+'.log')).open('ab');logs.append(log)
  p=subprocess.Popen([sys.executable,str(ROOT/'scripts/rain-worker.py'),'--hub',hub,'--home',str(home),'--workspace',str(shared),'--invite',invite['token'],'--allow-program',sys.executable],cwd=ROOT,env=env,stdout=log,stderr=log);procs.append(p)
  identity=wait(lambda:json.loads((home/'worker.json').read_text()),name+' enrollment')
  wait(lambda:any(w['id']==identity['worker_id'] and w['online'] for w in call('/api/v1/cloud')['workers']),name+' heartbeat')
  reject('/api/v1/cloud/worker/enroll',{'invite':invite['token']})
  return p,identity,shared
 try:
  server=start_hub()
  for endpoint in ['/api/v1/cloud','/api/v1/fs/list?path=/','/api/v1/conversations','/api/v1/admin/session']:
   try:urllib.request.urlopen(hub+endpoint)
   except urllib.error.HTTPError as e:assert e.code==401
   else:raise AssertionError('Unauthenticated admin endpoint accepted')
  bad=urllib.request.Request(hub+'/api/v1/admin/login',data=b'{"key":"wrong"}',headers={'Content-Type':'application/json'})
  try:urllib.request.urlopen(bad)
  except urllib.error.HTTPError as e:assert e.code==401
  else:raise AssertionError('Wrong admin key accepted')
  passed('Admin key is required for every privileged API; wrong key rejected')
  one,first,shared=worker('studio');two,second,_=worker('laptop');wid=first['worker_id'];wheaders={'X-Rain-Worker-Token':first['token']}
  passed('Single-use enrollment and real resource heartbeats from two workers')
  duplicate=subprocess.run([sys.executable,str(ROOT/'scripts/rain-worker.py'),'--hub',hub,'--home',str(d/'studio'),'--workspace',str(shared),'--once'],env=env,cwd=ROOT,capture_output=True,text=True,timeout=10)
  assert duplicate.returncode!=0 and 'already running' in duplicate.stderr
  recovery=d/'recovered';recovery.mkdir(mode=0o700);credential=secrets.token_hex(32);owner_key=secrets.token_hex(32)
  (recovery/'pending-credential').write_text(credential);(recovery/'owner.key').write_text(owner_key)
  invite=call('/api/v1/cloud/invites',{'name':'Recover lost enrollment'})
  identity=call('/api/v1/cloud/worker/enroll',{'invite':invite['token'],'worker_token':credential,'owner_key_hash':hashlib.sha256(owner_key.encode()).hexdigest()})
  command=[sys.executable,str(ROOT/'scripts/rain-worker.py'),'--hub',hub,'--home',str(recovery),'--workspace',str(d/'recovered-shared'),'--invite',invite['token'],'--once']
  subprocess.run(command,env=env,cwd=ROOT,check=True,stdout=subprocess.DEVNULL,timeout=15)
  assert json.loads((recovery/'worker.json').read_text())['worker_id']==identity['worker_id']
  subprocess.run(command,env=env,cwd=ROOT,check=True,stdout=subprocess.DEVNULL,timeout=15)
  assert credential not in (d/'hub/cloud.json').read_text()
  passed('Lost enrollment response recovers using a pre-saved credential; restart reuses identity; concurrent duplicate worker rejected; hub stores worker credential hashes')
  reject('/api/v1/cloud/worker/poll',{'resources':{},'programs':[]});reject('/api/v1/cloud',headers=wheaders)
  passed('Missing worker tokens denied; worker credentials cannot access owner API')
  content=b'input from phone client\x00'+bytes(range(256))*100
  file=call('/api/v1/cloud/files?name=input.bin',content)
  code="import os,pathlib;src=next(pathlib.Path(os.environ['RAIN_INPUT_DIR']).iterdir());pathlib.Path(os.environ['RAIN_OUTPUT_DIR'],'result.bin').write_bytes(src.read_bytes()+b' processed');print('processed on worker')"
  submitted=submit('Process shared input',kind='command',target=wid,argv=[sys.executable,'-c',code],inputs=[file['id']])
  duplicate=call('/api/v1/cloud/tasks',{'title':'Duplicate request','request_id':submitted['request_id'],'kind':'command','argv':[sys.executable]})
  assert duplicate['id']==submitted['id']
  result=done(submitted['id']);assert result['status']=='completed',result;assert 'processed on worker' in result['result']['stdout'];assert len(result['outputs'])==1
  output=call('/api/v1/artifacts/'+result['outputs'][0]+'/content');assert output==content+b' processed'
  passed('Phone upload → leased worker task → SHA-256 verified output → owner download; duplicate submission deduplicated')
  for kind,path,inputs in [('list_files','',[]),('fetch_file','hello.txt',[]),('put_file','from-phone.bin',[file['id']])]:
   result=done(submit(kind,kind=kind,target=wid,path=path,inputs=inputs)['id']);assert result['status']=='completed',result
   if kind=='fetch_file':assert call('/api/v1/artifacts/'+result['outputs'][0]+'/content')==b'studio shared file'
  assert (shared/'from-phone.bin').read_bytes()==content
  passed('Remote folder listing, file retrieval and sending files into the approved shared folder')
  for relative in ['../outside-secret','/etc/passwd']:
   result=done(submit('Reject escape',kind='fetch_file',path=relative,target=wid)['id']);assert result['status']=='failed',result
  (shared/'escape-link').symlink_to('/etc/passwd');assert done(submit('Reject symlink',kind='fetch_file',path='escape-link',target=wid)['id'])['status']=='failed'
  assert done(submit('Refuse overwrite',kind='put_file',path='from-phone.bin',inputs=[file['id']],target=wid)['id'])['status']=='failed'
  passed('Traversal, absolute-path escapes, external symlinks and unintended file overwrites rejected')
  slow=submit('Cancel remote work',kind='command',target=wid,argv=[sys.executable,'-c','import time;time.sleep(60)'])
  wait(lambda:task(slow['id'])['status']=='running','slow task started');time.sleep(.5)
  owned=next(t for t in json.loads((d/'hub/cloud.json').read_text())['tasks'].values() if t['id']==slow['id'])
  peer_headers={'X-Rain-Worker-Token':second['token'],'X-Rain-Task-Lease':owned['lease']}
  reject(f"/api/v1/cloud/worker/tasks/{slow['id']}/renew",{},peer_headers)
  reject(f"/api/v1/cloud/worker/tasks/{slow['id']}/inputs/{file['id']}",headers=dict(wheaders,**{'X-Rain-Task-Lease':owned['lease']}))
  call('/api/v1/cloud/tasks/'+slow['id']+'/cancel',{},method='POST')
  retry=call('/api/v1/cloud/tasks/'+slow['id']+'/retry',{},method='POST');assert retry['id']!=slow['id'];call('/api/v1/cloud/tasks/'+retry['id']+'/cancel',{},method='POST')
  passed('Cancellation, explicit new-attempt retry, cross-worker lease isolation and task-specific file authorization')
  # Wait until the cancelled command has stopped before another lease is delivered.
  time.sleep(4)
  timed=done(submit('Timeout work',kind='command',target=wid,timeout_seconds=1,argv=[sys.executable,'-c','import time;time.sleep(20)'])['id']);assert timed['status']=='failed',timed
  passed('Worker enforces command timeout and returns a durable failure')
  call('/api/v1/cloud/workers/'+wid+'/pause',{},method='POST');queued=submit('Wait for paused worker',kind='list_files',target=wid);time.sleep(3);assert task(queued['id'])['status']=='queued';call('/api/v1/cloud/workers/'+wid+'/resume',{},method='POST');assert done(queued['id'])['status']=='completed'
  passed('Paused resources stop taking tasks; resume drains queued work')
  running=submit('Interrupt on hub restart',kind='command',target=wid,argv=[sys.executable,'-c','import time;time.sleep(60)']);wait(lambda:task(running['id'])['status']=='running','restart task running')
  server.terminate();server.wait(timeout=15);server=start_hub();assert task(running['id'])['status']=='interrupted';assert any(f['id']==file['id'] for f in call('/api/v1/cloud')['files']);assert call('/api/v1/artifacts/'+file['id']+'/content')==content
  wait(lambda:any(w['id']==wid and w['online'] for w in call('/api/v1/cloud')['workers']),'worker reconnect')
  passed('Hub restart preserves enrollment, files and task history; in-flight work is interrupted rather than replayed')
  managed=call('/api/v1/cloud/agents',{'name':'Isolated agent','worker':wid,'argv':[sys.executable,'-c',f"import os,pathlib;assert not pathlib.Path({str(d/'hub/admin.key')!r}).exists();assert not pathlib.Path({str(d/'studio/worker.json')!r}).exists();assert os.environ.get('RAIN_AGENT_TOKEN');print('isolated agent ready')"]})
  managed_identity=next(a for a in call('/api/v1/cloud/security')['agents'] if a['id']==managed['agent_id'])
  managed_result=done(managed_identity['task_id']);assert managed_result['status']=='completed',managed_result
  assert 'isolated agent ready' in managed_result['result']['stdout'];assert 'agent_token' not in managed_result
  passed('Managed agent executes in an isolated filesystem/process namespace and cannot read hub admin or worker credentials')
  # Distinct capability roles: agents never inherit a sysadmin or worker session.
  a=call('/api/v1/cloud/agents',{'name':'Researcher'});b=call('/api/v1/cloud/agents',{'name':'Reviewer'});outsider=call('/api/v1/cloud/agents',{'name':'Outside group'})
  ah={'X-Rain-Agent-Token':a['token']};bh={'X-Rain-Agent-Token':b['token']};oh={'X-Rain-Agent-Token':outsider['token']}
  group=call('/api/v1/cloud/groups',{'name':'Protected research','members':[a['agent_id'],b['agent_id']]})
  call('/api/v1/cloud/groups/'+group['id']+'/files',{'file':file['id']})
  assert len(call('/api/v1/cloud/agent/groups',headers=ah))==1
  assert call('/api/v1/cloud/agent/groups',headers=oh)==[]
  call('/api/v1/cloud/agent/groups/'+group['id']+'/messages',{'text':'Please review the shared input'},ah)
  assert call('/api/v1/cloud/agent/groups',headers=bh)[0]['messages'][0]['text']=='Please review the shared input'
  reject('/api/v1/cloud/agent/groups/'+group['id']+'/messages',{'text':'intrude'},oh)
  assert call('/api/v1/cloud/agent/groups/'+group['id']+'/files/'+file['id'],headers=ah)==content
  reject('/api/v1/cloud/agent/groups/'+group['id']+'/files/'+file['id'],headers=oh)
  # Agent credentials alone are rejected even when a caller knows private API paths.
  direct=urllib.request.Request(hub+'/api/v1/fs/list?path=/',headers=ah)
  try:urllib.request.urlopen(direct)
  except urllib.error.HTTPError as error:assert error.code==401
  else:raise AssertionError('Agent reached hub file explorer')
  passed('Protected group messaging, member-only read-only file sharing and denial of hub filesystem access')
  guest=call('/api/v1/cloud/agent/guests',{'worker':wid,'path':'','reason':'Inspect shared project files'},ah)
  reject('/api/v1/cloud/agent/guests/'+guest['id']+'/tasks',{'kind':'list_files'},ah)
  owner={'X-Rain-Owner-Id':wid,'X-Rain-Owner-Key':(d/'studio/owner.key').read_text().strip()}
  assert len(call('/api/v1/cloud/owner/guests',headers=owner))==1
  wrong_owner={'X-Rain-Owner-Id':second['worker_id'],'X-Rain-Owner-Key':(d/'laptop/owner.key').read_text().strip()}
  reject('/api/v1/cloud/owner/guests/'+guest['id']+'/approve',{'approved':True},wrong_owner)
  call('/api/v1/cloud/owner/guests/'+guest['id']+'/approve',{'approved':True},owner)
  inspection=call('/api/v1/cloud/agent/guests/'+guest['id']+'/tasks',{'kind':'fetch_file','path':'hello.txt'},ah)
  completed=done(inspection['id']);assert completed['status']=='completed',completed
  assert call('/api/v1/cloud/agent/tasks/'+inspection['id']+'/files/'+completed['outputs'][0],headers=ah)==b'studio shared file'
  reject('/api/v1/cloud/agent/guests/'+guest['id']+'/tasks',{'kind':'put_file','path':'not-allowed'},ah)
  reject('/api/v1/cloud/agent/guests/'+guest['id']+'/tasks',{'kind':'fetch_file','path':'../escape'},ah)
  reject('/api/v1/cloud/agent/tasks/'+inspection['id'],headers=bh)
  call('/api/v1/cloud/guests/'+guest['id']+'/approve',{'approved':False})
  reject('/api/v1/cloud/agent/tasks/'+inspection['id'],headers=ah)
  passed('Machine-owner approval, own-machine-only authority, read-only guest inspection and immediate grant revocation')
  folder=call('/api/v1/cloud/groups/'+group['id']+'/folders',{'worker':wid,'path':''})
  browse=call('/api/v1/cloud/agent/groups/'+group['id']+'/folders/'+folder['id']+'/tasks',{'kind':'list_files'},ah);assert done(browse['id'])['status']=='completed'
  call('/api/v1/cloud/groups/'+group['id']+'/members',{'members':[b['agent_id']]})
  reject('/api/v1/cloud/agent/tasks/'+browse['id'],headers=ah)
  reject('/api/v1/cloud/agent/groups/'+group['id']+'/files/'+file['id'],headers=ah)
  passed('Group folder inspection is read-only and revoking membership removes access to files and completed inspection results')
  call('/api/v1/cloud/agents/'+b['agent_id']+'/revoke',{})
  reject('/api/v1/cloud/agent/groups',headers=bh)
  passed('Revoked agent credentials cannot read group history')
  call('/api/v1/cloud/workers/'+wid+'/revoke',{},method='POST');reject('/api/v1/cloud/worker/poll',{'resources':{},'programs':[]},wheaders)
  passed('Revoked worker cannot poll or claim further work')
  print(json.dumps({'passed':True,'checks':checks},indent=2))
 except Exception:
  for path in d.glob('*.log'):print(path.name+':\n'+path.read_text(errors='replace')[-4000:])
  raise
 finally:
  for p in reversed(procs):
   if p.poll() is None:
    p.terminate()
    try:p.wait(timeout=5)
    except subprocess.TimeoutExpired:p.kill();p.wait()
  for log in logs:log.close()

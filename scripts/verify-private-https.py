"""Real HTTPS + admin key + scoped worker integration. Does not alter host trust/networking."""
import tempfile,pathlib,subprocess,os,socket,time,urllib.request,urllib.error,ssl,json,secrets,shutil,sys,uuid
root=pathlib.Path(__file__).resolve().parents[1]
caddy=os.environ.get('RAIN_CADDY_BINARY') or shutil.which('caddy')
if not caddy:raise SystemExit('Install Caddy or set RAIN_CADDY_BINARY.')
def port():
 with socket.socket() as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]
with tempfile.TemporaryDirectory(prefix='rain-admin-https-') as temp:
 d=pathlib.Path(temp);backend=port();https=port();origin=f'https://localhost:{https}';gateway=secrets.token_hex(32)
 (d/'Caddyfile').write_text((root/'deploy/private-hub/Caddyfile').read_text().replace('127.0.0.1:7890',f'127.0.0.1:{backend}'))
 env=dict(os.environ,RAIN_PRIVATE_HOST=f'localhost:{https}',RAIN_VPN_ADDRESS='127.0.0.1',RAIN_GATEWAY_TOKEN=gateway,RAIN_ALLOWED_ORIGINS=origin,RAIN_PORT=str(backend),RAIN_STATE_PATH=str(d/'state.json'),XDG_DATA_HOME=str(d/'data'),XDG_CONFIG_HOME=str(d/'config'))
 processes=[]
 with (d/'logs').open('w') as log:
  try:
   subprocess.run([caddy,'validate','--config',str(d/'Caddyfile'),'--adapter','caddyfile'],env=env,stdout=log,stderr=log,check=True)
   processes.append(subprocess.Popen([str(root/'target/debug/rain-daemon')],cwd=root,env=env,stdout=log,stderr=log))
   processes.append(subprocess.Popen([caddy,'run','--config',str(d/'Caddyfile'),'--adapter','caddyfile'],env=env,stdout=log,stderr=log))
   ca=d/'data/caddy/pki/authorities/local/root.crt'
   for _ in range(100):
    if ca.exists() and (d/'admin.key').exists():break
    time.sleep(.1)
   context=ssl.create_default_context(cafile=str(ca));key=(d/'admin.key').read_text().strip()
   def request(path='/',headers=None,data=None):
    req=urllib.request.Request(origin+path,headers=dict({'Content-Type':'application/json'},**(headers or {})),data=json.dumps(data).encode() if data is not None else None)
    try:
     with urllib.request.urlopen(req,context=context,timeout=10) as r:return r.status,r.read(),r.headers
    except urllib.error.HTTPError as e:return e.code,e.read(),e.headers
   for _ in range(100):
    try:
     if request('/health')[0]==200:break
     time.sleep(.1)
    except OSError:time.sleep(.1)
   assert request('/')[0]==200 # Public sign-in shell, never private data.
   for path in ['/api/v1/cloud','/api/v1/fs/list?path=/','/api/v1/admin/session','/api/v1/conversations']:
    assert request(path)[0]==401,path
   assert request('/api/v1/admin/login',data={'key':'wrong'})[0]==401
   status,_,headers=request('/api/v1/admin/login',data={'key':key});assert status==200
   assert 'HttpOnly' in headers['Set-Cookie'] and 'Secure' in headers['Set-Cookie'] and 'SameSite=Strict' in headers['Set-Cookie']
   cookie={'Cookie':headers['Set-Cookie'].split(';',1)[0]}
   assert request('/api/v1/cloud',cookie)[0]==200
   assert request('/api/v1/cloud',dict(cookie,**{'X-Rain-Gateway-Token':'forged'}))[0]==200
   assert request('/api/v1/cloud',dict(cookie,Origin='https://evil.example'))[0]==403
   def owner_post(path,data):
    status,raw,_=request(path,cookie,data);assert status==200,(status,raw);return json.loads(raw)['data']
   invite=owner_post('/api/v1/cloud/invites',{'name':'HTTPS worker'})
   job=owner_post('/api/v1/cloud/tasks',{'title':'HTTPS remote task','kind':'command','argv':[sys.executable,'-c',"import os,pathlib;pathlib.Path(os.environ['RAIN_OUTPUT_DIR'],'result.txt').write_text('encrypted worker result')"],'request_id':str(uuid.uuid4())})
   (d/'invite').write_text(invite['token'])
   subprocess.run([sys.executable,str(root/'scripts/rain-worker.py'),'--hub',origin,'--home',str(d/'worker'),'--workspace',str(d/'shared'),'--invite-file',str(d/'invite'),'--allow-program',sys.executable,'--ca',str(ca),'--once'],env=env,stdout=log,stderr=log,check=True,timeout=45)
   cloud=json.loads(request('/api/v1/cloud',cookie)[1])['data'];task=next(t for t in cloud['tasks'] if t['id']==job['id']);assert task['status']=='completed',task
   assert request('/api/v1/artifacts/'+task['outputs'][0]+'/content',cookie)[1]==b'encrypted worker result'
   identity=json.loads((d/'worker/worker.json').read_text());assert request('/api/v1/cloud',{'X-Rain-Worker-Token':identity['token']})[0]==401
   subprocess.run(['node',str(root/'scripts/verify-private-https-browser.mjs')],env=dict(env,RAIN_HTTPS_URL=origin,RAIN_TEST_ADMIN_KEY=key),check=True,timeout=45)
   new=owner_post('/api/v1/admin/rotate-key',{})['key'];assert request('/api/v1/admin/session',cookie)[0]==401;assert request('/api/v1/admin/login',data={'key':key})[0]==401;assert request('/api/v1/admin/login',data={'key':new})[0]==200
   processes[0].terminate();processes[0].wait(timeout=15)
   markers={'X-Rain-Worker-Token':'worker-log-marker-secret','X-Rain-Agent-Token':'agent-log-marker-secret','X-Rain-Owner-Key':'owner-log-marker-secret','X-Rain-Task-Lease':'lease-log-marker-secret'}
   assert request('/api/v1/cloud/worker/poll',markers,{})[0]==502
   log.flush();logged=(d/'logs').read_text()
   assert all(value not in logged for value in markers.values()),'Gateway error log leaked a credential header'
   assert key not in logged and new not in logged
   print(json.dumps({'passed':['Real TLS verified against temporary private CA','Anonymous admin API and wrong admin key denied','HttpOnly Secure SameSite session','Gateway header overwrite and browser-origin enforcement','Worker enrollment, task execution and output return over HTTPS without admin credentials','Browser admin login and dashboard polling','Key rotation invalidates every old session and key','Gateway error logs redact custom credential headers and admin keys']},indent=2))
  except Exception:
   print((d/'logs').read_text()[-5000:]);raise
  finally:
   for child in reversed(processes):
    if child.poll() is None:child.terminate();child.wait(timeout=15)

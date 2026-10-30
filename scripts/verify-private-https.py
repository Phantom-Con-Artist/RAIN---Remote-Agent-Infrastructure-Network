"""Live isolated HTTPS integration check. Set RAIN_CADDY_BINARY and optional browser paths."""
import tempfile,pathlib,subprocess,os,socket,time,urllib.request,urllib.error,ssl,json,base64,secrets,shutil
root=pathlib.Path(__file__).resolve().parents[1]
caddy=os.environ.get('RAIN_CADDY_BINARY') or shutil.which('caddy')
if not caddy: raise SystemExit('Install Caddy or set RAIN_CADDY_BINARY to run this check.')
def freeport():
 with socket.socket() as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]
with tempfile.TemporaryDirectory(prefix='rain-https-') as temp:
 d=pathlib.Path(temp);backend=freeport();port=freeport();origin=f'https://localhost:{port}';token=secrets.token_hex(32);password=secrets.token_hex(16)
 hashed=subprocess.check_output([caddy,'hash-password','--plaintext',password],text=True).strip()
 config=(root/'deploy/private-hub/Caddyfile').read_text().replace('127.0.0.1:7890',f'127.0.0.1:{backend}')
 config='''{
 admin off
 auto_https disable_redirects
 skip_install_trust
 persist_config off
}
'''+config
 (d/'Caddyfile').write_text(config)
 env=dict(os.environ,RAIN_PRIVATE_HOST=f'localhost:{port}',RAIN_VPN_ADDRESS='127.0.0.1',RAIN_OWNER_USER='owner',RAIN_OWNER_PASSWORD_HASH=hashed,RAIN_GATEWAY_TOKEN=token,RAIN_ALLOWED_ORIGINS=origin,RAIN_PORT=str(backend),RAIN_STATE_PATH=str(d/'state.json'),XDG_DATA_HOME=str(d/'data'),XDG_CONFIG_HOME=str(d/'config'))
 procs=[]
 with (d/'logs').open('w') as log:
  try:
   subprocess.run([caddy,'validate','--config',str(d/'Caddyfile'),'--adapter','caddyfile'],env=env,stdout=log,stderr=log,check=True)
   procs.append(subprocess.Popen([str(root/'target/debug/rain-daemon')],cwd=root,env=env,stdout=log,stderr=log))
   procs.append(subprocess.Popen([caddy,'run','--config',str(d/'Caddyfile'),'--adapter','caddyfile'],env=env,stdout=log,stderr=log))
   ca=d/'data/caddy/pki/authorities/local/root.crt'
   for _ in range(100):
    if ca.exists():break
    time.sleep(.1)
   tls=ssl.create_default_context(cafile=str(ca));auth='Basic '+base64.b64encode(f'owner:{password}'.encode()).decode()
   def request(path='/',headers=None):
    try:
     with urllib.request.urlopen(urllib.request.Request(origin+path,headers=headers or {}),context=tls,timeout=5) as r:return r.status,r.read()
    except urllib.error.HTTPError as e:return e.code,e.read()
   for _ in range(100):
    try:request();break
    except OSError:time.sleep(.1)
   for path in ['/','/health','/api/v1/conversations','/api/v1/system/status']:
    assert request(path)[0]==401,path
    assert request(path,{'Authorization':'Basic YmFkOmJhZA=='})[0]==401,path
   for path in ['/','/health','/api/v1/conversations']:
    assert request(path,{'Authorization':auth})[0]==200,path
   assert request('/health',{'Authorization':auth,'X-Rain-Gateway-Token':'forged'})[0]==200
   assert request('/health',{'Authorization':auth,'Origin':'https://evil.example'})[0]==403
   assert request('/health',{'Authorization':auth,'Origin':origin})[0]==200
   subprocess.run(['node',str(root/'scripts/verify-private-https-browser.mjs')],env=dict(env,RAIN_HTTPS_URL=origin,RAIN_TEST_PASSWORD=password),check=True)
   print(json.dumps({'passed':['Caddy deployment config validates','real HTTPS with verified temporary CA','unauthenticated and wrong-password access denied on UI and API','owner can open workspace and API','gateway overwrites forged header','cross-origin request denied','allowed origin accepted'],'caddy':subprocess.check_output([caddy,'version'],text=True).strip()},indent=2))
  except Exception:
   print((d/'logs').read_text()[-5000:]);raise
  finally:
   for p in reversed(procs):p.terminate();p.wait(timeout=10)

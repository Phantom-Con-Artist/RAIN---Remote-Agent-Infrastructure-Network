#!/usr/bin/env python3
"""Persistent infrastructure identity, explicit bootstrap and private-key handling."""
import json,os,pathlib,subprocess,tempfile,hashlib,socket,time
root=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='rain-profile-test-') as temp:
 d=pathlib.Path(temp);env=dict(os.environ,RAIN_HOME=str(d/'default'),RAIN_REGISTRY_PATH=str(d/'registry.json'))
 def cli(*args,environment=env,ok=True):
  result=subprocess.run([str(root/'bin/rain'),*args],env=environment,cwd=root,text=True,capture_output=True,timeout=15)
  assert (result.returncode==0)==ok,(result.stdout,result.stderr);return result
 cli('initialize','--skip-build',ok=False)
 cli('initialize','--skip-build','--name','My private cloud')
 directory=d/'default';key=(directory/'admin.key').read_text();identity=json.loads((directory/'infrastructure.json').read_text());assert len(key)==64
 if os.name=='posix':assert (directory/'admin.key').stat().st_mode&0o077==0
 cli('initialize','--skip-build');assert (directory/'admin.key').read_text()==key;assert json.loads((directory/'infrastructure.json').read_text())==identity
 assert key not in cli('initialize','--skip-build').stdout
 imported=d/'import.key';imported.write_text('ab'*32)
 cli('initialize','--skip-build','--admin-key-file',str(imported),ok=False);assert (directory/'admin.key').read_text()==key
 # Named profiles persist directories/IDs only; keys never enter the registry.
 profiles={k:v for k,v in env.items() if k!='RAIN_HOME'}
 cli('infrastructure','create','studio','--directory',str(d/'studio'),environment=profiles)
 cli('infrastructure','create','travel','--directory',str(d/'travel'),environment=profiles)
 cli('infrastructure','use','studio',environment=profiles)
 registry=json.loads((d/'registry.json').read_text());assert registry['active']=='studio';assert 'admin.key' not in json.dumps(registry)
 original=(d/'studio/admin.key').read_text();cli('initialize','--skip-build',environment=profiles);assert (d/'studio/admin.key').read_text()==original
 with socket.socket() as listener:listener.bind(('127.0.0.1',0));port=listener.getsockname()[1]
 server=[str(root/'bin/rain-server')]
 try:
  started=subprocess.run([*server,'start','--port',str(port),'--share','--allow-program','python3'],env=env,cwd=root,text=True,capture_output=True,timeout=20)
  assert started.returncode==0,(started.stdout,started.stderr)
  assert key not in started.stdout
  for _ in range(60):
   cloud=directory/'cloud.json'
   if cloud.exists() and json.loads(cloud.read_text())['workers']:break
   time.sleep(.1)
  assert json.loads(cloud.read_text())['workers']
  assert key not in (directory/'server.log').read_text(errors='replace')
 finally:
  stopped=subprocess.run([*server,'stop'],env=env,cwd=root,capture_output=True,timeout=20);assert stopped.returncode==0
 assert not (directory/'server.json').exists()
 (directory/'admin.key').unlink();cli('initialize','--skip-build','--name','Do not replace',ok=False)
 assert not (directory/'admin.key').exists()
 print('PASS: explicit first initialization, stable identity/key, private permissions, saved profiles, no secret registry/log output, no silent replacement of missing keys')

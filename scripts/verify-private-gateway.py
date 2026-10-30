import subprocess,tempfile,os,socket,time,urllib.request,urllib.error,json
with tempfile.TemporaryDirectory() as d:
 with socket.socket() as s: s.bind(('127.0.0.1',0)); port=s.getsockname()[1]
 token='a'*64
 env=dict(os.environ,RAIN_PORT=str(port),RAIN_STATE_PATH=d+'/state.json',RAIN_GATEWAY_TOKEN=token,RAIN_ALLOWED_ORIGINS='https://hub.private')
 p=subprocess.Popen(['target/debug/rain-daemon'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 cookie={}
 def request(path,headers=None):
  headers=dict(cookie,**(headers or {}))
  try:
   with urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}'+path,headers=headers),timeout=2) as r: return r.status,json.load(r)
  except urllib.error.HTTPError as e: return e.code,json.load(e)
 try:
  for _ in range(100):
   try: request('/health');break
   except OSError: time.sleep(.05)
  assert request('/health')[0]==200
  assert request('/api/v1/auth/session')[0]==401
  key=open(d+'/admin.key').read().strip()
  with urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}/api/v1/admin/login',data=json.dumps({'key':key}).encode(),headers={'Content-Type':'application/json'})) as response:cookie['Cookie']=response.headers['Set-Cookie'].split(';',1)[0]
  assert request('/api/v1/auth/session')[1]['data']['authenticated'] is False
  assert request('/api/v1/system/status',{'Host':'hub.private'})[0]==403
  assert request('/api/v1/system/status',{'Host':'hub.private','X-Rain-Gateway-Token':token,'Origin':'https://evil.example'})[0]==403
  assert request('/api/v1/system/status',{'Host':'hub.private','X-Rain-Gateway-Token':token,'Origin':'https://hub.private'})[0]==200
  print('PASS: live gateway rejection, authorized access, origin rejection, session status')
 finally: p.terminate();p.wait(timeout=10)

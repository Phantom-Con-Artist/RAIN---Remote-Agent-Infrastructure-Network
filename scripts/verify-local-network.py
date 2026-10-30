#!/usr/bin/env python3
"""Opt-in real localhost hierarchy/CLI/media verification. Starts and stops only its own runtimes."""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error
import uuid

ROOT=Path(__file__).resolve().parents[1]
work=Path(tempfile.mkdtemp(prefix='rain-hybrid-validation-'))
key=str(uuid.uuid4())
nodes=[]
worker=None
report={'directory':str(work),'checks':[]}


def check(name):
    report['checks'].append(name)
    (work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print('PASS '+name,flush=True)


def port():
    with socket.socket() as connection:
        connection.bind(('127.0.0.1',0))
        return connection.getsockname()[1]


def env(name):
    values={k:v for k,v in os.environ.items() if k not in ['RAIN_PARENT_HUB','RAIN_NODE_ID','RAIN_ADVERTISE_URL','RAIN_ARTIFACT_DIR','RAIN_STATE_PATH','RAIN_SSH_HOSTS','RAIN_SSH_BINARY']}
    values.update(RAIN_HOME=str(work/name),RAIN_CLUSTER_KEY=key)
    return values


def command(name,*args):
    result=subprocess.run([str(work/'bin'/'rain-server'),*args],cwd=ROOT,env=env(name),text=True,capture_output=True,timeout=40)
    if result.returncode:raise RuntimeError(result.stdout+result.stderr)
    return result.stdout


def call(base,path,body=None,method=None,token=None):
    headers={'content-type':'application/json'}
    if token:headers['Authorization']='Bearer '+token
    request=urllib.request.Request(base+path,data=json.dumps(body).encode() if body is not None else None,headers=headers,method=method)
    with urllib.request.urlopen(request,timeout=20) as response:
        payload=json.load(response)
    if not payload.get('success'):raise RuntimeError(payload)
    return payload['data']


def wait_for(predicate,description,seconds=40):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        try:
            result=predicate()
            if result:return result
        except (OSError,KeyError,IndexError,ValueError):pass
        time.sleep(.35)
    raise RuntimeError('Timed out: '+description)


def walk(node):
    return [node]+[descendant for child in node['children'] for descendant in walk(child)]

try:
    subprocess.run([str(ROOT/'install.sh'),'--prefix',str(work/'bin')],cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
    check('all three command names installed without changing shell configuration')
    parent=None
    for name in ['root','subhub','subsubhub']:
        number=port()
        arguments=['start','--port',str(number),'--name',name]
        if parent:arguments+=['--parent',parent]
        print('Starting '+name,flush=True)
        address=f'http://127.0.0.1:{number}'
        nodes.append((name,address))
        command(name,*arguments)
        parent=address
    worker_log=(work/'worker.log').open('wb')
    worker=subprocess.Popen([str(work/'bin'/'rain-server'),'spoke','--hub',parent,'--name','worker'],cwd=ROOT,env=env('worker'),stdout=worker_log,stderr=worker_log)
    root=nodes[0][1]
    snapshot=wait_for(lambda:(value if len(walk(value['root']))==4 and all(n['online'] for n in walk(value['root'])) else None) if (value:=call(root,'/api/v1/network/topology')) else None,'four-level live hierarchy')
    assert [n['name'] for n in walk(snapshot['root'])]==['root','subhub','subsubhub','worker']
    check('real root → subhub → subsubhub → worker heartbeats aggregate through all levels')
    root_id=snapshot['root']['id']
    sub=snapshot['root']['children'][0]
    heartbeat={'device_id':sub['id'],'cpu_usage':0,'memory_used_bytes':0,'memory_total_bytes':0,'disk_used_bytes':0,'disk_total_bytes':0,'active_agents_count':0,'uptime_seconds':1,'battery':None,'children':[]}
    try:call(root,'/api/v1/hub/spokes/heartbeat',heartbeat)
    except urllib.error.HTTPError as error:assert error.code==401
    else:raise AssertionError('Unauthenticated heartbeat was accepted')
    token=(work/'subhub'/'device-id.token').read_text().strip()
    heartbeat['children']=[{**snapshot['root'],'children':[]}]
    try:call(root,'/api/v1/hub/spokes/heartbeat',heartbeat,token=token)
    except urllib.error.HTTPError as error:assert error.code==400
    else:raise AssertionError('Cyclic hierarchy was accepted')
    check('heartbeat authentication and hierarchy cycle rejection work over HTTP')
    parsed=json.loads(subprocess.check_output([str(work/'bin'/'rain'),'network','--url',root,'--json'],cwd=ROOT,env=env('root'),text=True))
    assert parsed['root']['id']==root_id
    parsed=json.loads(subprocess.check_output([str(work/'bin'/'rain'),'flow','--url',root,'--json'],cwd=ROOT,env=env('root'),text=True))
    assert 'files' in parsed and 'messages' in parsed
    check('terminal network and dataflow commands return actual JSON without banner contamination')
    output=subprocess.check_output([str(work/'bin'/'rain-client'),'start','--hub',root,'--no-open'],cwd=ROOT,env=env('root'),text=True)
    assert root in output
    check('rain-client connects to the explicitly selected hub and prints its URL')
    # Large binary upload remains a disk-backed reference, supports byte ranges and exact download.
    data=bytes(range(251))*30000
    request=urllib.request.Request(root+'/api/v1/artifacts/upload?name=large-media.bin',data=data,headers={'content-type':'application/octet-stream'},method='POST')
    with urllib.request.urlopen(request,timeout=30) as response:attachment=json.load(response)['data']
    assert attachment['size_bytes']==len(data) and not attachment.get('data_base64')
    content=root+'/api/v1/artifacts/'+attachment['id']+'/content'
    with urllib.request.urlopen(urllib.request.Request(content,headers={'Range':'bytes=1048570-1048600'})) as response:
        assert response.status==206 and response.read()==data[1048570:1048601]
    with urllib.request.urlopen(content) as response:assert response.read()==data
    try:urllib.request.urlopen(urllib.request.Request(content,headers={'Range':f'bytes={len(data)}-'}))
    except urllib.error.HTTPError as error:assert error.code==416
    else:raise AssertionError('Invalid range accepted')
    check('7.5 MB disk-backed file upload, ranged reads, exact streaming download and invalid-range handling')
    # Restart only this test's root; the other hubs continue operating and reconnect.
    command('root','stop')
    assert call(nodes[1][1],'/health')['status']
    command('root','start')
    recovered=wait_for(lambda:(value if value['root']['id']==root_id and len(walk(value['root']))==4 and all(n['online'] for n in walk(value['root'])) else None) if (value:=call(root,'/api/v1/network/topology')) else None,'parent restart recovery')
    check('parent restart preserves root identity and enrollment; children stay alive and reconnect')
    worker_id=walk(recovered['root'])[-1]['id']
    call(nodes[2][1],f'/api/v1/hub/devices/{worker_id}/revoke',method='POST')
    wait_for(lambda:any(n['id']==worker_id and n.get('revoked') and not n['online'] for n in walk(call(root,'/api/v1/network/topology')['root'])),'revocation propagation')
    check('revoked worker remains inspectable but cannot rejoin or advertise live resources')
    report['root_url']=root
    report['passed']=True
except Exception as error:
    report['error']=repr(error)
    raise
finally:
    if worker is not None:
        worker.terminate()
        try:worker.wait(timeout=5)
        except subprocess.TimeoutExpired:worker.kill();worker.wait()
    for name,address in reversed(nodes):
        try:command(name,'stop')
        except Exception as error:report.setdefault('cleanup_errors',[]).append(str(error))
    (work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Validation record: '+str(work/'report.json'),flush=True)

#!/usr/bin/env python3
"""Scoped agent client: protected conversation and explicitly approved read-only inspection."""
import argparse,json,os,ssl,sys,urllib.request,urllib.parse,uuid
from pathlib import Path

class NoRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,req,fp,code,msg,headers,newurl):return None

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--hub',default=os.environ.get('RAIN_HUB_URL'));p.add_argument('--key-file');p.add_argument('--ca',default=os.environ.get('SSL_CERT_FILE'))
 sub=p.add_subparsers(dest='action',required=True);sub.add_parser('groups');sub.add_parser('requests')
 say=sub.add_parser('say');say.add_argument('--group',required=True);say.add_argument('text')
 request=sub.add_parser('request-access');request.add_argument('--worker',required=True);request.add_argument('--path',default='');request.add_argument('--reason',required=True)
 inspect=sub.add_parser('inspect');inspect.add_argument('--grant',required=True);inspect.add_argument('--path',default='');inspect.add_argument('--file',action='store_true')
 task=sub.add_parser('task');task.add_argument('id')
 folder=sub.add_parser('group-folder');folder.add_argument('--group',required=True);folder.add_argument('--folder',required=True);folder.add_argument('--path',default='');folder.add_argument('--file',action='store_true')
 download=sub.add_parser('download');download.add_argument('--group',required=True);download.add_argument('--file',required=True);download.add_argument('--output',required=True)
 a=p.parse_args();token=Path(a.key_file).read_text().strip() if a.key_file else os.environ.get('RAIN_AGENT_TOKEN','')
 if not token or not a.hub:p.error('Use --hub and --key-file, or the managed agent environment')
 origin=urllib.parse.urlsplit(a.hub)
 if origin.scheme not in ('http','https') or origin.username or origin.password or origin.path not in ('','/') or origin.query or origin.fragment or not origin.hostname:p.error('Use a hub origin without credentials or paths')
 if origin.scheme!='https' and origin.hostname not in ('localhost','127.0.0.1','::1'):p.error('Remote agent connections require HTTPS')
 def id(value):return str(uuid.UUID(value))
 body=None
 if a.action=='groups':path='groups'
 elif a.action=='requests':path='guests'
 elif a.action=='say':path=f'groups/{id(a.group)}/messages';body={'text':a.text}
 elif a.action=='request-access':path='guests';body={'worker':id(a.worker),'path':a.path,'reason':a.reason}
 elif a.action=='inspect':path=f'guests/{id(a.grant)}/tasks';body={'kind':'fetch_file' if a.file else 'list_files','path':a.path}
 elif a.action=='task':path='tasks/'+id(a.id)
 elif a.action=='group-folder':path=f'groups/{id(a.group)}/folders/{id(a.folder)}/tasks';body={'kind':'fetch_file' if a.file else 'list_files','path':a.path}
 else:path=f'groups/{id(a.group)}/files/{id(a.file)}'
 headers={'X-Rain-Agent-Token':token,'Content-Type':'application/json'}
 request=urllib.request.Request(a.hub.rstrip('/')+'/api/v1/cloud/agent/'+path,headers=headers,data=json.dumps(body).encode() if body is not None else None)
 opener=urllib.request.build_opener(urllib.request.HTTPSHandler(context=ssl.create_default_context(cafile=a.ca)),NoRedirect())
 with opener.open(request,timeout=15) as r:
  if a.action=='download':
   # Never overwrite another file or permit unrestricted downloads through this helper.
   count=0
   with Path(a.output).open('xb') as file:
    while chunk:=r.read(262144):
     count+=len(chunk)
     if count>64*1024*1024:raise ValueError('Shared file exceeds the supported size')
     file.write(chunk)
   print(json.dumps({'saved':a.output,'bytes':count}))
  else:print(json.dumps(json.load(r),indent=2))
if __name__=='__main__':
 try:main()
 except (ValueError,OSError) as error:print(f'RAIN agent: {error}',file=sys.stderr);sys.exit(1)

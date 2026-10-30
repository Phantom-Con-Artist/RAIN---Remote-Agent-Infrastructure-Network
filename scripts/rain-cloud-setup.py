#!/usr/bin/env python3
"""Prepare a private personal-cloud deployment without changing system networking."""
import argparse
import base64
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import subprocess
import sys
import urllib.parse

ROOT=Path(__file__).resolve().parents[1]
def write_private(path,text):
    fd=os.open(path,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
    with os.fdopen(fd,'w') as f:f.write(text)

def keypair():
    private=subprocess.check_output(['openssl','genpkey','-algorithm','X25519','-outform','DER'],stderr=subprocess.DEVNULL)
    public=subprocess.check_output(['openssl','pkey','-inform','DER','-pubout','-outform','DER'],input=private,stderr=subprocess.DEVNULL)
    return base64.b64encode(private[-32:]).decode(),base64.b64encode(public[-32:]).decode()

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--directory',required=True,help='New directory for private deployment files')
    p.add_argument('--endpoint',help='Your hub public IP/DNS name and forwarded UDP port, e.g. hub.example:51820')
    p.add_argument('--local',action='store_true',help='Prepare a same-Wi-Fi phone connection using this machine’s LAN address')
    p.add_argument('--name',default='My RAIN cloud');p.add_argument('--phone-name',default='phone')
    p.add_argument('--caddy',default=shutil.which('caddy'));p.add_argument('--vpn-network',default='10.77.0.0/24');p.add_argument('--port',type=int,default=7890);p.add_argument('--https-port',type=int,default=8443)
    a=p.parse_args()
    if not a.caddy:p.error('Install Caddy or supply --caddy /path/to/caddy')
    if not shutil.which('openssl'):p.error('OpenSSL is required to generate device keys')
    if a.local and a.endpoint:p.error('Choose --local or --endpoint, not both')
    if a.local:
        with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as route:
            # Choosing a source address does not transmit a packet or consult an outside service.
            route.connect(('192.0.2.1',51820));local_ip=route.getsockname()[0]
        if not ipaddress.ip_address(local_ip).is_private:p.error('Could not find a private LAN address; use --endpoint')
        a.endpoint=f'{local_ip}:51820'
    if not a.endpoint:p.error('Use --local for same-Wi-Fi setup, or --endpoint your-hub:51820 for internet access')
    endpoint=urllib.parse.urlsplit('//'+a.endpoint)
    try:
        if not endpoint.hostname or endpoint.port is None or not 1<=endpoint.port<=65535 or endpoint.path or endpoint.username or endpoint.password or endpoint.query or endpoint.fragment:raise ValueError()
    except ValueError:p.error('Endpoint must be a hostname or IP with a UDP port, without credentials or paths')
    if '\n' in a.name or '\r' in a.name or len(a.name)>100:p.error('Cloud name must be a single line of at most 100 characters')
    if not re.fullmatch(r'[A-Za-z0-9_-]{1,50}',a.phone_name):p.error('Phone name must use letters, digits, underscores or hyphens')
    if not 1<=a.port<=65535 or not 1<=a.https_port<=65535:p.error('Server port must be between 1 and 65535')
    network=ipaddress.ip_network(a.vpn_network)
    if network.version!=4 or not any(network.subnet_of(ipaddress.ip_network(cidr)) for cidr in ['10.0.0.0/8','172.16.0.0/12','192.168.0.0/16']) or not 24<=network.prefixlen<=28:p.error('Choose a private IPv4 subnet between /24 and /28')
    hub=str(network.network_address+1);phone=str(network.network_address+2)
    destination=Path(a.directory).expanduser().resolve()
    if destination.exists():p.error('Choose a new directory; existing secrets will never be overwritten')
    destination.mkdir(parents=True,mode=0o700)
    origin=f'https://{hub}:{a.https_port}'
    private,public=keypair();phone_private,phone_public=keypair();gateway=secrets.token_hex(32)
    write_private(destination/'rain0.conf',f'[Interface]\nPrivateKey = {private}\nAddress = {hub}/{network.prefixlen}\nListenPort = {endpoint.port}\n\n[Peer]\n# {a.phone_name}\nPublicKey = {phone_public}\nAllowedIPs = {phone}/32\n')
    write_private(destination/(a.phone_name+'.conf'),f'[Interface]\nPrivateKey = {phone_private}\nAddress = {phone}/32\n\n[Peer]\nPublicKey = {public}\nEndpoint = {a.endpoint}\nAllowedIPs = {hub}/32\nPersistentKeepalive = 25\n')
    runtime=destination/'runtime';runtime.mkdir(mode=0o700)
    write_private(runtime/'admin.key',secrets.token_hex(32))
    write_private(destination/'hub.env',f'RAIN_GATEWAY_TOKEN={gateway}\nRAIN_ALLOWED_ORIGINS={origin}\nRAIN_PRIVATE_HOST={hub}:{a.https_port}\nRAIN_VPN_ADDRESS={hub}\nRAIN_PORT={a.port}\nRAIN_STATE_PATH={runtime}/state.json\nRAIN_NODE_NAME={a.name}\nRAIN_CLIENT_DIST={ROOT}/apps/rain-client/dist\n')
    config=(ROOT/'deploy/private-hub/Caddyfile').read_text().replace('127.0.0.1:7890',f'127.0.0.1:{a.port}')
    write_private(destination/'Caddyfile',config)
    write_private(destination/'ADMIN-ACCESS.txt',f'Hub: {origin}\nAdmin key file: {runtime}/admin.key\nOpen the hub on a trusted device and choose this key file. Never share it with a worker or agent.\n')
    write_private(destination/'deployment.json',json.dumps({'name':a.name,'hub_url':origin,'endpoint':a.endpoint,'runtime':str(runtime),'phone_profile':a.phone_name+'.conf'},indent=2)+'\n')
    write_private(destination/'start.py', '''import os,subprocess,pathlib,sys
root=pathlib.Path(__file__).resolve().parent
env=dict(os.environ,XDG_DATA_HOME=str(root/'gateway-data'),XDG_CONFIG_HOME=str(root/'gateway-config'))
for line in (root/'hub.env').read_text().splitlines():
 key,value=line.split('=',1);env[key]=value
rain=pathlib.Path(sys.argv[1]) if len(sys.argv)>1 else None
if rain is None:raise SystemExit('Usage: python3 start.py /path/to/rain-daemon')
server=subprocess.Popen([str(rain.resolve())],env=env)
try:subprocess.run([sys.argv[2] if len(sys.argv)>2 else 'caddy','run','--config',str(root/'Caddyfile'),'--adapter','caddyfile'],env=env,check=True)
finally:server.terminate();server.wait(timeout=15)
''')
    if shutil.which('node') and (ROOT/'apps/rain-client/node_modules/qrcode').exists():
        subprocess.run(['node',str(ROOT/'scripts/rain-profile-qr.mjs'),str(destination/(a.phone_name+'.conf')),str(destination/(a.phone_name+'.svg'))],check=True)
    print(f'Private cloud prepared in {destination}')
    print(f'Phone address: {origin}')
    print(f'Admin key file: {runtime}/admin.key')
    if sys.stdout.isatty():
        print('Your sysadmin key (full control; keep private):')
        print((runtime/'admin.key').read_text().strip())
    print('Next: activate rain0.conf on this server, import the phone profile (or scan its SVG QR), and start the hub/gateway.')
    if not a.local:print('For internet access, forward only the WireGuard UDP port to this server.')
    print('Trust the gateway CA on the phone, then open the address and choose runtime/admin.key at the RAIN sign-in screen. Keep all generated files private.')
    print('This command generated configuration only. It did not change your VPN, router, firewall, DNS or certificate trust.')
if __name__=='__main__':
    try:main()
    except (OSError,ValueError,subprocess.CalledProcessError) as e:raise SystemExit(f'RAIN setup: {e}')

# Exact setup: this Fedora hub + phone + another computer

Saved after implementation stopped. UI commit: `08eea2d`. Network/auth commit: `f7e1577`.

This machine is Fedora 44. Its current Wi-Fi address is `192.168.0.6`, interface `wlp2s0`, router `192.168.0.1`. Addresses can change; reserve `192.168.0.6` for this machine in your router's DHCP settings. Start with all three devices on the same Wi-Fi. This guide creates a NEW persistent infrastructure in `~/.rain-personal`. It does not adopt the disposable test hub at port 17903.

The hub and HTTPS gateway run on this machine. The phone runs a browser and WireGuard. The other computer runs WireGuard, Python 3, and RAIN's worker script. WireGuard and Caddy are open-source dependencies you run yourself; there is no hosted transport account or relay. Managed sandboxed agents additionally require Linux and bubblewrap. Basic worker tasks/files do not require a model provider.

## 1. On this hub: install the missing networking dependencies

Open a terminal:

```sh
cd /home/subhradeep/Development/project-rain
sudo dnf install wireguard-tools openssl bubblewrap dnf5-plugins
sudo dnf copr enable @caddy/caddy
sudo dnf install caddy
```

The Caddy installation follows its official Fedora instructions: https://caddyserver.com/docs/install#fedora-redhat-centos . Do not start the generic system Caddy service: the generated launcher below runs Caddy with this infrastructure's private configuration as your user.

The RAIN daemon and frontend have already been built on this machine. To rebuild later:

```sh
cd /home/subhradeep/Development/project-rain
cargo build -p rain-daemon --offline
npm --prefix apps/rain-client ci
npm --prefix apps/rain-client run build
```

## 2. On this hub: generate your persistent infrastructure once

```sh
cd /home/subhradeep/Development/project-rain
python3 scripts/rain-cloud-setup.py \
  --local \
  --name 'My RAIN' \
  --directory "$HOME/.rain-personal" \
  --port 17910 \
  --https-port 8443 \
  --caddy /usr/bin/caddy
```

It prints your admin key in an interactive terminal and saves it at `~/.rain-personal/runtime/admin.key`. This key grants full sysadmin control. Do not put it in chat, a screenshot, Git, or the worker's configuration. Keep the entire infrastructure directory private and backed up. Existing directories are deliberately rejected; do not rerun this generation step on restart.

Your VPN addresses will be:

- Hub: `10.77.0.1`
- Phone: `10.77.0.2`
- Other computer, added below: `10.77.0.3`

If another VPN already uses `10.77.0.0/24`, stop here and choose a different private subnet with `--vpn-network` before generation; adjust all later addresses accordingly.

## 3. On this hub: generate a separate VPN identity for the computer

Run this entire block once. It generates secrets without printing them, writes a private `computer.conf`, and appends that computer's public identity to the generated hub configuration. Do this BEFORE installing/starting rain0 in step 4.

```sh
python3 - <<'PY'
from pathlib import Path
import configparser, os, subprocess
root = Path.home() / '.rain-personal'
computer = root / 'computer.conf'
if computer.exists():
    raise SystemExit('computer.conf already exists; reuse it, do not generate again')
phone = configparser.ConfigParser()
phone.optionxform = str
phone.read(root / 'phone.conf')
private = subprocess.check_output(['wg', 'genkey'], text=True).strip()
public = subprocess.check_output(['wg', 'pubkey'], input=private+'\n', text=True).strip()
text = ('[Interface]\nPrivateKey = '+private+'\nAddress = 10.77.0.3/32\n\n'
        '[Peer]\nPublicKey = '+phone['Peer']['PublicKey']+'\n'
        'Endpoint = '+phone['Peer']['Endpoint']+'\n'
        'AllowedIPs = 10.77.0.1/32\nPersistentKeepalive = 25\n')
fd = os.open(computer, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, 'w') as stream:
    stream.write(text)
with (root / 'rain0.conf').open('a') as stream:
    stream.write('\n[Peer]\n# computer\nPublicKey = '+public+'\nAllowedIPs = 10.77.0.3/32\n')
print('Saved computer.conf and registered its public key in rain0.conf')
PY
```

Each device must use its own profile. Never import the phone profile on both devices.

## 4. On this hub: activate WireGuard and allow the two required ports

These commands use the firewall zone observed on THIS machine. If your active zones have changed, inspect `sudo firewall-cmd --get-active-zones` first and use the zone containing `wlp2s0` for the UDP rule.

```sh
sudo install -d -m 700 /etc/wireguard
sudo install -m 600 "$HOME/.rain-personal/rain0.conf" /etc/wireguard/rain0.conf
sudo firewall-cmd --permanent --zone=FedoraWorkstation --add-port=51820/udp
sudo firewall-cmd --permanent --new-zone=rainvpn
sudo firewall-cmd --permanent --zone=rainvpn --set-target=DROP
sudo firewall-cmd --permanent --zone=rainvpn --add-interface=rain0
sudo firewall-cmd --permanent --zone=rainvpn --add-port=8443/tcp
sudo firewall-cmd --reload
sudo systemctl enable --now wg-quick@rain0
sudo wg show rain0
```

If `rainvpn` already exists, inspect its configuration rather than recreating it. You need only HTTPS 8443 on rain0 and WireGuard UDP 51820 on Wi-Fi. Do not open or router-forward backend port 17910. No IP forwarding between phone and computer is required: both connect to the hub.

WireGuard configuration reference: https://www.wireguard.com/quickstart/ . Firewall reference: https://firewalld.org/documentation/howto/open-a-port-or-service.html .

## 5. On this hub: start RAIN and its private HTTPS gateway

In a terminal you will leave open:

```sh
cd /home/subhradeep/Development/project-rain
python3 "$HOME/.rain-personal/start.py" \
  /home/subhradeep/Development/project-rain/target/debug/rain-daemon \
  /usr/bin/caddy
```

This starts both processes. The private gateway address is:

```text
https://10.77.0.1:8443
```

The first start creates the private certificate authority. In a SECOND terminal:

```sh
cp "$HOME/.rain-personal/gateway-data/caddy/pki/authorities/local/root.crt" \
   "$HOME/.rain-personal/hub-root.crt"
curl --cacert "$HOME/.rain-personal/hub-root.crt" \
  https://10.77.0.1:8443/api/v1/cloud
```

The curl request must fail authentication without an admin session; a JSON rejection means the encrypted gateway is reachable. A connection error means VPN, gateway or firewall configuration needs attention. Do not use `curl -k` as a substitute for trusting the correct certificate.

## 6. On the phone: connect as a remote client

1. Install the official WireGuard application for your phone from https://www.wireguard.com/install/ . No RAIN server, Python, SSH, or worker runs on the phone.
2. On the hub, open `~/.rain-personal/phone.svg` in an image viewer/browser. In WireGuard on the phone choose Add tunnel → Scan QR code. Alternatively transfer `phone.conf` privately over USB and import it. This VPN QR is different from the RAIN UI's address-only QR.
3. Enable that tunnel. Both devices should still be on the same Wi-Fi for this first test.
4. Transfer ONLY `hub-root.crt` to the phone using USB. Install it as a trusted CA certificate. Android: search Settings for “Install a certificate”, select “CA certificate”, then select this file; vendor menu paths differ. iPhone: install the certificate profile, then enable full trust under Settings → General → About → Certificate Trust Settings. Trust only the certificate copied from your own hub. Never transfer the CA's private key.
5. Open `https://10.77.0.1:8443` in your browser. If you see a certificate warning, stop and fix certificate trust; do not bypass it.
6. At RAIN's sign-in screen, enter the admin key printed during step 2. Alternatively privately transfer `runtime/admin.key` by USB and choose IMPORT_ADMIN_KEY. Anyone possessing this key has full sysadmin access.
7. Tap AUTHENTICATE_ADMIN. Bookmark the URL. Infrastructure state stays on the hub; the browser's admin session expires after two hours and is invalidated by hub restart or key rotation.

Do not email or SMS the key. No automatic email/SMS provider is configured.

## 7. On the other computer: connect the VPN

Install WireGuard for that computer's operating system from https://www.wireguard.com/install/ . Transfer `~/.rain-personal/computer.conf` from the hub privately, import it as a tunnel, and activate it. On Linux with wireguard-tools you may instead use `sudo wg-quick up /absolute/path/to/computer.conf`.

Transfer `hub-root.crt` to the computer. Keep it next to the worker script; Python will trust this specific certificate through `--ca`. The computer does NOT need your admin key to contribute resources.

Install Python 3 if absent. Copy `scripts/rain-worker.py` from this checkout to a new folder named `rain-worker` on the computer. The worker script uses Python's standard library; basic tasks do not require installing Node, Rust, or the RAIN desktop app. Put `hub-root.crt` in that folder too. Install Python from https://www.python.org/downloads/ if needed. The commands below use `python3`; on Windows installations exposing `py`, use `py -3` instead.

## 8. From the phone: invite the computer

1. Open the admin dashboard → Network → Cloud.
2. Enter the computer's name in Worker name.
3. Press ISSUE_JOIN_TOKEN.
4. On the COMPUTER, save that exact token into a plain-text file named `invite.txt` in its `rain-worker` folder. Use a trusted local transfer; do not send the admin key. The token expires after five minutes and can be used once. Generate another if it expires.

In a terminal ON THE COMPUTER, change into its `rain-worker` folder, then run:

```sh
python3 rain-worker.py \
  --hub https://10.77.0.1:8443 \
  --home .rain-worker-state \
  --workspace shared \
  --invite-file invite.txt \
  --allow-program python3 \
  --ca hub-root.crt
```

On Windows PowerShell, use a single line and the Windows Python executable name:

```powershell
py -3 rain-worker.py --hub https://10.77.0.1:8443 --home .rain-worker-state --workspace shared --invite-file invite.txt --allow-program python --ca hub-root.crt
```

The allowed program must actually exist on PATH. If it does not, use the absolute Python executable path both in `--allow-program` and in task argument arrays. Leave the worker running. It opens outbound connections only. It stores its credential and separate machine-owner key in `.rain-worker-state`. Keep that directory private. Future starts reuse it and do not need a fresh invitation.

## 9. Verify the full round trip from the phone

1. In Network → Cloud confirm the computer is online.
2. Upload a small text file under Shared files.
3. Under Send a task, select the computer, select the uploaded file as the optional input, and enter a task name.
4. Choose the command action and paste this JSON argument array, using the same Python program you allowed on the worker:

```json
["python3", "-c", "import os,pathlib; p=next(pathlib.Path(os.environ['RAIN_INPUT_DIR']).iterdir()); pathlib.Path(os.environ['RAIN_OUTPUT_DIR'],'result.txt').write_text(p.read_text()+' processed by worker'); print('RAIN round trip complete')"]
```

On the Windows command above, replace the first element with `python`.

5. Press DISPATCH_TASK. Wait for completed. Download `result.txt` and verify it contains the uploaded text plus ` processed by worker`.
6. Create `hello.txt` inside the computer's `shared` folder. Submit a list-files action to that computer; confirm it appears. Fetch it to test remote reads. Only the explicitly shared root is available through scoped file operations.
7. Press PAUSE_SHARING, queue another task, confirm it waits, then cancel it. Press RESUME_SHARING.

An allowlisted ordinary command executes with the worker's OS account permissions. The shared-folder restriction does not sandbox arbitrary sysadmin Python commands. For untrusted managed agents, use the separate managed-agent path on Linux with bubblewrap. Tokens alone do not install an AI model or make an agent autonomous.

## 10. Enable internet/cellular access after same-Wi-Fi succeeds

Your current `--local` profiles point at `192.168.0.6:51820`. That private endpoint works only on the local network.

1. Reserve the hub's Wi-Fi address in your router.
2. Obtain a public reachable address for your home connection from your ISP/router. I cannot infer your public address or CGNAT status from the local test. If the ISP uses CGNAT, ask for a public reachable IP; without any reachable endpoint or relay, unsolicited Internet connections cannot reach this hub.
3. In the router's Port Forwarding page, create exactly this rule: UDP external 51820 → `192.168.0.6`, UDP internal 51820. Router labels vary; do not forward 8443 or 17910.
4. On BOTH phone and computer, edit the WireGuard tunnel's Peer Endpoint from `192.168.0.6:51820` to `YOUR_PUBLIC_IP:51820`, substituting the actual public IP. Keep each device's own keys and all AllowedIPs unchanged. A dynamic public address will need updating when it changes, or your own DNS arrangement.
5. On the phone turn OFF Wi-Fi, enable cellular data and the tunnel, and open the SAME `https://10.77.0.1:8443` URL.
6. Confirm WireGuard on the hub reports a recent handshake: `sudo wg show rain0`. Verify the RAIN page becomes unreachable when the phone tunnel is switched off.

If it works on Wi-Fi but not cellular, the issue is public endpoint, router forwarding, ISP filtering or CGNAT. Do not expose the RAIN backend as a workaround. These WAN steps have not been physically tested on your router or phone.

## Restart, stop and recovery

- Keep the hub awake and connected. This is a single hub, not an automatic failover cluster.
- Ctrl+C in the launcher terminal stops the gateway and daemon. It retains keys/state/files. Rerun step 5 to restart. Do not regenerate the infrastructure.
- WireGuard starts automatically after hub reboot because step 4 enabled its service. The foreground RAIN launcher in this guide must be started again manually; an automatic RAIN service has not been configured.
- Restart the computer worker with the same home, workspace, program and CA arguments; omit `--invite-file invite.txt` after successful enrollment. No new invitation is required unless its credential is revoked.
- Keep an offline private backup of `~/.rain-personal`, including runtime and gateway-data. Never copy those secrets into the repository.
- The local disposable review UI is at `http://127.0.0.1:17903`; it is a different infrastructure and is not a phone-access URL. Its key/state are temporary test data.

Build, real task/file round trips, authentication, scopes and browser UI checks passed locally. Physical phone, other-computer OS integration and Internet routing remain your next tests. Exact light emerald chat contrast is 3.77:1, retained by request; universal AAA is not claimed.

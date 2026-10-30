# RAIN personal cloud

The hub owns identity, permissions, the durable queue, immutable shared-file
snapshots and records. Phones are clients. Workers make outbound HTTPS requests;
no SSH daemon, tunnel, or incoming worker port is used. SSH execution is disabled.

## Create and keep an infrastructure

After installing the build dependencies and linking the commands:

```sh
rain initialize --name 'My cloud'
rain-server start --share --allow-program python3
rain-client start --hub http://127.0.0.1:7890
```

Initialization establishes authentication locally. It generates a high-entropy admin
key, or imports a key with `--admin-key-file /private/backup.key`. The key and
infrastructure identity are saved beside server state and reused on restart.
An interactive terminal shows a new key automatically. Redirected output and the
server log do not receive the key. To locate or display it later:

```sh
rain-server admin-key
rain-server admin-key --show
```

Open the admin UI and choose the key file or enter the key. The browser receives
a two-hour HttpOnly, SameSite=Strict session, marked Secure over HTTPS. The key
is not stored in browser storage. Restarting the hub or rotating the key revokes
sessions. Losing a saved key does not silently reset authentication: restore
its private backup. Keep the infrastructure directory, including keys, artifacts
and state, in a private backup.

Save multiple named infrastructures without rebuilding or recreating their keys:

```sh
rain infrastructure create home --directory ~/.local/share/rain/home
rain infrastructure create studio --directory ~/.local/share/rain/studio
rain infrastructure list
rain infrastructure use home
rain-server start --port 7890 --share --allow-program python3
```

`RAIN_HOME` explicitly overrides the selected profile. Concurrent infrastructures
need distinct ports; RAIN refuses to attach a profile to a different hub already
using its port. The browser remembers successfully authenticated infrastructure
names and URLs; choosing one navigates to that hub and its own authentication.
The saved-address list contains no admin keys.

## Add a worker

In **Network → Cloud**, create a one-use invitation and save it as `invite.txt`
on the target computer. From its RAIN checkout, with Python 3 installed:

```sh
rain worker --hub https://YOUR-PRIVATE-HUB \
  --home .rain-worker --workspace ./shared \
  --invite-file invite.txt --allow-program python3 \
  --ca /path/to/hub-root.crt
```

Omit `--ca` only when the hub uses a certificate trusted by that machine. An
invitation expires after five minutes. The worker stores a revocable credential
and an independent machine-owner key in its private home. Programs must be
explicitly allowed on the worker. The default shared folder is not the hub's
filesystem. Sharing pauses new tasks; revocation cancels work and disables
managed-agent credentials on that worker.

Workers execute one task at a time. They advertise actual CPU, available memory
and free disk; this is execution placement, not transparent pooling of RAM or
GPUs. Tasks accept argument arrays, input files and a timeout. Results and output
files return through the hub. Files are bounded to 64 MB and verified with
SHA-256. Shared-folder sends refuse overwrites. Guest operations cannot execute
commands or write files.

## Admin dashboard and agents

The interactive tree shows the hub, workers, managed agents, protected groups
and group membership. Nodes support keyboard selection, panning and zoom.
Moving paths represent reported running tasks or recent group messages; they
are not fabricated packet-level progress. Motion stops for stale/hidden views
and respects reduced-motion preferences.

The sysadmin can launch/stop managed agents, cancel/retry tasks, pause/revoke
workers, issue/revoke agent credentials, change membership and read records.
The admin Files/terminal views control the hub with the server account's OS
permissions. Worker programs run with the worker account's OS permissions.
They do not acquire root/sudo authority automatically.

Managed agent tasks require Linux bubblewrap. They receive their own scoped
agent credential in an isolated filesystem/process namespace, with their task
inputs/outputs and read-only system runtime. Worker keys, admin keys and the
worker's shared root are not mounted. An unavailable sandbox fails closed.
For a CLI outside system runtime directories, the machine owner must explicitly
add read-only `--agent-runtime-path` mounts for its runtime/configuration.
Never mount a credential/state root. Ordinary sysadmin command tasks deliberately
have the worker account's full permissions and must not be used as an untrusted
agent sandbox. Local legacy agent sessions on the hub also retain their existing
OS-account permissions; isolation is enforced for **managed cloud agents**.

## Protected groups and guest inspection

Agent credentials cannot use admin APIs or enumerate the hub's filesystem.
An agent sees only its groups, their messages, and explicitly shared files or
folders. Group sharing is read-only. Files are immutable snapshots. Folder
reads use scoped worker tasks and cannot follow symlinks or escape the approved
folder. Removing membership or unsharing removes access to related results.

A guest request names a machine, relative folder and reason. Either the sysadmin
or that machine's owner can approve it. The owner's separate page is `#owner`;
its key cannot open the admin dashboard or approve another machine's requests.
Guest grants last 15 minutes, remain read-only and can be revoked immediately.

Agents can use the scoped API directly or the Python helper:

```sh
rain agent --hub https://YOUR-HUB --key-file rain-agent.key --ca hub-root.crt groups
rain agent --hub https://YOUR-HUB --key-file rain-agent.key --ca hub-root.crt say --group GROUP_UUID 'Please review the shared results'
```

Managed agents receive `RAIN_HUB_URL`, `RAIN_AGENT_ID`, `RAIN_AGENT_TOKEN`,
`RAIN_INPUT_DIR` and `RAIN_OUTPUT_DIR`. Their helper can use this environment
instead of an on-disk credential. It supports group messaging, approved file
downloads, guest requests and read-only inspection tasks. Credentials alone do
not make a model autonomous; the installed CLI/agent must use these tools.

## Connect a phone or another network

Prepare the hub's private endpoint and per-device WireGuard profile:

```sh
rain cloud-setup --local --directory ~/.rain-cloud --caddy /path/to/caddy
# Or use --endpoint YOUR-PUBLIC-HUB:51820 for a directly reachable internet hub.
```

Generated files are private: hub/phone VPN configurations, a local phone-profile
QR, a gateway configuration, a persistent admin key and a launcher. Activate the
hub's `rain0.conf` using WireGuard, import the phone profile, start the generated
launcher, and trust its private CA on the phone. The generated HTTPS endpoint
uses port 8443 so the gateway does not require a privileged TCP bind. Open it in
the phone browser and authenticate with the admin key. No RAIN server or worker
runs on the phone.

This preparer does not change system networking, install dependencies, forward
router ports or install certificate trust automatically. Direct internet access
requires a reachable UDP endpoint; CGNAT cannot be bypassed without a reachable
entry point. The default contains no outside relay or coordination service.
The application flow is create hub → invite machines → open client; unconfigured
routers and private certificate trust still require one-time network setup.

TLS encrypts remote API/file/message traffic. WireGuard protects private network
membership. TLS terminates at the trusted hub so the sysadmin can inspect records;
this is not end-to-end content encryption that hides data from the hub. Email/SMS
key delivery is not configured. The default needs no delivery provider and never
pretends to have sent a key or verification message.

## Failure and scale behavior

Worker leases expire after 30 seconds. Lost connections, cancellations and
revocations stop execution at the next lease check. Commands stop after prolonged
loss of the hub; completed file operations cannot be rolled back. Hub restarts
retain files, enrollment and history and mark in-flight tasks interrupted. Work
is never automatically replayed. Explicit retries create new attempts because
commands can have external effects.

Heartbeat telemetry and lease renewals stay in memory; task claims and permission
changes are saved before acknowledgement. The dashboard polls bounded recent
history, with complete task records and paginated older records available on
request. Group history is paginated; audit records are retained in
`admin-audit.jsonl`, with the UI reading the newest 500 at a time. This is a
single-hub private deployment tested with two workers, not an HA cluster or a
large-scale capacity certification.

Successful routine keepalives are not appended as individual audit events;
failed access attempts and control mutations are. Browser and Python credential
clients reject redirects so custom role headers cannot be forwarded to another
origin. Gateway header redaction uses Caddy's [logging filters](https://caddyserver.com/docs/caddyfile/directives/log).

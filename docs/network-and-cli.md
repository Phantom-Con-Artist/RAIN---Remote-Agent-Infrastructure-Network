# Running and monitoring RAIN

From the checkout:

```sh
./install.sh
rain initialize
rain-server start
rain-client start --hub http://127.0.0.1:7890
```

The installer links `rain`, `rain-server`, and `rain-client` into `~/.local/bin` (or `--prefix /your/bin`). It refuses to overwrite other commands. Keep the checkout at its installed location and ensure the target bin directory is on PATH. `initialize` builds the backend and browser client. Lifecycle commands track only their own processes, including process identity to guard against PID reuse. An unrelated daemon is never replaced.

```sh
rain-server status
rain-server logs
rain-server restart
rain-server stop
rain-client start --hub http://127.0.0.1:7890 --no-open
rain network --watch
rain flow --watch
rain network --json --url http://127.0.0.1:7890
```

Default state lives in `.rain`; `RAIN_HOME=/path` isolates another runtime. Start a subhub with its own port/state and a reachable parent:

```sh
RAIN_HOME=/path/subhub rain-server start --port 7891 --parent http://parent:7890 --name Studio
RAIN_HOME=/path/worker rain-server spoke --hub http://subhub:7891 --name Worker
```

For another subhub beneath Studio, use Studio's URL as its `--parent`. Set `RAIN_CLUSTER_KEY` consistently for unattended enrollment, or use the pairing flow. Heartbeats require the enrolled node's bearer token. Stable IDs and enrollment state persist across restarts. The parent learns recursive child topology; cycle/duplicate identities are rejected. Stale ancestors make descendant resources unavailable. Revoked nodes remain inspectable but cannot re-enroll under that identity. Reconnect uses bounded exponential backoff.

**Network → Topology** drills into any registered hub and its descendants, with live status, CPU/RAM and optional endpoint. **Dataflow** shows message dispatches and verified file transfers, including errors, direct/fallback routes and retry controls. **Connections** retains pairing and execution-host setup. Probes report actual configured SSH availability. Unknown GPU/speed values remain unknown.

The topology combines a hierarchical star control plane with peer file transfer. It is not distributed consensus or transparent migration of arbitrary running work. A child hub keeps its own service while its parent is unavailable and reconnects afterward; the parent excludes stale resources. Physical SSH network behavior requires testing on configured hosts. Keep the existing authenticated tunnel/access arrangement for remote access; new routes do not introduce a separate internet-facing security boundary.

Private internet access uses a user-owned hub and remote-only clients. See [self-hosted WireGuard and HTTPS gateway](../deploy/private-hub/README.md). `rain-client` never starts or stops a server. Remote client URLs require HTTPS.

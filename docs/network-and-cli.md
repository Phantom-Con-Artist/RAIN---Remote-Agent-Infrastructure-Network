# RAIN network and CLI

See [Personal cloud](personal-cloud.md) for current setup, authentication,
resource sharing, files, groups, guest access and failure behavior.

```sh
rain initialize --name 'My cloud'
rain-server start --share --allow-program python3
rain-client start --hub http://127.0.0.1:7890
rain-server admin-key --show
rain network --watch
rain flow --watch
rain-server status
rain-server logs
rain-server stop
```

The server owns state and execution. `rain-client` opens an existing hub and
never starts or stops a server. Remote browser clients use the hub's HTTPS
address. HTTP is permitted only on loopback. Cloud workers use outbound
HTTPS, scoped credentials and leases; SSH execution is disabled. Legacy
spoke/subhub endpoints are not the personal-cloud worker enrollment path.

Install links with `./install.sh`; keep the checkout in that location. Default
state lives in `.rain`; `RAIN_HOME=/path` selects an explicit runtime. Named
infrastructure profiles preserve identities and keys. Lifecycle commands track
their own process identities and refuse to replace another hub using the port.

Core verification entry points:

```sh
python3 scripts/verify-infrastructure.py
python3 scripts/verify-cloud.py
python3 scripts/verify-private-gateway.py
python3 scripts/verify-private-https.py
node scripts/verify-cloud-ui.mjs
node scripts/verify-admin-ui.mjs
```

Browser tests require Playwright and a disposable review hub; HTTPS checks also
require Caddy. See [validation notes](private-hub-validation.md).

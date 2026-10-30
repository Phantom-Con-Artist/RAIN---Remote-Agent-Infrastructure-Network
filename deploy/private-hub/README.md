# Private gateway deployment

Read [Personal cloud](../../docs/personal-cloud.md) first. The adjacent Caddyfile
terminates HTTPS and forwards only to the hub's loopback listener. RAIN itself
checks the sysadmin session or scoped worker/agent/machine-owner credential.
There is no shared gateway password that gives workers administrator access.

Set `RAIN_GATEWAY_TOKEN` to the same generated random value in the gateway and
hub service environments, plus `RAIN_PRIVATE_HOST`, `RAIN_VPN_ADDRESS` and
`RAIN_ALLOWED_ORIGINS`. Caddy overwrites the internal gateway header. Never
provide that internal secret to workers or agents. Bind the gateway to the
owner-operated VPN address; do not expose the daemon's TCP port publicly.

`rain cloud-setup --local --directory /new/private/directory --caddy /path/to/caddy`
generates private per-device WireGuard profiles and QR, an admin key, Caddy
configuration and a launcher. An internet endpoint can be supplied with
`--endpoint YOUR-HUB:51820`. Generated configuration does not activate VPNs,
modify router/firewall policy or install certificate trust. The default HTTPS
port is 8443. The private CA certificate appears under the generated
`gateway-data/caddy/pki/authorities/local/root.crt` after startup.

Admin keys and infrastructure identities persist in the runtime directory.
Remote login is performed in the RAIN UI. Email/SMS delivery is not configured.
Public sign-in assets contain no infrastructure records; every privileged API
requires authentication. Gateway admin access and automatic HTTP redirects are
disabled; automatic root trust installation is disabled.

Validate on the actual deployment: without VPN the service must be unreachable;
with VPN but without the admin key the admin API must reject access; approved
worker tokens must reach only worker endpoints; revoked tokens must fail. Test
on cellular separately. WAN/CGNAT behavior and physical-device trust cannot be
established by local HTTPS tests.

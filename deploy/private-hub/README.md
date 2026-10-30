# Self-hosted private hub

The owner's machine is the authoritative server: it holds state, runs agents,
serves the client, and routes work. Phones and browsers are remote controls;
they never enroll as workers or take over hub duties. An unavailable hub means
unavailable remote control, not automatic promotion of a phone. Back up the
hub's state and artifact directories; arbitrary running processes do not fail over.

Transport is independent of RAIN. Use owner-operated WireGuard, with either a
reachable UDP port on the hub or an owner-operated VPS routing VPN peers to it.
No third-party coordination or relay service is required. A CGNAT-only setup
without a reachable peer cannot provide reliable unsolicited internet access.

1. Configure WireGuard on the hub and phone. Give each device its own key and
   restrict routes and firewall rules to the hub's VPN address. Do not enable
   peer-to-peer forwarding for phones. With a VPS, allow phone-to-hub forwarding
   only, and use persistent keepalive for the hub behind NAT. The owner controls
   keys, revocation, IP assignments and firewall policy.
2. Build with `rain initialize`; start with `rain-server start`. RAIN binds only
   `127.0.0.1`; never forward its TCP port from a router or public proxy.
3. Install Caddy on that same hub. Set `RAIN_PRIVATE_HOST` to the private DNS name,
   `RAIN_VPN_ADDRESS` to the hub's WireGuard address, `RAIN_OWNER_USER`, and
   `RAIN_OWNER_PASSWORD_HASH` (generated with `caddy hash-password`). Generate a
   random gateway secret with `openssl rand -hex 32`; supply it as
   `RAIN_GATEWAY_TOKEN` to both Caddy and rain-server before starting them.
   Set `RAIN_ALLOWED_ORIGINS=https://your-private-host` on rain-server.
   Keep secrets in owner-readable service environment files, never in URLs.
4. Run Caddy with the adjacent Caddyfile. Its internal CA needs explicit trust
   on each client; install its root certificate through the platform's supported
   trust workflow. Alternatively configure your own trusted TLS certificate.
5. Connect the phone's VPN, open `https://your-private-host`, and authenticate.
   On a desktop use `rain-client start --hub https://your-private-host`.
   The gateway overwrites its secret header after authentication. Browsers never
   receive that secret. A RAIN device pairing token does not bypass the gateway.

The gateway grants its authenticated owner full hub control, including process
execution and file access. This is a single-owner boundary, not multi-user RBAC.
The hub OS and its local processes are trusted. Do not place an untrusted local
user or unauthenticated proxy inside that boundary.

Validation on the owner's deployment is required: VPN disconnected must fail;
VPN connected with wrong credentials must fail; correct credentials must work;
revoked VPN peers must fail; the public interface must not accept TCP 7890 or
443. Restrict the WireGuard UDP listener to its intended interface/port. Do not
expose Caddy's admin API. Existing SSH execution hosts remain optional legacy
workers; SSH is not part of this client connection path.

A phone switching Wi-Fi/cellular reconnects through WireGuard to the same hub.
The hub retains work and history independently of the client. Network retry is
not a guarantee of exactly-once mutation: refresh state before repeating a launch
whose response was lost. Hub outage recovery uses restart and persisted state;
this configuration does not implement redundant hubs or automatic failover.

Product direction: direct phone-to-owner-hub access is the default. The VPS
variant above is optional and must never be provisioned silently. The current
configuration is a deployment foundation, not a completed automatic setup wizard.
Automatic reachability checks, router configuration guidance and phone enrollment
still need implementation before setup can be described as effortless.

References: [WireGuard quick start](https://www.wireguard.com/quickstart/),
[Caddy reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy),
[Caddy authentication](https://caddyserver.com/docs/caddyfile/directives/basic_auth).

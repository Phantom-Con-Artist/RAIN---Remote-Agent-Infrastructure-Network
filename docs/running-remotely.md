# Running RAIN remotely

Run the host daemon on the computer containing your CLI tools and workspaces:

```sh
cargo run --bin rain-daemon
```

The daemon listens on `127.0.0.1:7890`. Build the web client once so the daemon can serve it (run the daemon from the repository root):

```sh
npm --prefix apps/rain-client run build
```

On your other computer, forward the host through SSH:

```sh
ssh -N -L 7890:127.0.0.1:7890 your-user@your-host
```

Open `http://127.0.0.1:7890` on that computer. SSH supplies authentication and encryption. The SSH account needs access to the host; RAIN uses the daemon account's filesystem and process permissions. A phone needs an SSH client supporting local port forwarding. Direct LAN/public access is currently disabled; the in-app pairing screen does not establish an encrypted tunnel.

In Machines, launch an executable installed on the host. Arguments are a JSON string array, for example `["--some-option"]`. A working directory is a path on the host. Use the CLI's supported standard-input mode. Programs requiring an interactive terminal are not yet supported by this pipe adapter.

Open a session's chat, or create a group and select sessions. A group message is delivered to each member's stdin. `@name message` selects a uniquely named group member; `@all message` selects all members. `/commands` are passed literally to the CLI, so the CLI's input mode determines which commands work. Output lines and process exit events appear in conversations containing that session.

Deleting a group removes its conversation and workflow records without terminating its agents. Stop a session explicitly from Machines when needed.

## Current limits

Conversations and sessions are currently held in memory. Restarting the daemon loses conversation history and closes its supervised processes. Spoke mode publishes telemetry only; it does not yet execute forwarded work. Provider-specific approvals, interactive terminal emulation, persistent provider threads, and cross-machine workflow execution still require adapters. Output is line-buffered, and the broadcast event channel can drop events under sustained overload; a warning is logged when that happens.

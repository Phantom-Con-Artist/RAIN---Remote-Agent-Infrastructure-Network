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

In Machines, launch an executable installed on the host. Arguments are a JSON string array, for example `["--some-option"]`. A working directory is a path on the host. Enable Interactive terminal for CLIs that require a TTY, or disable it for standard-input programs. Unix hosts allocate a controlling PTY. The Terminal button opens an xterm.js terminal emulator with keyboard, paste, Enter, Tab, arrows, Escape, Interrupt, and PTY resize support. Interactive direct chats open their native terminal by default; use the terminal selector to return to the conversation. Terminal redraws stay out of chat messages.

Open a session's chat, or create a group and select sessions. A group message is delivered to each member's stdin. `@name message` selects a uniquely named group member; `@all message` selects all members. `/commands` are passed literally to the CLI, so the CLI's input mode determines which commands work. Output lines and process exit events appear in conversations containing that session.

Deleting a group removes its conversation and workflow records without terminating its agents. Stop a session explicitly from Machines when needed. Stopped sessions have a Remove action: it removes the process record and group routing membership while retaining conversation history. Running sessions must be stopped before removal.

## Execute on other machines

Configure an allowlist of SSH aliases before starting the hub:

```sh
export RAIN_SSH_HOSTS='{"workstation":"you@workstation", "gpu":"rain-gpu"}'
cargo run --bin rain-daemon
```

SSH targets may reference your existing SSH config, including ports, identities, and jump hosts. First verify each target from the daemon account with `ssh target`; authenticate with a key or SSH agent and verify the host key. RAIN uses BatchMode and strict host-key verification, so it cannot prompt for passwords or accept unknown hosts. It never disables host-key verification.

Choose the machine in the launcher. Working directories are paths on that remote machine and default to the remote login directory when omitted. Executables and arguments run on that machine; arguments are individually shell quoted. Remote environment overrides currently return an error. Sessions on different machines can participate in the same group, receiving group input through their individual SSH channels. Stopping a remote session closes its SSH process; detached remote descendants may need separate cleanup.

This execution path uses SSH directly. Spoke mode remains a telemetry publisher and is not used as an execution transport. No public daemon listener is needed on workers.

## Persistence and recovery

Conversations, messages, workflow records, and session metadata are saved to `.rain/state.json`. Set `RAIN_STATE_PATH` to choose another location. Writes use an atomic rename and filesystem sync; the daemon reports a failed history write rather than acknowledging a mutation as durable. Output events are saved by the event consumer; output persistence failures are logged. Back up this file while the daemon is stopped. Run only one daemon per state file.

On restart, history and session records return. Previously running sessions are marked stopped and their stale PIDs are removed. Processes are not automatically relaunched or reattached. Relaunch a session explicitly to continue. SSH keys remain in the host's normal SSH configuration and are not saved in conversation history.

## Current limits

PTY support currently requires a Unix hub. Terminal output is polled in ordered chunks; if a client falls behind the retained output window, it resets and replays that window. Provider-native thread resume and approval APIs still require provider-specific integrations. The broadcast output channel can drop events under sustained overload, with a logged warning. Cross-machine launch requires an SSH connection already configured for the daemon's account.

## Discover tools and browse remote files

In Machines, select a host to scan installed executables. Recognized agent CLIs get individual launch buttons; search finds other executables. The scanner checks PATH and common user binary directories, including npm, Cargo, Bun, and NVM locations. It does not run discovered executables, check subscriptions, or authenticate providers. A successful scan lists available executable files, not whether each provider is signed in. Each launch creates an independent global session that can be added to groups.

Use Launch terminal to start the detected shell. Files shares the machine selection with Machines and browses that host over SSH. Open folders, read UTF-8 files, and explicitly save edits; binary files cannot be edited and files larger than 5 MB are rejected. Launch terminal here starts a shell in the displayed directory. Changing hosts clears the explorer and editor so a file from one host cannot be saved accidentally on another.

Host scans and file operations require Python 3 on the selected host in addition to SSH access. They use the same SSH host allowlist and strict host-key checks as agent launches, with a 20-second timeout. Listings are limited to 10,000 entries and discovery to 5,000 executable names. Permissions are those of the SSH user (or daemon user for This host).

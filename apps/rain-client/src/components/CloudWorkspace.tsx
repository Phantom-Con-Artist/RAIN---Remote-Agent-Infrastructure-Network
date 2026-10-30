import { useCallback, useEffect, useRef, useState } from "react";
import { AdminNetworkCanvas, type SecuritySnapshot, } from "./AdminNetworkCanvas";
import { SecurityPanel } from "./SecurityPanel";
import QRCode from "qrcode";
import { FiRefreshCw, FiCopy, FiPlus, FiDownload, FiCheck, } from "react-icons/fi";
import { apiRequest, formatBytes, getHostUrl } from "../api";
interface Worker {
    id: string;
    name: string;
    revoked: boolean;
    enabled: boolean;
    online: boolean;
    programs: string[];
    resources: {
        cpu_cores?: number;
        ram_available_bytes?: number;
        disk_free_bytes?: number;
        shared_folder?: string;
    };
}
interface SharedFile {
    id: string;
    name: string;
    size_bytes: number;
    sha256: string;
    source: string;
}
interface Task {
    id: string;
    title: string;
    kind: string;
    target: string;
    worker_id?: string;
    status: string;
    outputs: string[];
    error?: string;
    result: {
        stdout?: string;
        stderr?: string;
        error?: string;
        exit_code?: number;
        truncated?: boolean;
        files?: {
            name: string;
            path: string;
            is_dir: boolean;
            size_bytes: number;
        }[];
    };
}
interface Snapshot {
    task_count?: number;
    hub?: {
        id: string;
        name: string;
    };
    workers: Worker[];
    tasks: Task[];
    files: SharedFile[];
}
export function CloudWorkspace() {
    const [olderTasks, setOlderTasks] = useState<Task[]>([]), [historyOffset, setHistoryOffset] = useState(200);
    const [security, setSecurity] = useState<SecuritySnapshot>({
        agents: [],
        groups: [],
        guests: [],
    });
    const [snapshot, setSnapshot] = useState<Snapshot | null>(null), [error, setError] = useState(""), [busy, setBusy] = useState(""), [stale, setStale] = useState(false);
    const [name, setName] = useState(""), [invite, setInvite] = useState<{
        token: string;
        expires_at: number;
    } | null>(null), [qr, setQr] = useState(""), [copied, setCopied] = useState(false), [record, setRecord] = useState<{
        id: string;
        value: unknown;
    } | null>(null);
    const [title, setTitle] = useState(""), [target, setTarget] = useState("any"), [kind, setKind] = useState("command"), [argv, setArgv] = useState('["python3", "-c", "print(\'Hello from my worker\')"]'), [path, setPath] = useState(""), [input, setInput] = useState("");
    const alive = useRef(true), pending = useRef(false), action = useRef(false);
    const refresh = useCallback(async () => {
        if (pending.current)
            return;
        pending.current = true;
        try {
            const [r, permissions] = await Promise.all([
                apiRequest<Snapshot>("/api/v1/cloud"),
                apiRequest<SecuritySnapshot>("/api/v1/cloud/security"),
            ]);
            if (alive.current) {
                setSnapshot(r.data);
                setSecurity(permissions.data);
                setStale(false);
                setError("");
            }
        }
        catch (e) {
            if (alive.current) {
                setStale(true);
                setError(e instanceof Error ? e.message : "Hub unavailable");
            }
        }
        finally {
            pending.current = false;
        }
    }, []);
    useEffect(() => {
        alive.current = true;
        refresh();
        const timer = setInterval(refresh, 3000);
        return () => {
            alive.current = false;
            clearInterval(timer);
        };
    }, [refresh]);
    const hub = getHostUrl(), local = (() => {
        try {
            return ["localhost", "127.0.0.1", "[::1]"].includes(new URL(hub).hostname);
        }
        catch {
            return true;
        }
    })();
    useEffect(() => {
        if (!local)
            QRCode.toDataURL(hub + "/#network/cloud", {
                width: 240,
                margin: 2,
                errorCorrectionLevel: "M",
            })
                .then((data) => {
                if (alive.current)
                    setQr(data);
            })
                .catch(() => { });
    }, [hub, local]);
    async function mutate(key: string, fn: () => Promise<unknown>) {
        if (action.current)
            return;
        action.current = true;
        setBusy(key);
        setError("");
        try {
            await fn();
            await refresh();
        }
        catch (e) {
            setError(e instanceof Error ? e.message : "Action failed");
        }
        finally {
            action.current = false;
            setBusy("");
        }
    }
    async function submit(e: React.FormEvent) {
        e.preventDefault();
        await mutate("task", async () => {
            let args: string[] = [];
            if (kind === "command") {
                try {
                    args = JSON.parse(argv);
                }
                catch {
                    throw Error("Enter the command as a JSON array of arguments.");
                }
                if (!Array.isArray(args) || args.some((a) => typeof a !== "string"))
                    throw Error("Every command argument must be text.");
            }
            await apiRequest("/api/v1/cloud/tasks", {
                method: "POST",
                redirect: "error",
                body: JSON.stringify({
                    title,
                    kind,
                    target,
                    argv: args,
                    path,
                    inputs: input ? [input] : [],
                    request_id: crypto.randomUUID(),
                }),
            });
            setTitle("");
        });
    }
    async function upload(file: File) {
        await mutate("upload", async () => {
            const response = await fetch(hub + "/api/v1/cloud/files?name=" + encodeURIComponent(file.name), {
                method: "POST",
                redirect: "error",
                body: file,
                signal: AbortSignal.timeout(120000),
            });
            const r = await response.json().catch(() => null);
            if (!response.ok || !r?.success)
                throw Error(r?.error || "File upload failed. Check your hub connection.");
        });
    }
    async function download(file: SharedFile) {
        await mutate("download", async () => {
            const r = await fetch(hub + `/api/v1/artifacts/${file.id}/content`, {
                redirect: "error",
                signal: AbortSignal.timeout(120000),
            });
            if (!r.ok)
                throw Error("File download failed. Check your hub connection.");
            const blob = await r.blob();
            if (blob.size !== file.size_bytes)
                throw Error("Download size mismatch. Retry the file.");
            const digest = await crypto.subtle.digest("SHA-256", await blob.arrayBuffer());
            const hash = Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
            if (hash !== file.sha256)
                throw Error("Download checksum mismatch. Retry the file.");
            const url = URL.createObjectURL(blob), a = document.createElement("a");
            a.href = url;
            a.download = file.name;
            a.click();
            setTimeout(() => URL.revokeObjectURL(url), 60000);
        });
    }
    const workers = snapshot?.workers || [], files = snapshot?.files || [], tasks = [
        ...[...(snapshot?.tasks || [])].reverse(),
        ...olderTasks.filter((t) => !snapshot?.tasks.some((current) => current.id === t.id)),
    ];
    return (<div className="cloud-workspace w-full min-w-0 [&>section]:mb-6 [&>section]:rounded-xl [&>section]:border [&>section]:border-border-subtle [&>section]:bg-canvas-surface/65 [&>section]:p-4 md:[&>section]:p-6 [&_h3]:mb-4 [&_h3]:text-base [&_p]:my-3 [&_p]:max-w-3xl [&_small]:block [&_label]:flex [&_label]:flex-col [&_label]:gap-2 [&_input:not([type=checkbox])]:w-full [&_textarea]:w-full [&_select]:w-full [&_summary]:font-mono [&_summary]:text-xs">
      <section className="cloud-introduction flex flex-wrap items-start justify-between gap-4 [&_h2]:text-2xl">
        <div>
          <h2>Your personal cloud</h2>
          <p>
            The hub keeps your files, task queue and results. Approved workers
            contribute execution and shared folders. Your phone is a remote
            client.
          </p>
        </div>
        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={refresh} aria-label="Refresh personal cloud">
          <FiRefreshCw />
        </button>
      </section>
      {error && (<p className="network-error rounded-lg border border-status-danger/35 bg-status-danger/10 p-3 text-sm text-ink [&:empty]:hidden" role="alert">
          {error}
          {stale ? " · Showing the last received state." : ""}
        </p>)}
      {!snapshot && !error && <p role="status">Loading your cloud…</p>}
      <AdminNetworkCanvas hub={snapshot?.hub} workers={workers} tasks={tasks} security={security} stale={stale} onAgentRevoke={(id) => mutate(id, () => apiRequest(`/api/v1/cloud/agents/${id}/revoke`, { method: "POST" }))} onTaskCancel={(id) => mutate(id, () => apiRequest(`/api/v1/cloud/tasks/${id}/cancel`, { method: "POST" }))} onControl={(id, action) => mutate(id, () => apiRequest(`/api/v1/cloud/workers/${id}/${action}`, {
            method: "POST",
            redirect: "error",
        }))}/>
      <SecurityPanel security={security} files={files} workers={workers} refresh={refresh}/>
      <section className="cloud-connect [&_img]:my-4 [&_img]:rounded-lg [&_a]:break-all">
        <h3>Connect your phone</h3>
        {local ? (<p>
            You are viewing a localhost address. Prepare private access on the
            server with <code>rain cloud-setup --help</code>. Once the HTTPS hub
            is reachable from your phone, open it here to get a connection QR
            code.
          </p>) : (<>
            <p>
              Connect your phone to your private network, scan this address and
              sign in with the owner’s gateway account. The QR code contains
              only the hub address.
            </p>
            {qr && (<img src={qr} width="200" height="200" alt="QR code for this hub’s cloud page"/>)}
            <a href={hub + "/#network/cloud"}>{hub}</a>
          </>)}
        <details>
          <summary>How private access works</summary>
          <p>
            The server owns the gateway and VPN. RAIN never provisions an
            outside relay. A public UDP endpoint or router port-forward is
            needed for direct WireGuard access over the internet. A QR code does
            not create network reachability or install certificate trust.
          </p>
        </details>
      </section>
      <section>
        <div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted">
          <h3>Workers & shared resources</h3>
          <span>
            {workers.filter((w) => w.online && !w.revoked).length} online
          </span>
        </div>
        <form className="cloud-invite-form space-y-3" onSubmit={(e) => {
            e.preventDefault();
            mutate("invite", async () => {
                const r = await apiRequest<{
                    token: string;
                    expires_at: number;
                }>("/api/v1/cloud/invites", {
                    method: "POST",
                    redirect: "error",
                    body: JSON.stringify({ name }),
                });
                setInvite(r.data);
                setCopied(false);
            });
        }}>
          <label htmlFor="cloud-worker-name">Worker name</label>
          <div className="connection-address flex min-w-0 flex-col gap-3 sm:flex-row [&>input]:min-w-0 [&>input]:flex-1">
            <input id="cloud-worker-name" value={name} onChange={(e) => setName(e.target.value)} maxLength={100} required placeholder="For example, Studio computer"/>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy}>
              <FiPlus />
              ISSUE_JOIN_TOKEN
            </button>
          </div>
        </form>
        {invite && (<div className="cloud-invitation my-4 space-y-3 rounded-xl border border-status-warning/30 bg-canvas-elevated p-4 [&_code]:break-all [&_code]:text-xs [&_pre]:whitespace-pre-wrap [&_pre]:break-all">
            <p>
              One use · expires{" "}
              {new Date(invite.expires_at * 1000).toLocaleTimeString()}. Save
              this secret into <code>invite.txt</code> on the worker.
            </p>
            <div className="connection-address flex min-w-0 flex-col gap-3 sm:flex-row [&>input]:min-w-0 [&>input]:flex-1">
              <code>{invite.token}</code>
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => mutate("copy", async () => {
                await navigator.clipboard.writeText(invite.token);
                setCopied(true);
            })}>
                {copied ? <FiCheck /> : <FiCopy />}
                {copied ? "Copied" : "COPY_JOIN_TOKEN"}
              </button>
            </div>
            <p>On that computer, run from its RAIN checkout:</p>
            <pre>
              rain worker --hub {hub} --home .rain-worker --workspace ./shared
              --invite-file invite.txt --allow-program python3
            </pre>
            <p>
              The worker opens an outbound connection. For a private CA, add{" "}
              <code>--ca /path/to/hub-root.crt</code>. Allow each command
              program explicitly. Phone clients do not run this command.
            </p>
            <button className="text-link inline-flex min-h-11 items-center gap-2 text-sm text-ink-secondary underline-offset-4 hover:text-ink hover:underline" onClick={() => setInvite(null)}>
              HIDE_JOIN_TOKEN
            </button>
          </div>)}
        <div className="network-table-scroll my-4 max-w-full overflow-x-auto rounded-xl border border-border-subtle bg-canvas-surface/70 [&_table]:min-w-[640px]">
          <table>
            <thead>
              <tr>
                <th>Worker</th>
                <th>Availability</th>
                <th>Shared capacity</th>
                <th>Allowed programs</th>
                <th>Control</th>
              </tr>
            </thead>
            <tbody>
              {workers.map((w) => (<tr key={w.id}>
                  <td>{w.name}</td>
                  <td>
                    {w.revoked
                ? "Revoked"
                : !w.online
                    ? "Offline"
                    : w.enabled
                        ? "Ready"
                        : "Paused"}
                  </td>
                  <td>
                    {w.resources.cpu_cores == null
                ? "Awaiting telemetry"
                : `${w.resources.cpu_cores} cores · ${w.resources.ram_available_bytes == null ? "Memory unknown" : formatBytes(w.resources.ram_available_bytes) + " available RAM"}`}
                    <small>
                      {w.resources.disk_free_bytes == null
                ? ""
                : formatBytes(w.resources.disk_free_bytes) +
                    " free disk"}
                    </small>
                  </td>
                  <td>{w.programs.join(", ") || "Shared files only"}</td>
                  <td>
                    {!w.revoked && (<>
                        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => mutate(w.id, () => apiRequest(`/api/v1/cloud/workers/${w.id}/${w.enabled ? "pause" : "resume"}`, { method: "POST" }))}>
                          {w.enabled ? "PAUSE_SHARING" : "RESUME_SHARING"}
                        </button>
                        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => {
                    if (window.confirm(`Revoke ${w.name}? Active tasks will be cancelled. Its shared folder stays on that computer.`))
                        mutate(w.id, () => apiRequest(`/api/v1/cloud/workers/${w.id}/revoke`, { method: "POST" }));
                }}>
                          REVOKE_KEY
                        </button>
                      </>)}
                  </td>
                </tr>))}
            </tbody>
          </table>
          {!workers.length && (<p className="network-empty min-h-24 p-6 text-sm text-ink-muted">
              Invite a computer to share resources. No worker is required on
              your phone.
            </p>)}
        </div>
      </section>
      <section>
        <div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted">
          <h3>Shared files</h3>
          <label className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger cloud-upload relative inline-flex! min-h-11 cursor-pointer flex-row! items-center gap-2 rounded-lg border border-border-subtle px-4 [&>input]:absolute [&>input]:inset-0 [&>input]:w-full [&>input]:cursor-pointer [&>input]:opacity-0 focus-within:outline-2 focus-within:outline-status-working">
            <FiPlus />
            {busy === "upload" ? "Uploading…" : "Upload file"}
            <input type="file" disabled={!!busy} aria-label="Upload shared file" onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) {
                if (f.size > 64 * 1024 * 1024)
                    setError("Choose a file up to 64 MB.");
                else
                    upload(f);
            }
            e.target.value = "";
        }}/>
          </label>
        </div>
        <p>
          Upload from this device or retrieve a file from a worker. Files pass
          through the hub and are verified with SHA-256.
        </p>
        <div className="cloud-file-list space-y-2 [&>div]:flex [&>div]:flex-wrap [&>div]:items-center [&>div]:justify-between [&>div]:gap-3 [&>div]:border-b [&>div]:border-border-chisel [&>div]:py-4 [&>div>span]:min-w-0 [&_strong]:break-all">
          {files.map((f) => (<div key={f.id}>
              <span>
                <strong>{f.name}</strong>
                <small>
                  {formatBytes(f.size_bytes)} ·{" "}
                  {f.source === "hub"
                ? "Uploaded to hub"
                : "Returned by worker"}
                </small>
              </span>
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => download(f)} aria-label={`Download ${f.name}`}>
                <FiDownload />
                DOWNLOAD_ARTIFACT
              </button>
            </div>))}
        </div>
        {!files.length && <p>No shared files yet.</p>}
      </section>
      <section>
        <h3>Send a task</h3>
        <p>
          To share the hub itself, run{" "}
          <code>rain-server share --allow-program python3</code> on the server.
          One task runs at a time on each worker. Commands run only when that
          worker allows the program. Queued tasks wait while their worker is
          offline.
        </p>
        <form className="cloud-task-form grid gap-4 md:grid-cols-2 [&>button]:justify-self-start" onSubmit={submit}>
          <label>
            Task name
            <input value={title} onChange={(e) => setTitle(e.target.value)} maxLength={200} required placeholder="What should this worker do?"/>
          </label>
          <label>
            Run on
            <select aria-label="Run on" value={target} onChange={(e) => setTarget(e.target.value)}>
              <option value="any">Any available worker</option>
              {workers
            .filter((w) => !w.revoked)
            .map((w) => (<option key={w.id} value={w.id}>
                    {w.name}
                    {w.online ? "" : " · offline"}
                  </option>))}
            </select>
          </label>
          <label>
            Action
            <select aria-label="Action" value={kind} onChange={(e) => {
            setKind(e.target.value);
            setInput("");
        }}>
              <option value="command">Run a command</option>
              <option value="list_files">Browse a shared folder</option>
              <option value="fetch_file">Retrieve a shared file</option>
              <option value="put_file">Send a file to a shared folder</option>
            </select>
          </label>
          {kind === "command" ? (<label className="cloud-wide md:col-span-2">
              Command arguments
              <textarea aria-label="Command arguments" value={argv} onChange={(e) => setArgv(e.target.value)} rows={3} required aria-describedby="cloud-command-help"/>
              <small id="cloud-command-help">
                Use an argument array. Input files are in RAIN_INPUT_DIR; put
                files to share back in RAIN_OUTPUT_DIR. Commands run with the
                worker account’s permissions.
              </small>
            </label>) : (<label className="cloud-wide md:col-span-2">
              Path inside the worker’s shared folder
              <input value={path} onChange={(e) => setPath(e.target.value)} required={kind !== "list_files"} placeholder={kind === "list_files"
                ? "Leave empty for the shared folder"
                : "For example, reports/result.txt"}/>
            </label>)}
          {(kind === "command" || kind === "put_file") && (<label className="cloud-wide md:col-span-2">
              {kind === "put_file" ? "File to send" : "Input file (optional)"}
              <select aria-label={kind === "put_file" ? "File to send" : "Input file (optional)"} value={input} onChange={(e) => setInput(e.target.value)} required={kind === "put_file"}>
                <option value="">Choose a shared file</option>
                {files.map((f) => (<option key={f.id} value={f.id}>
                    {f.name}
                  </option>))}
              </select>
            </label>)}
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={!!busy || !workers.some((w) => !w.revoked)}>
            {busy === "task" ? "Queuing…" : "DISPATCH_TASK"}
          </button>
        </form>
      </section>
      <section>
        <h3>Tasks & results</h3>
        {!tasks.length && (<p>
            Tasks and their results will stay here when you close this client.
          </p>)}
        <div className="cloud-task-list space-y-4 [&>article]:rounded-xl [&>article]:border [&>article]:border-border-chisel [&>article]:bg-canvas-base/40 [&>article]:p-4 [&_pre]:max-h-80 [&_pre]:whitespace-pre-wrap [&_pre]:break-words [&_pre]:text-xs">
          {tasks.map((t) => (<article key={t.id}>
              <div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted">
                <h4>{t.title}</h4>
                <span>
                  {t.status}
                  {t.status === "queued"
                ? " · waiting for an eligible worker"
                : ""}
                </span>
              </div>
              <p>
                {t.kind.replaceAll("_", " ")} ·{" "}
                {workers.find((w) => w.id === (t.worker_id || t.target))
                ?.name || "Any worker"}
              </p>
              {(t.error || t.result.error) && (<p className="network-transfer-error text-sm text-status-danger">
                  {t.error || t.result.error}
                </p>)}
              {t.result.stdout && <pre>{t.result.stdout}</pre>}
              {t.result.stderr && <pre>{t.result.stderr}</pre>}
              {t.result.exit_code != null && (<small>Exit code {t.result.exit_code}</small>)}
              {t.result.truncated && (<p>
                  This listing is limited to 1,000 entries or 180 KB. Choose a
                  narrower folder to inspect more files.
                </p>)}
              {t.result.files && (<ul className="cloud-directory space-y-2 [&>li]:flex [&>li]:items-center [&>li]:justify-between [&>li]:gap-3 [&>li]:border-b [&>li]:border-border-chisel [&>li]:py-2 [&>li>span]:break-all">
                  {t.result.files.map((f) => (<li key={f.path}>
                      <span>
                        {f.name}
                        {f.is_dir ? "/" : ` · ${formatBytes(f.size_bytes)}`}
                      </span>
                      <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => {
                        setTarget(t.worker_id || t.target);
                        setKind(f.is_dir ? "list_files" : "fetch_file");
                        setPath(f.path);
                        setTitle((f.is_dir ? "Browse " : "Retrieve ") + f.name);
                        document
                            .querySelector(".cloud-task-form")
                            ?.scrollIntoView({ behavior: "smooth" });
                    }}>
                        {f.is_dir ? "BROWSE_DIRECTORY" : "FETCH_ARTIFACT"}
                      </button>
                    </li>))}
                </ul>)}
              {t.outputs.map((id) => {
                const f = files.find((f) => f.id === id);
                return f ? (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" key={id} disabled={!!busy} onClick={() => download(f)}>
                    <FiDownload />
                    {f.name}
                  </button>) : null;
            })}
              <details>
                <summary>Full task record</summary>
                <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => mutate("record", async () => {
                const r = await apiRequest(`/api/v1/cloud/tasks/${t.id}`);
                setRecord({ id: t.id, value: r.data });
            })}>
                  INSPECT_TASK_RECORD
                </button>
                {record?.id === t.id && (<pre>{JSON.stringify(record.value, null, 2)}</pre>)}
              </details>
              <div className="cloud-task-actions mt-4 flex flex-wrap gap-3">
                {["queued", "running"].includes(t.status) ? (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => mutate(t.id, () => apiRequest(`/api/v1/cloud/tasks/${t.id}/cancel`, {
                    method: "POST",
                    redirect: "error",
                }))}>
                    CANCEL_TASK
                  </button>) : (["failed", "interrupted", "cancelled"].includes(t.status) && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => {
                    if (window.confirm("Retry as a new task? The previous attempt may have changed files or produced other effects."))
                        mutate(t.id, () => apiRequest(`/api/v1/cloud/tasks/${t.id}/retry`, {
                            method: "POST",
                            redirect: "error",
                        }));
                }}>
                      RETRY_TASK
                    </button>))}
              </div>
            </article>))}
        </div>
        {(snapshot?.task_count || 0) > tasks.length && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => mutate("history", async () => {
                const r = await apiRequest<{
                    tasks: Task[];
                    next: number;
                }>(`/api/v1/cloud/history?offset=${historyOffset}`);
                setOlderTasks((old) => [...old, ...r.data.tasks]);
                setHistoryOffset(r.data.next);
            })}>
            QUERY_TASK_HISTORY
          </button>)}
      </section>
    </div>);
}

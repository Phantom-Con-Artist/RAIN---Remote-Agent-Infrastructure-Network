import { useState } from "react";
import { apiRequest } from "../api";
import type { SecuritySnapshot } from "./AdminNetworkCanvas";
interface Props {
    security: SecuritySnapshot;
    files: {
        id: string;
        name: string;
    }[];
    workers: {
        id: string;
        name: string;
        revoked: boolean;
    }[];
    refresh: () => Promise<void>;
}
export function SecurityPanel({ security, files, workers, refresh }: Props) {
    const [error, setError] = useState(""), [busy, setBusy] = useState(false), [agentName, setAgentName] = useState(""), [agentWorker, setAgentWorker] = useState(""), [agentArgs, setAgentArgs] = useState('["python3","-c","print(\"Agent ready\")"]'), [agentKey, setAgentKey] = useState<{
        agent_id: string;
        token: string;
    } | null>(null), [groupName, setGroupName] = useState(""), [members, setMembers] = useState<string[]>([]), [group, setGroup] = useState(""), [file, setFile] = useState(""), [worker, setWorker] = useState(""), [path, setPath] = useState(""), [groupRecords, setGroupRecords] = useState<{
        id: string;
        messages: {
            sequence: number;
            record: {
                sender: string;
                text: string;
            };
        }[];
        next: number;
        total: number;
    } | null>(null), [newKey, setNewKey] = useState(""), [audit, setAudit] = useState<{
        action: string;
        status: number;
        time: string;
        role: string;
    }[] | null>(null);
    async function act(fn: () => Promise<unknown>, reload = true) {
        if (busy)
            return;
        setBusy(true);
        setError("");
        try {
            await fn();
            if (reload)
                await refresh();
        }
        catch (e) {
            setError(e instanceof Error ? e.message : "Permission update failed");
        }
        finally {
            setBusy(false);
        }
    }
    function post(path: string, value: unknown = {}) {
        return apiRequest(path, { method: "POST", body: JSON.stringify(value) });
    }
    function saveKey(key: string, name: string) {
        const a = document.createElement("a"), url = URL.createObjectURL(new Blob([key], { type: "text/plain" }));
        a.href = url;
        a.download = name;
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 30000);
    }
    return (<section className="admin-security [&>details]:border-t [&>details]:border-border-chisel [&>details]:py-3 [&_form]:flex [&_form]:max-w-2xl [&_form]:flex-col [&_form]:items-start [&_form]:gap-4 [&_form]:py-4 [&_form>label]:w-full [&_fieldset]:w-full [&_fieldset]:rounded-lg [&_fieldset]:border [&_fieldset]:border-border-subtle [&_fieldset]:p-4" id="admin-permissions">
      <div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted">
        <h3>Permissions & access</h3>
        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => act(async () => {
            await post("/api/v1/admin/logout");
            window.dispatchEvent(new Event("rain-admin-required"));
        })}>
          CLOSE_ADMIN_SESSION
        </button>
      </div>
      <p>
        Only the sysadmin can change network-wide policy. Machine owners can
        approve read-only guest requests for their own shared folder.
      </p>
      {error && (<p role="alert" className="network-error rounded-lg border border-status-danger/35 bg-status-danger/10 p-3 text-sm text-ink [&:empty]:hidden">
          {error}
        </p>)}
      <details>
        <summary>Admin key & access records</summary>
        <p>
          The admin key grants full control of the hub and approved workers.
          Rotating it signs out every admin session.
        </p>
        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => {
            if (window.confirm("Rotate the admin key and sign out every admin session? Save the new key before leaving this page."))
                act(async () => {
                    const r = await post("/api/v1/admin/rotate-key");
                    const key = (r.data as {
                        key: string;
                    }).key;
                    setNewKey(key);
                    saveKey(key, "rain-admin.key");
                }, false);
        }}>
          ROTATE_ADMIN_KEY
        </button>
        {newKey && (<div className="cloud-invitation my-4 space-y-3 rounded-xl border border-status-warning/30 bg-canvas-elevated p-4 [&_code]:break-all [&_code]:text-xs [&_pre]:whitespace-pre-wrap [&_pre]:break-all">
            <p>
              Save the replacement key. Previous keys and sessions are invalid.
            </p>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => saveKey(newKey, "rain-admin.key")}>
              EXPORT_ADMIN_KEY
            </button>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => window.dispatchEvent(new Event("rain-admin-required"))}>
              AUTHENTICATE_ADMIN
            </button>
          </div>)}
        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => act(async () => {
            const r = await apiRequest<{
                action: string;
                status: number;
                time: string;
                role: string;
            }[]>("/api/v1/admin/audit");
            setAudit(r.data);
        })}>
          READ_ACCESS_RECORDS
        </button>
        {audit && (<div className="network-table-scroll my-4 max-w-full overflow-x-auto rounded-xl border border-border-subtle bg-canvas-surface/70 [&_table]:min-w-[640px]">
            <table>
              <thead>
                <tr>
                  <th>When</th>
                  <th>Role</th>
                  <th>Action</th>
                  <th>HTTP result</th>
                </tr>
              </thead>
              <tbody>
                {audit.map((a, i) => (<tr key={i}>
                    <td>{new Date(a.time).toLocaleString()}</td>
                    <td>{a.role}</td>
                    <td>{a.action}</td>
                    <td>{a.status}</td>
                  </tr>))}
              </tbody>
            </table>
          </div>)}
      </details>
      <details open>
        <summary>Agents & credentials</summary>
        <form className="cloud-invite-form space-y-3" onSubmit={(e) => {
            e.preventDefault();
            act(async () => {
                let argv = [];
                if (agentWorker) {
                    try {
                        argv = JSON.parse(agentArgs);
                    }
                    catch {
                        throw Error("Agent command must be a JSON array of arguments");
                    }
                }
                const r = await post("/api/v1/cloud/agents", {
                    name: agentName,
                    worker: agentWorker || null,
                    argv,
                });
                setAgentKey(r.data as {
                    agent_id: string;
                    token: string;
                });
                setAgentName("");
            });
        }}>
          <label>
            Agent name
            <input value={agentName} onChange={(e) => setAgentName(e.target.value)} required maxLength={100}/>
          </label>
          <label>
            Run on machine (optional)
            <select value={agentWorker} onChange={(e) => setAgentWorker(e.target.value)}>
              <option value="">Credential only</option>
              {workers
            .filter((w) => !w.revoked)
            .map((w) => (<option key={w.id} value={w.id}>
                    {w.name}
                  </option>))}
            </select>
          </label>
          {agentWorker && (<label>
              Agent command arguments
              <textarea value={agentArgs} onChange={(e) => setAgentArgs(e.target.value)} rows={3}/>
              <small>
                The worker must allow this program. The agent receives its
                scoped key through its environment.
              </small>
            </label>)}
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy}>
            {agentWorker
            ? "LAUNCH_MANAGED_AGENT" : "ISSUE_AGENT_KEY"}
          </button>
        </form>
        {agentKey && (<div className="cloud-invitation my-4 space-y-3 rounded-xl border border-status-warning/30 bg-canvas-elevated p-4 [&_code]:break-all [&_code]:text-xs [&_pre]:whitespace-pre-wrap [&_pre]:break-all">
            <p>
              This credential grants only its agent’s group and approved guest
              access. Save it on that agent’s machine.
            </p>
            <p>
              Agent ID: <code>{agentKey.agent_id}</code>
            </p>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => saveKey(agentKey.token, "rain-agent.key")}>
              EXPORT_AGENT_KEY
            </button>
            <button className="text-link inline-flex min-h-11 items-center gap-2 text-sm text-ink-secondary underline-offset-4 hover:text-ink hover:underline" onClick={() => setAgentKey(null)}>
              HIDE_CREDENTIAL
            </button>
          </div>)}
        <div className="cloud-file-list space-y-2 [&>div]:flex [&>div]:flex-wrap [&>div]:items-center [&>div]:justify-between [&>div]:gap-3 [&>div]:border-b [&>div]:border-border-chisel [&>div]:py-4 [&>div>span]:min-w-0 [&_strong]:break-all">
          {security.agents.map((a) => (<div key={a.id}>
              <span>
                <strong>{a.name}</strong>
                <small>
                  {a.revoked
                ? "Revoked"
                : a.worker
                    ? "Managed on " +
                        (workers.find((w) => w.id === a.worker)?.name ||
                            a.worker)
                    : "Scoped agent credential"}
                </small>
              </span>
              {!a.revoked && a.task_id && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => act(() => post(`/api/v1/cloud/tasks/${a.task_id}/cancel`))}>
                  TERMINATE_AGENT
                </button>)}
              {!a.revoked && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => {
                    if (window.confirm(`Revoke ${a.name} and its guest permissions?`))
                        act(() => post(`/api/v1/cloud/agents/${a.id}/revoke`));
                }}>
                  REVOKE_AGENT_KEY
                </button>)}
            </div>))}
        </div>
      </details>
      <details>
        <summary>Protected groups & membership</summary>
        <form onSubmit={(e) => {
            e.preventDefault();
            act(async () => {
                await post("/api/v1/cloud/groups", { name: groupName, members });
                setGroupName("");
                setMembers([]);
            });
        }}>
          <label>
            Group name
            <input value={groupName} onChange={(e) => setGroupName(e.target.value)} required maxLength={100}/>
          </label>
          <fieldset>
            <legend>Members</legend>
            {security.agents
            .filter((a) => !a.revoked)
            .map((a) => (<label className="admin-member-option flex min-h-11 flex-row! items-center gap-3 [&_input]:h-5! [&_input]:w-5! [&_input]:min-h-5!" key={a.id}>
                  <input type="checkbox" checked={members.includes(a.id)} onChange={(e) => setMembers((m) => e.target.checked
                ? [...m, a.id]
                : m.filter((id) => id !== a.id))}/>
                  {a.name}
                </label>))}
          </fieldset>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy}>
            CREATE_PROTECTED_GROUP
          </button>
        </form>
        {security.groups.map((g) => (<article className="admin-group my-4 space-y-3 rounded-xl border border-border-subtle bg-canvas-base/30 p-4" key={g.id}>
            <h4>{g.name}</h4>
            <p>
              Members can talk in this group and read only the files and folders
              explicitly shared here.
            </p>
            {security.agents
                .filter((a) => !a.revoked)
                .map((a) => (<label className="admin-member-option flex min-h-11 flex-row! items-center gap-3 [&_input]:h-5! [&_input]:w-5! [&_input]:min-h-5!" key={a.id}>
                  <input type="checkbox" checked={g.members.includes(a.id)} disabled={busy} onChange={(e) => act(() => post(`/api/v1/cloud/groups/${g.id}/members`, {
                    members: e.target.checked
                        ? [...g.members, a.id]
                        : g.members.filter((id) => id !== a.id),
                }))}/>
                  {a.name}
                </label>))}
            {g.files.map((id) => (<div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted" key={id}>
                <span>
                  {files.find((f) => f.id === id)?.name || id} · read-only
                </span>
                <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => act(() => post(`/api/v1/cloud/groups/${g.id}/unshare/${id}`))}>
                  UNSHARE_ARTIFACT
                </button>
              </div>))}
            {g.folders?.map((f) => (<div className="cloud-section-heading mb-4 flex flex-wrap items-center justify-between gap-3 [&>span]:font-mono [&>span]:text-xs [&>span]:text-ink-muted" key={f.id}>
                <span>
                  {workers.find((w) => w.id === f.worker)?.name || f.worker}:{" "}
                  {f.path || "shared folder"} · read-only
                </span>
                <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => act(() => post(`/api/v1/cloud/groups/${g.id}/unshare/${f.id}`))}>
                  REVOKE_FOLDER_GRANT
                </button>
              </div>))}
            <details>
              <summary>
                Group records · {g.message_count ?? g.messages.length} messages
              </summary>
              {g.messages.slice(-30).map((m) => (<p key={m.id}>
                  <strong>
                    {security.agents.find((a) => a.id === m.sender)?.name ||
                    m.sender}
                  </strong>
                  : {m.text}
                </p>))}
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => act(async () => {
                const r = await apiRequest<{
                    messages: {
                        sequence: number;
                        record: {
                            sender: string;
                            text: string;
                        };
                    }[];
                    next: number;
                    total: number;
                }>(`/api/v1/cloud/groups/${g.id}/messages?after=${groupRecords?.id === g.id ? groupRecords.next : 0}`);
                setGroupRecords((old) => ({
                    id: g.id,
                    ...r.data,
                    messages: [
                        ...(old?.id === g.id ? old.messages : []),
                        ...r.data.messages,
                    ],
                }));
            })}>
                QUERY_GROUP_RECORDS
              </button>
              {groupRecords?.id === g.id && (<div>
                  {groupRecords.messages.map((m) => (<p key={m.sequence}>
                      <strong>
                        {security.agents.find((a) => a.id === m.record.sender)
                        ?.name || m.record.sender}
                      </strong>
                      : {m.record.text}
                    </p>))}
                  <small>
                    {groupRecords.next} of {groupRecords.total} records loaded
                  </small>
                </div>)}
            </details>
          </article>))}
      </details>
      <details>
        <summary>Share files or folders with a group</summary>
        <form onSubmit={(e) => {
            e.preventDefault();
            act(() => post(`/api/v1/cloud/groups/${group}/files`, { file }));
        }}>
          <label>
            Group
            <select value={group} onChange={(e) => setGroup(e.target.value)} required>
              <option value="">Choose a group</option>
              {security.groups.map((g) => (<option key={g.id} value={g.id}>
                  {g.name}
                </option>))}
            </select>
          </label>
          <label>
            INSPECT_ARTIFACT
            <select value={file} onChange={(e) => setFile(e.target.value)} required>
              <option value="">Choose a shared file</option>
              {files.map((f) => (<option key={f.id} value={f.id}>
                  {f.name}
                </option>))}
            </select>
          </label>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy}>
            SHARE_READ_ONLY
          </button>
        </form>
        <form onSubmit={(e) => {
            e.preventDefault();
            act(() => post(`/api/v1/cloud/groups/${group}/folders`, { worker, path }));
        }}>
          <label>
            Group
            <select value={group} onChange={(e) => setGroup(e.target.value)} required>
              <option value="">Choose a group</option>
              {security.groups.map((g) => (<option key={g.id} value={g.id}>
                  {g.name}
                </option>))}
            </select>
          </label>
          <label>
            Machine
            <select value={worker} onChange={(e) => setWorker(e.target.value)} required>
              <option value="">Choose a machine</option>
              {workers
            .filter((w) => !w.revoked)
            .map((w) => (<option key={w.id} value={w.id}>
                    {w.name}
                  </option>))}
            </select>
          </label>
          <label>
            Folder inside that machine’s shared root
            <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="For example, research"/>
          </label>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy}>
            GRANT_READ_ONLY_FOLDER
          </button>
        </form>
      </details>
      <details open>
        <summary>Guest inspection requests</summary>
        {!security.guests.length && (<p>
            No requests. Agents must request access before inspecting another
            machine.
          </p>)}
        {security.guests.map((g) => (<article className="owner-request my-4 space-y-3 rounded-xl border border-border-subtle bg-canvas-elevated/50 p-4" key={g.id}>
            <h4>
              {security.agents.find((a) => a.id === g.agent)?.name || g.agent} →{" "}
              {workers.find((w) => w.id === g.worker)?.name || g.worker}
            </h4>
            <p>
              {g.path || "Shared root"} · {g.reason}
            </p>
            <p>
              {g.status}
              {g.status === "approved"
                ? " · expires " +
                    new Date(g.expires_at * 1000).toLocaleTimeString()
                : ""}
            </p>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => act(() => post(`/api/v1/cloud/guests/${g.id}/approve`, {
                approved: true,
            }))}>
              GRANT_READ_ONLY_15M
            </button>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => act(() => post(`/api/v1/cloud/guests/${g.id}/approve`, {
                approved: false,
            }))}>
              DENY_OR_REVOKE
            </button>
          </article>))}
      </details>
    </section>);
}

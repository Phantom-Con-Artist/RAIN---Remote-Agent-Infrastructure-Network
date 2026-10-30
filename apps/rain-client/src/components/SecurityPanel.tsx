import { useState } from "react";
import { apiRequest } from "../api";
import type { SecuritySnapshot } from "./AdminNetworkCanvas";
interface Props {
  security: SecuritySnapshot;
  files: { id: string; name: string }[];
  workers: { id: string; name: string; revoked: boolean }[];
  refresh: () => Promise<void>;
}
export function SecurityPanel({ security, files, workers, refresh }: Props) {
  const [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [agentName, setAgentName] = useState(""),
    [agentWorker, setAgentWorker] = useState(""),
    [agentArgs, setAgentArgs] = useState(
      '["python3","-c","print(\"Agent ready\")"]',
    ),
    [agentKey, setAgentKey] = useState<{
      agent_id: string;
      token: string;
    } | null>(null),
    [groupName, setGroupName] = useState(""),
    [members, setMembers] = useState<string[]>([]),
    [group, setGroup] = useState(""),
    [file, setFile] = useState(""),
    [worker, setWorker] = useState(""),
    [path, setPath] = useState(""),
    [groupRecords, setGroupRecords] = useState<{
      id: string;
      messages: {
        sequence: number;
        record: { sender: string; text: string };
      }[];
      next: number;
      total: number;
    } | null>(null),
    [newKey, setNewKey] = useState(""),
    [audit, setAudit] = useState<
      { action: string; status: number; time: string; role: string }[] | null
    >(null);
  async function act(fn: () => Promise<unknown>, reload = true) {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await fn();
      if (reload) await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Permission update failed");
    } finally {
      setBusy(false);
    }
  }
  function post(path: string, value: unknown = {}) {
    return apiRequest(path, { method: "POST", body: JSON.stringify(value) });
  }
  function saveKey(key: string, name: string) {
    const a = document.createElement("a"),
      url = URL.createObjectURL(new Blob([key], { type: "text/plain" }));
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 30000);
  }
  return (
    <section className="admin-security" id="admin-permissions">
      <div className="cloud-section-heading">
        <h3>Permissions & access</h3>
        <button
          className="icon-button"
          onClick={() =>
            act(async () => {
              await post("/api/v1/admin/logout");
              window.dispatchEvent(new Event("rain-admin-required"));
            })
          }
        >
          Sign out admin
        </button>
      </div>
      <p>
        Only the sysadmin can change network-wide policy. Machine owners can
        approve read-only guest requests for their own shared folder.
      </p>
      {error && (
        <p role="alert" className="network-error">
          {error}
        </p>
      )}
      <details>
        <summary>Admin key & access records</summary>
        <p>
          The admin key grants full control of the hub and approved workers.
          Rotating it signs out every admin session.
        </p>
        <button
          className="icon-button"
          disabled={busy}
          onClick={() => {
            if (
              window.confirm(
                "Rotate the admin key and sign out every admin session? Save the new key before leaving this page.",
              )
            )
              act(async () => {
                const r = await post("/api/v1/admin/rotate-key");
                const key = (r.data as { key: string }).key;
                setNewKey(key);
                saveKey(key, "rain-admin.key");
              }, false);
          }}
        >
          Rotate admin key
        </button>
        {newKey && (
          <div className="cloud-invitation">
            <p>
              Save the replacement key. Previous keys and sessions are invalid.
            </p>
            <button
              className="icon-button"
              onClick={() => saveKey(newKey, "rain-admin.key")}
            >
              Download new admin key
            </button>
            <button
              className="icon-button"
              onClick={() =>
                window.dispatchEvent(new Event("rain-admin-required"))
              }
            >
              Sign in with new key
            </button>
          </div>
        )}
        <button
          className="icon-button"
          onClick={() =>
            act(async () => {
              const r = await apiRequest<
                { action: string; status: number; time: string; role: string }[]
              >("/api/v1/admin/audit");
              setAudit(r.data);
            })
          }
        >
          Read access records
        </button>
        {audit && (
          <div className="network-table-scroll">
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
                {audit.map((a, i) => (
                  <tr key={i}>
                    <td>{new Date(a.time).toLocaleString()}</td>
                    <td>{a.role}</td>
                    <td>{a.action}</td>
                    <td>{a.status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </details>
      <details open>
        <summary>Agents & credentials</summary>
        <form
          className="cloud-invite-form"
          onSubmit={(e) => {
            e.preventDefault();
            act(async () => {
              let argv = [];
              if (agentWorker) {
                try {
                  argv = JSON.parse(agentArgs);
                } catch {
                  throw Error(
                    "Agent command must be a JSON array of arguments",
                  );
                }
              }
              const r = await post("/api/v1/cloud/agents", {
                name: agentName,
                worker: agentWorker || null,
                argv,
              });
              setAgentKey(r.data as { agent_id: string; token: string });
              setAgentName("");
            });
          }}
        >
          <label>
            Agent name
            <input
              value={agentName}
              onChange={(e) => setAgentName(e.target.value)}
              required
              maxLength={100}
            />
          </label>
          <label>
            Run on machine (optional)
            <select
              value={agentWorker}
              onChange={(e) => setAgentWorker(e.target.value)}
            >
              <option value="">Credential only</option>
              {workers
                .filter((w) => !w.revoked)
                .map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
            </select>
          </label>
          {agentWorker && (
            <label>
              Agent command arguments
              <textarea
                value={agentArgs}
                onChange={(e) => setAgentArgs(e.target.value)}
                rows={3}
              />
              <small>
                The worker must allow this program. The agent receives its
                scoped key through its environment.
              </small>
            </label>
          )}
          <button className="icon-button" disabled={busy}>
            {agentWorker
              ? "Create and launch agent"
              : "Create agent credential"}
          </button>
        </form>
        {agentKey && (
          <div className="cloud-invitation">
            <p>
              This credential grants only its agent’s group and approved guest
              access. Save it on that agent’s machine.
            </p>
            <p>
              Agent ID: <code>{agentKey.agent_id}</code>
            </p>
            <button
              className="icon-button"
              onClick={() => saveKey(agentKey.token, "rain-agent.key")}
            >
              Download agent key
            </button>
            <button className="text-link" onClick={() => setAgentKey(null)}>
              Hide credential
            </button>
          </div>
        )}
        <div className="cloud-file-list">
          {security.agents.map((a) => (
            <div key={a.id}>
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
              {!a.revoked && a.task_id && (
                <button
                  className="icon-button"
                  disabled={busy}
                  onClick={() =>
                    act(() => post(`/api/v1/cloud/tasks/${a.task_id}/cancel`))
                  }
                >
                  Stop agent process
                </button>
              )}
              {!a.revoked && (
                <button
                  className="icon-button"
                  disabled={busy}
                  onClick={() => {
                    if (
                      window.confirm(
                        `Revoke ${a.name} and its guest permissions?`,
                      )
                    )
                      act(() => post(`/api/v1/cloud/agents/${a.id}/revoke`));
                  }}
                >
                  Revoke agent
                </button>
              )}
            </div>
          ))}
        </div>
      </details>
      <details>
        <summary>Protected groups & membership</summary>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            act(async () => {
              await post("/api/v1/cloud/groups", { name: groupName, members });
              setGroupName("");
              setMembers([]);
            });
          }}
        >
          <label>
            Group name
            <input
              value={groupName}
              onChange={(e) => setGroupName(e.target.value)}
              required
              maxLength={100}
            />
          </label>
          <fieldset>
            <legend>Members</legend>
            {security.agents
              .filter((a) => !a.revoked)
              .map((a) => (
                <label className="admin-member-option" key={a.id}>
                  <input
                    type="checkbox"
                    checked={members.includes(a.id)}
                    onChange={(e) =>
                      setMembers((m) =>
                        e.target.checked
                          ? [...m, a.id]
                          : m.filter((id) => id !== a.id),
                      )
                    }
                  />
                  {a.name}
                </label>
              ))}
          </fieldset>
          <button className="icon-button" disabled={busy}>
            Create protected group
          </button>
        </form>
        {security.groups.map((g) => (
          <article className="admin-group" key={g.id}>
            <h4>{g.name}</h4>
            <p>
              Members can talk in this group and read only the files and folders
              explicitly shared here.
            </p>
            {security.agents
              .filter((a) => !a.revoked)
              .map((a) => (
                <label className="admin-member-option" key={a.id}>
                  <input
                    type="checkbox"
                    checked={g.members.includes(a.id)}
                    disabled={busy}
                    onChange={(e) =>
                      act(() =>
                        post(`/api/v1/cloud/groups/${g.id}/members`, {
                          members: e.target.checked
                            ? [...g.members, a.id]
                            : g.members.filter((id) => id !== a.id),
                        }),
                      )
                    }
                  />
                  {a.name}
                </label>
              ))}
            {g.files.map((id) => (
              <div className="cloud-section-heading" key={id}>
                <span>
                  {files.find((f) => f.id === id)?.name || id} · read-only
                </span>
                <button
                  className="icon-button"
                  disabled={busy}
                  onClick={() =>
                    act(() =>
                      post(`/api/v1/cloud/groups/${g.id}/unshare/${id}`),
                    )
                  }
                >
                  Unshare
                </button>
              </div>
            ))}
            {g.folders?.map((f) => (
              <div className="cloud-section-heading" key={f.id}>
                <span>
                  {workers.find((w) => w.id === f.worker)?.name || f.worker}:{" "}
                  {f.path || "shared folder"} · read-only
                </span>
                <button
                  className="icon-button"
                  disabled={busy}
                  onClick={() =>
                    act(() =>
                      post(`/api/v1/cloud/groups/${g.id}/unshare/${f.id}`),
                    )
                  }
                >
                  Unshare folder
                </button>
              </div>
            ))}
            <details>
              <summary>
                Group records · {g.message_count ?? g.messages.length} messages
              </summary>
              {g.messages.slice(-30).map((m) => (
                <p key={m.id}>
                  <strong>
                    {security.agents.find((a) => a.id === m.sender)?.name ||
                      m.sender}
                  </strong>
                  : {m.text}
                </p>
              ))}
              <button
                className="icon-button"
                disabled={busy}
                onClick={() =>
                  act(async () => {
                    const r = await apiRequest<{
                      messages: {
                        sequence: number;
                        record: { sender: string; text: string };
                      }[];
                      next: number;
                      total: number;
                    }>(
                      `/api/v1/cloud/groups/${g.id}/messages?after=${groupRecords?.id === g.id ? groupRecords.next : 0}`,
                    );
                    setGroupRecords((old) => ({
                      id: g.id,
                      ...r.data,
                      messages: [
                        ...(old?.id === g.id ? old.messages : []),
                        ...r.data.messages,
                      ],
                    }));
                  })
                }
              >
                Read group history in order
              </button>
              {groupRecords?.id === g.id && (
                <div>
                  {groupRecords.messages.map((m) => (
                    <p key={m.sequence}>
                      <strong>
                        {security.agents.find((a) => a.id === m.record.sender)
                          ?.name || m.record.sender}
                      </strong>
                      : {m.record.text}
                    </p>
                  ))}
                  <small>
                    {groupRecords.next} of {groupRecords.total} records loaded
                  </small>
                </div>
              )}
            </details>
          </article>
        ))}
      </details>
      <details>
        <summary>Share files or folders with a group</summary>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            act(() => post(`/api/v1/cloud/groups/${group}/files`, { file }));
          }}
        >
          <label>
            Group
            <select
              value={group}
              onChange={(e) => setGroup(e.target.value)}
              required
            >
              <option value="">Choose a group</option>
              {security.groups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            File
            <select
              value={file}
              onChange={(e) => setFile(e.target.value)}
              required
            >
              <option value="">Choose a shared file</option>
              {files.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.name}
                </option>
              ))}
            </select>
          </label>
          <button className="icon-button" disabled={busy}>
            Share file read-only
          </button>
        </form>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            act(() =>
              post(`/api/v1/cloud/groups/${group}/folders`, { worker, path }),
            );
          }}
        >
          <label>
            Group
            <select
              value={group}
              onChange={(e) => setGroup(e.target.value)}
              required
            >
              <option value="">Choose a group</option>
              {security.groups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Machine
            <select
              value={worker}
              onChange={(e) => setWorker(e.target.value)}
              required
            >
              <option value="">Choose a machine</option>
              {workers
                .filter((w) => !w.revoked)
                .map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Folder inside that machine’s shared root
            <input
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="For example, research"
            />
          </label>
          <button className="icon-button" disabled={busy}>
            Share folder read-only
          </button>
        </form>
      </details>
      <details open>
        <summary>Guest inspection requests</summary>
        {!security.guests.length && (
          <p>
            No requests. Agents must request access before inspecting another
            machine.
          </p>
        )}
        {security.guests.map((g) => (
          <article className="owner-request" key={g.id}>
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
            <button
              className="icon-button"
              disabled={busy}
              onClick={() =>
                act(() =>
                  post(`/api/v1/cloud/guests/${g.id}/approve`, {
                    approved: true,
                  }),
                )
              }
            >
              Allow read-only for 15 minutes
            </button>
            <button
              className="icon-button"
              disabled={busy}
              onClick={() =>
                act(() =>
                  post(`/api/v1/cloud/guests/${g.id}/approve`, {
                    approved: false,
                  }),
                )
              }
            >
              Deny or revoke
            </button>
          </article>
        ))}
      </details>
    </section>
  );
}

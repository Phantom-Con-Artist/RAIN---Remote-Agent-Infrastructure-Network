import { useState } from "react";
import { getHostUrl } from "../api";
import { RainIdentity } from "./RainIdentity";
interface Guest {
  id: string;
  agent: string;
  path: string;
  reason: string;
  status: string;
  expires_at: number;
}
export function OwnerAccess() {
  const [worker, setWorker] = useState(""),
    [key, setKey] = useState(""),
    [requests, setRequests] = useState<Guest[] | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  async function load() {
    const r = await fetch(getHostUrl() + "/api/v1/cloud/owner/guests", {
      headers: { "X-Rain-Owner-Id": worker, "X-Rain-Owner-Key": key },
      redirect: "error",
      signal: AbortSignal.timeout(10000),
    });
    const body = await r.json();
    if (!r.ok || !body.success)
      throw Error(body.error || "Machine owner authentication failed");
    setRequests(body.data);
  }
  async function enter(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Connection failed");
    } finally {
      setBusy(false);
    }
  }
  async function approve(id: string, approved: boolean) {
    setBusy(true);
    setError("");
    try {
      const r = await fetch(
        getHostUrl() + `/api/v1/cloud/owner/guests/${id}/approve`,
        {
          method: "POST",
          redirect: "error",
          headers: {
            "Content-Type": "application/json",
            "X-Rain-Owner-Id": worker,
            "X-Rain-Owner-Key": key,
          },
          body: JSON.stringify({ approved }),
        },
      );
      const body = await r.json();
      if (!r.ok || !body.success) throw Error(body.error || "Approval failed");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Approval failed");
    } finally {
      setBusy(false);
    }
  }
  return (
    <main className="admin-entry">
      <RainIdentity />
      <section>
        <h1>Your machine’s access</h1>
        <p>
          Approve temporary, read-only guest access to your machine’s shared
          folder. This key cannot open the hub’s sysadmin interface.
        </p>
        <form onSubmit={enter}>
          <label>
            Worker ID
            <input
              value={worker}
              onChange={(e) => setWorker(e.target.value)}
              required
            />
          </label>
          <label>
            Machine owner key
            <input
              type="password"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              required
              autoComplete="off"
            />
          </label>
          <p>
            Find the worker ID in <code>worker.json</code> and the key in{" "}
            <code>owner.key</code> in that machine’s RAIN worker directory.
          </p>
          <button className="icon-button primary" disabled={busy}>
            {busy ? "Checking…" : "View access requests"}
          </button>
        </form>
        {error && (
          <p role="alert" className="network-error">
            {error}
          </p>
        )}
        {requests?.map((g) => (
          <article className="owner-request" key={g.id}>
            <h2>{g.path || "Shared folder"}</h2>
            <p>{g.reason}</p>
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
              onClick={() => approve(g.id, true)}
            >
              Allow read-only for 15 minutes
            </button>
            <button
              className="icon-button"
              disabled={busy}
              onClick={() => approve(g.id, false)}
            >
              Deny or revoke
            </button>
          </article>
        ))}
        {requests?.length === 0 && <p>No requests for this machine.</p>}
        <a className="text-link" href="#home">
          Back to sysadmin sign-in
        </a>
      </section>
    </main>
  );
}

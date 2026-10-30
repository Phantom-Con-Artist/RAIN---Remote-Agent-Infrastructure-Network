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
    const [worker, setWorker] = useState(""), [key, setKey] = useState(""), [requests, setRequests] = useState<Guest[] | null>(null), [error, setError] = useState(""), [busy, setBusy] = useState(false);
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
        }
        catch (e) {
            setError(e instanceof Error ? e.message : "Connection failed");
        }
        finally {
            setBusy(false);
        }
    }
    async function approve(id: string, approved: boolean) {
        setBusy(true);
        setError("");
        try {
            const r = await fetch(getHostUrl() + `/api/v1/cloud/owner/guests/${id}/approve`, {
                method: "POST",
                redirect: "error",
                headers: {
                    "Content-Type": "application/json",
                    "X-Rain-Owner-Id": worker,
                    "X-Rain-Owner-Key": key,
                },
                body: JSON.stringify({ approved }),
            });
            const body = await r.json();
            if (!r.ok || !body.success)
                throw Error(body.error || "Approval failed");
            await load();
        }
        catch (e) {
            setError(e instanceof Error ? e.message : "Approval failed");
        }
        finally {
            setBusy(false);
        }
    }
    return (<main className="admin-entry min-h-dvh px-4 py-6 text-ink md:px-8 [&>section]:mx-auto [&>section]:mt-12 [&>section]:max-w-xl [&_h1]:mb-6 [&_h1]:text-3xl [&_p]:my-4 [&_form]:mt-6 [&_form]:flex [&_form]:flex-col [&_form]:items-start [&_form]:gap-4 [&_form>label]:w-full [&_label]:flex [&_label]:flex-col [&_label]:gap-2 [&_input]:w-full [&_select]:w-full [&_small]:mt-2">
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
            <input value={worker} onChange={(e) => setWorker(e.target.value)} required/>
          </label>
          <label>
            Machine owner key
            <input type="password" value={key} onChange={(e) => setKey(e.target.value)} required autoComplete="off"/>
          </label>
          <p>
            Find the worker ID in <code>worker.json</code> and the key in{" "}
            <code>owner.key</code> in that machine’s RAIN worker directory.
          </p>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={busy}>
            {busy ? "Checking…" : "QUERY_ACCESS_REQUESTS"}
          </button>
        </form>
        {error && (<p role="alert" className="network-error rounded-lg border border-status-danger/35 bg-status-danger/10 p-3 text-sm text-ink [&:empty]:hidden">
            {error}
          </p>)}
        {requests?.map((g) => (<article className="owner-request my-4 space-y-3 rounded-xl border border-border-subtle bg-canvas-elevated/50 p-4" key={g.id}>
            <h2>{g.path || "Shared folder"}</h2>
            <p>{g.reason}</p>
            <p>
              {g.status}
              {g.status === "approved"
                ? " · expires " +
                    new Date(g.expires_at * 1000).toLocaleTimeString()
                : ""}
            </p>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => approve(g.id, true)}>
              GRANT_READ_ONLY_15M
            </button>
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy} onClick={() => approve(g.id, false)}>
              DENY_OR_REVOKE
            </button>
          </article>))}
        {requests?.length === 0 && <p>No requests for this machine.</p>}
        <a className="text-link inline-flex min-h-11 items-center gap-2 text-sm text-ink-secondary underline-offset-4 hover:text-ink hover:underline" href="#home">
          Back to sysadmin sign-in
        </a>
      </section>
    </main>);
}

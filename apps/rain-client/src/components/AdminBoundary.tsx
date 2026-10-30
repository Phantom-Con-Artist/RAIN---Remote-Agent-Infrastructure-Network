import { useEffect, useState, type ReactNode } from "react";
import { RainIdentity } from "./RainIdentity";
import { getHostUrl } from "../api";
import { OwnerAccess } from "./OwnerAccess";
export function AdminBoundary({ children }: { children: ReactNode }) {
  const [saved, setSaved] = useState<
    { id: string; name: string; url: string }[]
  >(() => {
    try {
      return JSON.parse(
        localStorage.getItem("rain_infrastructures") || "[]",
      ).filter((p: { url: string; name: string; id: string }) => {
        try {
          if (
            typeof p.id !== "string" ||
            typeof p.name !== "string" ||
            typeof p.url !== "string" ||
            p.name.length > 100
          )
            return false;
          const u = new URL(p.url);
          return (
            !u.username &&
            !u.password &&
            !u.search &&
            !u.hash &&
            (u.protocol === "https:" ||
              (u.protocol === "http:" &&
                ["localhost", "127.0.0.1", "[::1]"].includes(u.hostname)))
          );
        } catch {
          return false;
        }
      });
    } catch {
      return [];
    }
  });
  const [state, setState] = useState<"checking" | "locked" | "ready">(
      "checking",
    ),
    [key, setKey] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [owner, setOwner] = useState(location.hash.startsWith("#owner"));
  useEffect(() => {
    const hash = () => setOwner(location.hash.startsWith("#owner"));
    window.addEventListener("hashchange", hash);
    return () => window.removeEventListener("hashchange", hash);
  }, []);
  useEffect(() => {
    let alive = true;
    const lock = () => {
      setState("locked");
      setKey("");
    };
    const check = async () => {
      try {
        const r = await fetch(getHostUrl() + "/api/v1/admin/session", {
          redirect: "error",
          signal: AbortSignal.timeout(8000),
        });
        if (alive) setState(r.ok ? "ready" : "locked");
      } catch {
        if (alive) {
          setState("locked");
          setError(
            "Cannot reach the hub. Check that it is running and your private connection is active.",
          );
        }
      }
    };
    check();
    window.addEventListener("rain-admin-required", lock);
    const timer = setInterval(check, 60000);
    return () => {
      alive = false;
      clearInterval(timer);
      window.removeEventListener("rain-admin-required", lock);
    };
  }, []);
  async function login(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      const r = await fetch(getHostUrl() + "/api/v1/admin/login", {
        method: "POST",
        redirect: "error",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: key.trim() }),
        signal: AbortSignal.timeout(10000),
      });
      const body = await r.json();
      if (!r.ok || !body.success)
        throw Error(body.error || "Admin key was not accepted");
      const infrastructure = body.data?.infrastructure;
      if (infrastructure) {
        let previous: { id: string; name: string; url: string }[] = [];
        try {
          const stored = JSON.parse(
            localStorage.getItem("rain_infrastructures") || "[]",
          );
          if (Array.isArray(stored)) previous = stored;
        } catch {}
        const profiles = [
          ...previous
            .filter((p: { id: string }) => p && p.id !== infrastructure.id)
            .slice(-19),
          {
            id: infrastructure.id,
            name: infrastructure.name,
            url: getHostUrl(),
          },
        ];
        localStorage.setItem("rain_infrastructures", JSON.stringify(profiles));
        setSaved(profiles);
      }
      setKey("");
      setState("ready");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Sign-in failed");
    } finally {
      setBusy(false);
    }
  }
  if (owner) return <OwnerAccess />;
  if (state === "ready") return <>{children}</>;
  return (
    <main className="admin-entry">
      <RainIdentity />
      <section>
        <h1>Your personal cloud</h1>
        <p>
          Use the admin key from your hub to open its control panel. Worker and
          agent keys cannot open this interface.
        </p>
        {saved.length > 0 && (
          <label>
            Saved infrastructures
            <select
              aria-label="Saved infrastructures"
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) window.location.assign(e.target.value);
              }}
            >
              <option value="">Choose an infrastructure</option>
              {saved.map((p) => (
                <option key={p.id} value={p.url}>
                  {p.name} · {p.url}
                </option>
              ))}
            </select>
            <small>
              Names and addresses are remembered on this device. Admin keys are
              never stored here.
            </small>
          </label>
        )}
        {state === "checking" ? (
          <p role="status">Checking your session…</p>
        ) : (
          <form onSubmit={login}>
            <label htmlFor="admin-key">Admin key</label>
            <input
              id="admin-key"
              type="password"
              autoComplete="off"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              required
              spellCheck={false}
              aria-describedby="admin-key-help"
            />
            <p id="admin-key-help">
              The key is stored in <code>admin.key</code> beside the hub’s state
              file. On the server, use <code>rain-server admin-key</code> to
              locate it. The key grants full sysadmin access.
            </p>
            <label className="text-link admin-key-file">
              Choose a key file
              <input
                type="file"
                aria-label="Choose admin key file"
                onChange={async (e) => {
                  const file = e.target.files?.[0];
                  if (file) {
                    if (file.size > 4096) {
                      setError("Choose the admin key file, up to 4 KB.");
                      return;
                    }
                    setKey((await file.text()).trim());
                  }
                }}
              />
            </label>
            <button className="icon-button primary" disabled={busy}>
              {busy ? "Signing in…" : "Open admin panel"}
            </button>
          </form>
        )}
        {error && (
          <p role="alert" className="network-error">
            {error}
          </p>
        )}
        <a className="text-link" href="#owner">
          Approve access as a machine owner
        </a>
      </section>
    </main>
  );
}

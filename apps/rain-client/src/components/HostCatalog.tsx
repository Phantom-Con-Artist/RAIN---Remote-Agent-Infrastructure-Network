import React, { useEffect, useState } from 'react';
import { apiRequest } from '../api';
interface Executable {
    name: string;
    command: string;
    chat_provider?: string;
}
interface Catalog {
    agents: Executable[];
    executables: Executable[];
    shell: string | null;
    home: string;
}
export function HostCatalog({ machine, cwd, onLaunch }: {
    machine: string;
    cwd: string;
    onLaunch: (command: string, name: string, args: string[], provider?: string) => Promise<void>;
}) {
    const [catalog, setCatalog] = useState<Catalog | null>(null), [error, setError] = useState(''), [loading, setLoading] = useState(false), [busy, setBusy] = useState(''), [search, setSearch] = useState(''), [revision, setRevision] = useState(0);
    useEffect(() => {
        let active = true;
        setCatalog(null);
        setError('');
        setLoading(true);
        apiRequest<Catalog>(`/api/v1/agents/catalog?machine=${encodeURIComponent(machine)}`).then(r => {
            if (active)
                setCatalog(r.data);
        }).catch(e => {
            if (active)
                setError(e.message);
        }).finally(() => {
            if (active)
                setLoading(false);
        });
        return () => { active = false; };
    }, [machine, revision]);
    async function launch(item: Executable, args: string[] = []) {
        setBusy(item.command);
        setError('');
        try {
            await onLaunch(item.command, item.name, args, item.chat_provider);
        }
        catch (e: any) {
            setError(e.message);
        }
        finally {
            setBusy('');
        }
    }
    const matches = search.trim() ? catalog?.executables.filter(e => e.name.toLowerCase().includes(search.toLowerCase())).slice(0, 40) : [];
    return <section className="host-catalog my-4 rounded-xl border border-border-subtle bg-canvas-surface/70 p-4 [&_h3]:mb-3 [&_p]:mb-4 [&_p]:text-sm" aria-label="Installed CLI tools">
  <div className="section-header-row mb-4 flex flex-wrap items-center justify-between gap-4"><h3>Installed agent CLIs</h3><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={loading} onClick={() => setRevision(v => v + 1)}>{loading ? 'Scanning…' : 'Scan again'}</button></div>
  <p>Launch on {machine || 'this host'}{cwd ? ` in ${cwd}` : ''}. Each click creates an independent session.</p>
  {error && <p role="alert">{error}</p>}
  <div className="catalog-actions mt-4 flex flex-wrap gap-2">{catalog?.agents.map(item => <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} key={item.command} title={item.command} onClick={() => launch(item)}>{item.chat_provider ? 'Chat with' : 'Launch'} {item.name}</button>)}
  {catalog?.shell && <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={!!busy} onClick={() => launch({ name: 'Terminal', command: catalog.shell! }, ['-l'])}>OPEN_REMOTE_TERMINAL</button>}</div>
  {catalog && !catalog.agents.length && <p>No recognized agent CLIs found. Search installed executables below, or use the custom launcher.</p>}
  <label>Find another installed CLI<input className="chat-input-field min-h-11 w-full min-w-0 resize-none border-0! bg-transparent! px-3 py-2 text-base text-ink placeholder:text-ink-muted" value={search} onChange={e => setSearch(e.target.value)} placeholder="Search executable names"/></label>
  {!!matches?.length && <div className="catalog-actions mt-4 flex flex-wrap gap-2">{matches.map(item => <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} key={item.command} title={item.command} onClick={() => launch(item)}>{item.chat_provider ? 'Chat with' : 'Launch'} {item.name}</button>)}</div>}
  {busy && <p role="status">Launching session…</p>}
 </section>;
}

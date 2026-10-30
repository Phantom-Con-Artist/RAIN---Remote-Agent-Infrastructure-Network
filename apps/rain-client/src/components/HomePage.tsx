import { useEffect, useState } from 'react';
import { FiArrowUpRight, FiCheck, FiCopy, FiServer, FiCpu, FiHardDrive, FiActivity, FiShield } from 'react-icons/fi';
import { apiRequest, formatBytes, getHostUrl } from '../api';
import type { AgentSession, Conversation, TabType } from '../types';
export function CopyCommand({ text }: {
    text: string;
}) { const [copied, setCopied] = useState(false), [error, setError] = useState(''); return <div className="copy-command my-3 flex min-w-0 items-center justify-between gap-3 rounded-lg border border-border-subtle bg-canvas-base px-4 py-2"><code className="min-w-0 overflow-auto text-xs text-ink-secondary">{text}</code><button className="size-11 shrink-0 text-ink-secondary" aria-label={`Copy ${text}`} onClick={async () => { try {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
}
catch {
    setError('Select and copy this command.');
} }}>{copied ? <FiCheck /> : <FiCopy />}</button>{error && <small role="status">{error}</small>}</div>; }
interface Overview {
    workers: {
        id: string;
        name: string;
        online: boolean;
        revoked: boolean;
        resources: {
            cpu_cores?: number;
            ram_total_bytes?: number;
            ram_available_bytes?: number;
            disk_total_bytes?: number;
            disk_free_bytes?: number;
        };
    }[];
    tasks: {
        id: string;
        title: string;
        status: string;
        kind: string;
    }[];
    files: unknown[];
    hub: {
        name: string;
    };
}
export function HomePage({ sessions, conversations, onNavigate, onOpenConversation, connectionStatus }: {
    sessions: AgentSession[];
    conversations: Conversation[];
    onNavigate: (tab: TabType) => void;
    onOpenConversation: (id: string) => void;
    connectionStatus: string;
}) {
    const [data, setData] = useState<Overview | null>(null), [error, setError] = useState('');
    useEffect(() => { let live = true; const load = async () => { try {
        const r = await apiRequest<Overview>('/api/v1/cloud');
        if (live) {
            setData(r.data);
            setError('');
        }
    }
    catch (e) {
        if (live)
            setError(e instanceof Error ? e.message : 'Telemetry unavailable');
    } }; load(); const t = setInterval(load, 5000); return () => { live = false; clearInterval(t); }; }, []);
    const running = sessions.filter(s => ['online', 'working', 'thinking', 'waiting', 'responding'].includes(s.state)), workers = data?.workers.filter(w => !w.revoked) || [], active = data?.tasks.filter(t => ['queued', 'running'].includes(t.status)) || [];
    return <div className="home-page mx-auto h-full w-full max-w-7xl overflow-y-auto px-4 py-6 md:px-6"><div className="mb-6 flex flex-wrap items-start justify-between gap-4"><div><div className="mb-2 flex items-center gap-2 font-mono text-[10px] tracking-wider text-ink-muted"><FiShield />SYSADMIN OPERATIONS / {data?.hub.name || 'INFRASTRUCTURE'}</div><h1 className="text-2xl font-medium tracking-tight">Operations console</h1><p className="mt-2 text-sm">Your infrastructure. Your agents. One authenticated control plane.</p></div><button className="min-h-11 gap-3 rounded-xl border border-status-working/30 bg-status-working/10 px-4 font-mono text-xs text-ink" onClick={() => onNavigate('mesh')}>DISPATCH_TASK <FiArrowUpRight /></button></div>
 <div className="mb-6 grid grid-cols-2 divide-x divide-border-chisel overflow-hidden rounded-xl border border-border-subtle bg-canvas-surface/80 lg:grid-cols-4">{[{ label: 'APPROVED WORKERS', value: data ? workers.length : '—', detail: data ? `${workers.filter(w => w.online).length} reporting online` : 'Awaiting telemetry' }, { label: 'AGENT SESSIONS', value: running.length, detail: 'Installed CLI processes' }, { label: 'ACTIVE TASKS', value: data ? active.length : '—', detail: data ? `${active.filter(t => t.status === 'running').length} currently executing` : 'Awaiting queue' }, { label: 'SHARED ARTIFACTS', value: data ? data.files.length : '—', detail: 'Verified immutable snapshots' }].map(m => <div key={m.label} className="min-h-28 border-b border-border-chisel p-4 lg:border-b-0 lg:p-5"><div className="font-mono text-[10px] tracking-wider text-ink-muted">{m.label}</div><div className="my-2 font-mono text-3xl tabular-nums text-ink">{m.value}</div><div className="text-xs text-ink-secondary">{m.detail}</div></div>)}</div>
 {error && <p role="status" className="mb-5 rounded-lg border border-status-warning/30 bg-bubble-sudo p-3 text-sm text-ink">{error} · Displaying the last received state.</p>}
 <div className="grid gap-6 xl:grid-cols-[minmax(0,1.5fr)_minmax(280px,1fr)]"><section className="min-w-0 rounded-xl border border-border-subtle bg-canvas-surface/75 p-4 backdrop-blur-md md:p-5"><div className="mb-5 flex items-center justify-between"><h2 className="text-base">Contributed machines</h2><button className="min-h-11 font-mono text-[10px] text-ink-secondary" onClick={() => onNavigate('mesh')}>INSPECT_TOPOLOGY <FiArrowUpRight /></button></div><div className="mb-4 flex items-center gap-4 rounded-lg border border-border-glow bg-canvas-elevated/60 p-4"><FiServer className="shrink-0 text-status-working" size={22}/><div className="min-w-0 flex-1"><strong className="block truncate text-sm">{data?.hub.name || 'Central hub'}</strong><span className="font-mono text-[10px] text-ink-muted">{new URL(getHostUrl()).host} · AUTHENTICATED_CONTROL_PLANE</span></div><span className={`size-2 shrink-0 rounded-full ${connectionStatus === 'online' ? 'bg-status-online' : 'bg-status-warning'}`}/></div>
 <div className="min-h-44 space-y-3">{workers.map(w => { const total = w.resources.ram_total_bytes, available = w.resources.ram_available_bytes, used = total != null && available != null ? Math.max(0, total - available) : null, percent = total && used != null ? used / total * 100 : 0; return <button key={w.id} className="flex w-full flex-col items-stretch gap-3 rounded-xl border border-border-subtle bg-canvas-base/50 p-4 text-left" onClick={() => onNavigate('mesh')}><div className="flex items-center gap-3"><FiCpu className="text-ink-secondary"/><strong className="min-w-0 flex-1 truncate text-sm">{w.name}</strong><span className="font-mono text-[10px] text-ink-secondary">{w.online ? 'ONLINE' : 'UNREACHABLE'}</span><i className={`size-1.5 rounded-full ${w.online ? 'bg-status-online' : 'bg-status-muted'}`}/></div><div className="flex flex-wrap items-center justify-between gap-3 font-mono text-[10px] text-ink-muted"><span>{w.resources.cpu_cores ?? '—'} CPU CORES</span><span>{used != null && total ? `${formatBytes(used)} / ${formatBytes(total)} RAM` : 'RAM: NOT_REPORTED'}</span></div><div className="h-1 overflow-hidden rounded-full bg-canvas-hover"><div className={`h-full rounded-full ${percent > 80 ? 'bg-status-warning' : percent > 50 ? 'bg-status-working' : 'bg-status-online'}`} style={{ width: `${Math.min(100, percent)}%` }}/></div><div className="flex items-center gap-2 font-mono text-[10px] text-ink-muted"><FiHardDrive />{w.resources.disk_free_bytes == null ? 'DISK: NOT_REPORTED' : `${formatBytes(w.resources.disk_free_bytes)} FREE DISK`}</div></button>; })}{!workers.length && <div className="flex min-h-44 flex-col items-start justify-center gap-3 rounded-xl border border-dashed border-border-subtle p-5"><p className="max-w-lg text-sm">No worker has joined this infrastructure. Issue an invitation or enable the hub’s local resource worker.</p><button className="border border-border-subtle font-mono text-xs" onClick={() => onNavigate('mesh')}>ENROLL_NODE <FiArrowUpRight /></button></div>}</div></section>
 <section className="rounded-xl border border-border-subtle bg-canvas-surface/75 p-4 backdrop-blur-md md:p-5"><div className="mb-5 flex items-center justify-between"><h2 className="text-base">Task activity</h2><FiActivity className="text-status-working"/></div><div className="min-h-64">{[...(data?.tasks || [])].reverse().slice(0, 6).map(t => <button key={t.id} className="flex w-full items-start justify-between gap-3 border-b border-border-chisel px-0 py-4 text-left" onClick={() => onNavigate('mesh')}><span className="min-w-0"><strong className="block truncate text-sm">{t.title}</strong><small className="mt-1 block font-mono text-[10px]">{t.kind.toUpperCase()}</small></span><span className="shrink-0 rounded border border-border-subtle bg-canvas-elevated px-2 py-1 font-mono text-[9px] uppercase">{t.status}</span></button>)}{!data?.tasks.length && <p className="py-12 text-sm text-ink-muted">No task records yet. Dispatch work to an approved machine from the network console.</p>}</div><button className="mt-4 min-h-11 w-full border border-border-subtle font-mono text-xs" onClick={() => onNavigate('mesh')}>QUERY_TASK_HISTORY <FiArrowUpRight /></button></section></div>
 <section className="mt-6 rounded-xl border border-border-subtle bg-canvas-surface/75 p-4 md:p-5"><div className="mb-4 flex items-center justify-between"><h2 className="text-base">Agent workbench</h2><button className="font-mono text-[10px]" onClick={() => onNavigate('machines')}>MANAGE_SESSIONS <FiArrowUpRight /></button></div><div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{conversations.slice(0, 6).map(c => <button key={c.id} className="flex min-h-20 items-center justify-between gap-4 rounded-xl border border-border-subtle bg-canvas-base/40 p-4 text-left" onClick={() => onOpenConversation(c.id)}><span className="min-w-0"><strong className="block truncate text-sm">{c.title}</strong><small className="mt-1 block font-mono text-[10px]">{c.is_group ? 'PROTECTED_CHANNEL' : 'DIRECT_SESSION'} · {c.is_active ? 'ACTIVE' : 'PAUSED'}</small></span><FiArrowUpRight /></button>)}{!conversations.length && <p className="py-6 text-sm">Initialize a local session in Machines or launch a sandboxed cloud agent from Network.</p>}</div></section>
 <div className="mt-6 flex flex-wrap items-center justify-between gap-4 border-t border-border-chisel py-4 font-mono text-[10px] text-ink-muted"><span>PROJECT RAIN / PRIVATE INFRASTRUCTURE OPERATIONS</span><button className="font-mono text-[10px]" onClick={() => onNavigate('guide')}>OPEN_OPERATIONS_GUIDE <FiArrowUpRight /></button></div>
 </div>;
}

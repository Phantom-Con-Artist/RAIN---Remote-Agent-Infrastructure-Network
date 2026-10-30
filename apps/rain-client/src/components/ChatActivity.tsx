import { useState, useEffect } from 'react';
import { apiRequest } from '../api';
import type { AgentSession } from '../types';
interface Dispatch {
    id: string;
    target_id: string;
    target_name: string;
    status: string;
    error?: string;
    queued_at: string;
}
interface Trace {
    id: string;
    dispatch_id: string;
    agent_id: string;
    step: string;
    timestamp: string;
}
export function ChatActivity({ id, members }: {
    id: string;
    members: AgentSession[];
}) {
    const [data, setData] = useState<{
        dispatches: Dispatch[];
        traces: Trace[];
    }>({ dispatches: [], traces: [] }), [error, setError] = useState(''), [busy, setBusy] = useState(''), [terminate, setTerminate] = useState<string | null>(null);
    useEffect(() => {
        let live = true;
        const controller = new AbortController();
        const load = () => apiRequest<typeof data>(`/api/v1/conversations/${id}/activity`, { signal: controller.signal }).then(r => {
            if (live) {
                setData(r.data);
                setError('');
            }
        }).catch(e => {
            if (live)
                setError(e.message);
        });
        load();
        const timer = setInterval(load, 2000);
        return () => { live = false; controller.abort(); clearInterval(timer); };
    }, [id]);
    async function control(agent: string, action: string) {
        setBusy(agent);
        setError('');
        try {
            await apiRequest(`/api/v1/conversations/${id}/agent-control`, { method: 'POST', body: JSON.stringify({ agent_id: agent, action }) });
            setTerminate(null);
        }
        catch (e) {
            setError(e instanceof Error ? e.message : 'Could not stop agent');
        }
        finally {
            setBusy('');
        }
    }
    const running = data.dispatches.filter(d => ['running', 'queued'].includes(d.status));
    return <details className="chat-activity shrink-0 border-b border-border-chisel bg-canvas-surface/35 px-4 [&>summary]:flex [&>summary]:items-center [&>summary]:gap-3 [&>summary]:text-xs [&>summary>span]:font-mono [&>summary>span]:text-ink-muted [&[open]]:max-h-[40vh] [&[open]]:overflow-y-auto"><summary>Activity <span>{running.length ? `${running.length} active ${running.length === 1 ? 'turn' : 'turns'}` : 'No active turns'}</span></summary>
 <p className="activity-help max-w-3xl py-2 text-xs">Tool events come from the installed CLI. Providers without tool events show turn status. Interrupt stops this chat’s turn; terminate stops the agent across all chats.</p>
 <div className="activity-members space-y-2 [&>div]:flex [&>div]:flex-wrap [&>div]:items-center [&>div]:gap-3 [&>div]:text-xs [&_strong]:mr-auto">{members.map(m => <div key={m.id}><strong>{m.name}</strong><span>{running.find(d => d.target_id === m.id)?.status || m.state}</span><button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy === m.id || !running.some(d => d.target_id === m.id)} onClick={() => control(m.id, 'interrupt')}>INTERRUPT_AGENT</button><button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy === m.id || ['stopped', 'completed', 'disconnected'].includes(m.state)} onClick={() => setTerminate(m.id)}>TERMINATE_AGENT</button></div>)}</div>
 {terminate && <div className="activity-confirm my-3 rounded-xl border border-status-danger/40 bg-status-danger/10 p-4 [&_p]:mb-3" role="alert"><p>Terminate {members.find(m => m.id === terminate)?.name}? This stops its process and work in every chat. You can launch a new session from Machines.</p><button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={!!busy} onClick={() => control(terminate, 'terminate')}>TERMINATE_AGENT</button><button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => setTerminate(null)}>KEEP_RUNNING</button></div>}
 {error && <p role="alert">{error}</p>}
 <ol className="activity-timeline space-y-3 py-4 font-mono text-xs [&_time]:text-ink-muted">{data.dispatches.slice(0, 12).map(d => <li key={d.id}><div><strong>{d.target_name}</strong> · {d.status} <time>{new Date(d.queued_at).toLocaleTimeString()}</time></div>{d.error && <p>{d.error}</p>}{data.traces.filter(t => t.dispatch_id === d.id).reverse().map(t => <div className="activity-step my-2 border-l border-border-subtle pl-4 text-ink-secondary" key={t.id}>{t.step}</div>)}</li>)}</ol>
 </details>;
}

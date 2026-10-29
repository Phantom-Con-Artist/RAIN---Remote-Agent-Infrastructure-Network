import { TerminalPanel } from './TerminalPanel';
import React, { useState, useEffect } from 'react';
import { apiRequest } from '../api';
import { AgentSession } from '../types';

export function AgentWorkbench({ sessions, onRefresh, onOpenChat }: {
  sessions: AgentSession[]; onRefresh: () => void; onOpenChat: (id: string) => Promise<void>;
}) {
  const [terminal, setTerminal] = useState<string | null>(null);
  const [interactive, setInteractive] = useState(true);
  const [machine, setMachine] = useState('');
  const [machines, setMachines] = useState<string[]>([]);
  useEffect(() => { apiRequest<string[]>('/api/v1/agents/machines').then(r => setMachines(r.data)).catch(e => setError(e.message)); }, []);
  const [command, setCommand] = useState('');
  const [args, setArgs] = useState('[]');
  const [cwd, setCwd] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  async function launch(e: React.FormEvent) {
    e.preventDefault(); setError(''); setBusy(true);
    try {
      const parsed = JSON.parse(args);
      if (!Array.isArray(parsed) || parsed.some(a => typeof a !== 'string')) throw new Error('Arguments must be a JSON array of strings.');
      await apiRequest('/api/v1/agents/start', { method: 'POST', body: JSON.stringify({
        command: command.trim(), args: parsed, name: name.trim() || command.trim(), provider: 'custom', interactive, machine: machine || null, cwd: cwd.trim() || null, env_vars: {},
      }) });
      onRefresh();
    } catch (e: any) { setError(e.message); }
    finally { setBusy(false); }
  }
  async function stop(id: string) {
    try { await apiRequest(`/api/v1/agents/${id}/stop`, { method: 'POST' }); onRefresh(); }
    catch (e: any) { setError(e.message); }
  }
  return <section className="agent-workbench" aria-label="Agent sessions">
    <h2>Agent sessions</h2>
    <p>Launch an installed CLI on this host. Interactive mode gives the CLI a terminal. Remote sessions use your configured SSH machines.</p>
    <form onSubmit={launch} className="agent-launch-form">
      <label>Machine<select value={machine} onChange={e => setMachine(e.target.value)}><option value="">This host</option>{machines.map(m => <option key={m}>{m}</option>)}</select></label>
      <label><span>Interactive terminal</span><input type="checkbox" checked={interactive} onChange={e => setInteractive(e.target.checked)} /></label>
      <label>Session name<input value={name} onChange={e => setName(e.target.value)} placeholder="Reviewer" /></label>
      <label>Executable<input required value={command} onChange={e => setCommand(e.target.value)} placeholder="Executable name or absolute path" /></label>
      <label>Arguments (JSON)<input required value={args} onChange={e => setArgs(e.target.value)} spellCheck={false} /></label>
      <label>Working directory<input value={cwd} onChange={e => setCwd(e.target.value)} placeholder="Host default directory" /></label>
      <button className="icon-button primary" disabled={busy}>{busy ? 'Launching…' : 'Launch session'}</button>
    </form>
    {error && <p role="alert">{error}</p>}
    <div className="agent-session-list">{sessions.map(s => <div className="agent-session-row" key={s.id}>
      <div><strong>{s.name}</strong><small>{s.cwd}</small></div><span>{s.state}</span>
      <button className="icon-button" onClick={() => setTerminal(s.id)}>Terminal</button>
      <button className="icon-button" onClick={() => onOpenChat(s.id).catch(e => setError(e.message))}>Open chat</button>
      {s.state === 'online' && <button className="icon-button" onClick={() => apiRequest(`/api/v1/agents/${s.id}/input`, {method:'POST', body: JSON.stringify({input:'\u0003'})}).catch(e => setError(e.message))}>Interrupt</button>}
      <button className="icon-button" disabled={s.state !== 'online'} onClick={() => stop(s.id)}>Stop</button>
    </div>)}</div>
    {terminal && <TerminalPanel id={terminal} onClose={() => setTerminal(null)} />}
    {!sessions.length && <p>No sessions yet. Launch a CLI to start a direct chat or add it to a group.</p>}
  </section>;
}

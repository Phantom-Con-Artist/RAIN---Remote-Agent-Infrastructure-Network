import React, {useEffect, useState} from 'react';
import {apiRequest} from '../api';

// Transcript view: strips display control sequences, while raw input reaches the PTY.
export function TerminalPanel({id, onClose}: {id: string; onClose: () => void}) {
  const [output, setOutput] = useState('');
  const [input, setInput] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const r = await apiRequest<{id: string; recent_logs: string[]}[]>('/api/v1/agents');
        if (active) setOutput((r.data.find(a => a.id === id)?.recent_logs || []).join(''));
      } catch(e: any) { if (active) setError(e.message); }
    };
    load(); const timer = setInterval(load, 1000);
    return () => { active = false; clearInterval(timer); };
  }, [id]);
  async function send(value: string) {
    try { await apiRequest(`/api/v1/agents/${id}/input`, {method:'POST', body:JSON.stringify({input:value})}); setError(''); }
    catch(e: any) { setError(e.message); throw e; }
  }
  return <section aria-label="Terminal transcript" className="agent-workbench">
    <button className="icon-button" onClick={onClose}>Close terminal</button>
    <pre style={{whiteSpace:'pre-wrap', overflowWrap:'anywhere', maxHeight:400, overflow:'auto'}}>{output.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '').replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, '')}</pre>
    <form onSubmit={async e => {e.preventDefault(); try {await send(input + '\r'); setInput('');} catch {}}}>
      <label>Terminal input<input className="chat-input-field" value={input} onChange={e => setInput(e.target.value)} /></label>
      <button className="icon-button" type="submit">Send</button>
    </form>
    <div style={{display:'flex', gap:8, flexWrap:'wrap', marginTop:12}}>{[['Interrupt','\x03'], ['Escape','\x1b'], ['Tab','\t'], ['Up','\x1b[A'], ['Down','\x1b[B'], ['Enter','\r']].map(([label,key]) => <button key={label} className="icon-button" onClick={() => send(key).catch(() => {})}>{label}</button>)}</div>
    {error && <p role="alert">{error}</p>}
  </section>;
}

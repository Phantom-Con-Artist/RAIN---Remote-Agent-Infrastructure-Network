import React, { useEffect, useRef, useState } from 'react';
import { apiRequest } from '../api';
import { FileEntry, ChatAttachment } from '../types';
import { AttachmentCard } from './AttachmentCard';
import { FilesView } from './FilesView';
import { MachinePicker } from './MachinePicker';
import { TerminalPanel } from './TerminalPanel';
export function FilesWorkspace({ machine, onMachineChange, onDirtyChange }: {
    machine: string;
    onMachineChange: (machine: string) => void;
    onDirtyChange?: (dirty: boolean) => void;
}) {
    const [path, setPath] = useState('~'), [files, setFiles] = useState<FileEntry[]>([]), [error, setError] = useState(''), [loading, setLoading] = useState(false), [terminal, setTerminal] = useState<string | null>(null);
    const [inspected, setInspected] = useState<ChatAttachment | null>(null), [inspecting, setInspecting] = useState(false);
    const [launching, setLaunching] = useState(false);
    const generation = useRef(0), inspectGeneration = useRef(0);
    const suffix = `machine=${encodeURIComponent(machine)}`;
    async function load(next: string) {
        const ticket = ++generation.current;
        setLoading(true);
        setError('');
        try {
            const r = await apiRequest<FileEntry[]>(`/api/v1/fs/list?path=${encodeURIComponent(next)}&${suffix}`);
            if (ticket === generation.current) {
                setPath(next);
                setFiles(r.data);
            }
        }
        catch (e: any) {
            if (ticket === generation.current)
                setError(e.message);
        }
        finally {
            if (ticket === generation.current)
                setLoading(false);
        }
    }
    useEffect(() => { inspectGeneration.current++; setInspecting(false); setFiles([]); setPath('~'); setTerminal(null); setInspected(null); load('~'); return () => { generation.current++; }; }, [machine]);
    async function launchTerminal() {
        if (launching)
            return;
        setLaunching(true);
        setError('');
        try {
            const catalog = await apiRequest<{
                shell: string | null;
            }>('/api/v1/agents/catalog?' + suffix);
            if (!catalog.data.shell)
                throw new Error('No shell found on this host');
            const r = await apiRequest<{
                id: string;
            }>('/api/v1/agents/start', { method: 'POST', body: JSON.stringify({ name: 'Terminal', provider: 'custom', command: catalog.data.shell, args: ['-l'], interactive: true, machine: machine || null, cwd: path === '~' ? null : path, env_vars: {} }) });
            setTerminal(r.data.id);
        }
        catch (e: any) {
            setError(e.message);
        }
        finally {
            setLaunching(false);
        }
    }
    return <div className="files-workspace mx-auto h-full w-full max-w-6xl overflow-y-auto px-4 py-6 md:px-6">
  <div className="workspace-toolbar mb-5 flex flex-wrap items-center gap-4 [&_h1]:mr-auto [&_h1]:text-2xl"><h1>Files</h1><MachinePicker machine={machine} onChange={onMachineChange}/><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={launching} onClick={launchTerminal}>{launching ? 'Launching…' : "OPEN_REMOTE_TERMINAL"}</button></div>
  {loading && <p role="status">Loading directory…</p>}{error && <p role="alert">{error}</p>}
  {terminal && <TerminalPanel id={terminal} onClose={() => setTerminal(null)}/>}
  {inspecting && <p role="status">Preparing file preview…</p>}
  {inspected && <section className="file-inspector-panel my-4 space-y-3 rounded-xl border border-border-subtle bg-canvas-elevated/70 p-4"><button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => setInspected(null)}>CLOSE_ARTIFACT</button><AttachmentCard key={inspected.id} file={inspected} initiallyOpen/></section>}
  <FilesView onInspectFile={async (file) => {
            if (inspecting)
                return;
            const ticket = ++inspectGeneration.current;
            setInspecting(true);
            setError('');
            try {
                const r = await apiRequest<ChatAttachment>('/api/v1/artifacts/from-file', { method: 'POST', body: JSON.stringify({ path: file.path, machine: machine || null }), signal: AbortSignal.timeout(300000) });
                if (ticket === inspectGeneration.current)
                    setInspected(r.data);
            }
            catch (e) {
                if (ticket === inspectGeneration.current)
                    setError(e instanceof Error ? e.message : 'Could not inspect file');
            }
            finally {
                if (ticket === inspectGeneration.current)
                    setInspecting(false);
            }
        }} onDirtyChange={onDirtyChange} key={machine} currentPath={path} files={files} onOpenDirectory={load} onReadFile={async (p) => {
            const r = await apiRequest<{
                content: string;
                is_binary: boolean;
            }>(`/api/v1/fs/read?path=${encodeURIComponent(p)}&${suffix}`);
            return r.data;
        }} onWriteFile={async (p, content) => { await apiRequest('/api/v1/fs/write?' + suffix, { method: 'POST', body: JSON.stringify({ path: p, content }) }); }}/>
 </div>;
}

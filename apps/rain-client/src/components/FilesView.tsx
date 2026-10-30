import React, { useState, useEffect } from 'react';
import { FiFolder, FiFileText, FiFile, FiRefreshCw, FiArrowLeft, FiSave } from 'react-icons/fi';
import { FileEntry } from '../types';
import { formatBytes } from '../api';
interface FilesViewProps {
    onInspectFile?: (file: FileEntry) => Promise<void>;
    onDirtyChange?: (dirty: boolean) => void;
    currentPath: string;
    files: FileEntry[];
    onOpenDirectory: (path: string) => Promise<void>;
    onReadFile: (path: string, name: string) => Promise<{
        content: string;
        is_binary: boolean;
    }>;
    onWriteFile: (path: string, content: string) => Promise<void>;
}
export const FilesView: React.FC<FilesViewProps> = ({ currentPath, onDirtyChange, files, onOpenDirectory, onReadFile, onInspectFile, onWriteFile, }) => {
    const [editingFile, setEditingFile] = useState<{
        path: string;
        name: string;
        content: string;
    } | null>(null);
    const [pathDraft, setPathDraft] = useState(currentPath);
    useEffect(() => setPathDraft(currentPath), [currentPath]);
    const [original, setOriginal] = useState('');
    const dirty = !!editingFile && editingFile.content !== original;
    useEffect(() => {
        onDirtyChange?.(dirty);
        const warn = (e: BeforeUnloadEvent) => {
            if (dirty) {
                e.preventDefault();
                e.returnValue = '';
            }
        };
        window.addEventListener('beforeunload', warn);
        return () => { window.removeEventListener('beforeunload', warn); onDirtyChange?.(false); };
    }, [dirty, onDirtyChange]);
    const [error, setError] = useState('');
    const [isSaving, setIsSaving] = useState(false);
    const handleFileClick = async (entry: FileEntry) => {
        setError('');
        try {
            if (entry.is_dir) {
                await onOpenDirectory(entry.path);
            }
            else if (entry.is_editable) {
                const res = await onReadFile(entry.path, entry.name);
                if (res.is_binary) {
                    await onInspectFile?.(entry);
                    return;
                }
                setOriginal(res.content);
                setEditingFile({ path: entry.path, name: entry.name, content: res.content });
            }
            else {
                await onInspectFile?.(entry);
            }
        }
        catch (e: any) {
            setError(e.message);
        }
    };
    const handleSave = async () => {
        if (!editingFile || isSaving)
            return;
        setIsSaving(true);
        try {
            await onWriteFile(editingFile.path, editingFile.content);
            setOriginal(editingFile.content);
            setError('');
        }
        catch (e: any) {
            setError(e.message);
        }
        finally {
            setIsSaving(false);
        }
    };
    if (editingFile) {
        return (<div className="files-screen min-w-0 [height:100%] [padding:16px]">
        <div className="[display:flex] [justify-content:space-between] [align-items:center]">
          <div className="[display:flex] [align-items:center] [gap:8px]">
            <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => {
                if (!dirty || window.confirm('Discard unsaved changes?'))
                    setEditingFile(null);
            }}>
              <FiArrowLeft size={14}/>
              <span>Back</span>
            </button>
            <span className="text-sm [font-weight:600]">{editingFile.name}{dirty ? ' · Unsaved' : ''}</span>
          </div>

          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" onClick={handleSave} disabled={isSaving}>
            <FiSave size={14}/>
            <span>{isSaving ? 'Saving...' : 'Save File'}</span>
          </button>
        </div>

        {error && <p role="alert">{error}</p>}
        <textarea style={{
                resize: 'none'
            }} spellCheck={false} value={editingFile.content} onChange={e => setEditingFile({ ...editingFile, content: e.target.value })} className="[flex:1] [width:100%] [background-color:var(--bg-surface)] [border:1px_solid_var(--border-subtle)] rounded-lg [padding:12px] font-mono text-xs [color:var(--text-primary)] [margin-top:10px]"/>
      </div>);
    }
    return (<div className="files-screen min-w-0">
      <div className="section-header-row mb-4 flex flex-wrap items-center justify-between gap-4">
        <h2 className="section-title text-lg font-medium tracking-tight">File explorer</h2>
        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => onOpenDirectory(currentPath)} title="Refresh directory">
          <FiRefreshCw size={13}/>
          <span>QUERY_TELEMETRY</span>
        </button>
      </div>

      {error && <p role="alert">{error}</p>}
      <form className="breadcrumb-strip mb-4 flex flex-wrap items-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 p-3 font-mono text-xs" onSubmit={e => { e.preventDefault(); onOpenDirectory(pathDraft); }}>
        <label htmlFor="explorer-path">Path</label>
        <input id="explorer-path" value={pathDraft} onChange={e => setPathDraft(e.target.value)} spellCheck={false}/>
        <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" type="submit">Go</button>
      </form>

      <div className="[display:flex] [flex-direction:column] [border:1px_solid_var(--border-subtle)] rounded-lg [overflow:hidden]">
        {files.map((file, idx) => (<div key={idx} className="file-row flex min-w-0 items-center gap-3 border-b border-border-chisel py-3" role="button" tabIndex={0} onKeyDown={e => {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleFileClick(file);
                }
            }} onClick={() => handleFileClick(file)}>
            <div className="file-meta-col min-w-0 flex-1 [&_strong]:block [&_strong]:truncate">
              <div style={{
                color: file.is_dir ? 'var(--accent-primary)' : 'var(--text-secondary)'
            }}>
                {file.is_dir ? <FiFolder size={16}/> : file.is_editable ? <FiFileText size={16}/> : <FiFile size={16}/>}
              </div>
              <div className="[display:flex] [flex-direction:column]">
                <span className="text-sm [font-weight:500] [color:var(--text-primary)]">
                  {file.name}
                </span>
                <span className="file-meta-sub font-mono text-xs text-ink-muted">
                  {file.is_dir ? 'Directory' : formatBytes(file.size_bytes)}
                  {file.is_editable ? ' · Open editor' : !file.is_dir ? ' · Preview or download' : ''}
                </span>
              </div>
            </div>

            {!file.is_dir && <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" type="button" onClick={e => { e.stopPropagation(); onInspectFile?.(file); }}>INSPECT_ARTIFACT</button>}
            {file.is_editable && (<span className="text-xs [padding:2px_6px] rounded-lg [border:1px_solid_var(--border-subtle)] [color:var(--text-muted)]">
                Edit
              </span>)}
          </div>))}

        {files.length === 0 && (<div className="[padding:24px] [text-align:center] [color:var(--text-muted)] text-sm">
            Directory is empty.
          </div>)}
      </div>
    </div>);
};

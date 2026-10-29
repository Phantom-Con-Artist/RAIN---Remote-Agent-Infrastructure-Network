import React, { useState, useEffect } from 'react';
import {
  FiFolder,
  FiFileText,
  FiFile,
  FiRefreshCw,
  FiArrowLeft,
  FiSave
} from 'react-icons/fi';
import { FileEntry } from '../types';
import { formatBytes } from '../api';

interface FilesViewProps {
  onDirtyChange?: (dirty:boolean)=>void;
  currentPath: string;
  files: FileEntry[];
  onOpenDirectory: (path: string) => Promise<void>;
  onReadFile: (path: string, name: string) => Promise<{ content: string; is_binary: boolean }>;
  onWriteFile: (path: string, content: string) => Promise<void>;
}

export const FilesView: React.FC<FilesViewProps> = ({
  currentPath,
  onDirtyChange,
  files,
  onOpenDirectory,
  onReadFile,
  onWriteFile,
}) => {
  const [editingFile, setEditingFile] = useState<{ path: string; name: string; content: string } | null>(null);
  const [pathDraft, setPathDraft] = useState(currentPath);
  useEffect(() => setPathDraft(currentPath), [currentPath]);
  const [original, setOriginal] = useState('');
  const dirty = !!editingFile && editingFile.content !== original;
  useEffect(()=>{onDirtyChange?.(dirty);const warn=(e:BeforeUnloadEvent)=>{if(dirty){e.preventDefault();e.returnValue=''}};window.addEventListener('beforeunload',warn);return()=>{window.removeEventListener('beforeunload',warn);onDirtyChange?.(false)}},[dirty,onDirtyChange]);
  const [error, setError] = useState('');
  const [isSaving, setIsSaving] = useState(false);

  const handleFileClick = async (entry: FileEntry) => {
    setError('');
    try {
    if (entry.is_dir) {
      await onOpenDirectory(entry.path);
    } else if (entry.is_editable) {
      const res = await onReadFile(entry.path, entry.name);
      if (res.is_binary) {
        alert('Binary files cannot be edited in the browser.');
        return;
      }
      setOriginal(res.content);
      setEditingFile({ path: entry.path, name: entry.name, content: res.content });
    }
    } catch(e: any) { setError(e.message); }
  };

  const handleSave = async () => {
    if (!editingFile || isSaving) return;
    setIsSaving(true);
    try {
      await onWriteFile(editingFile.path, editingFile.content);
      setOriginal(editingFile.content);
      setError('');
    } catch(e: any) { setError(e.message);
    } finally {
      setIsSaving(false);
    }
  };

  if (editingFile) {
    return (
      <div className="files-screen" style={{ height: '100%', padding: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <button className="icon-button" onClick={() => {if(!dirty||window.confirm('Discard unsaved changes?'))setEditingFile(null)}}>
              <FiArrowLeft size={14} />
              <span>Back</span>
            </button>
            <span style={{ fontSize: 13, fontWeight: 600 }}>{editingFile.name}{dirty?' · Unsaved':''}</span>
          </div>

          <button className="icon-button primary" onClick={handleSave} disabled={isSaving}>
            <FiSave size={14} />
            <span>{isSaving ? 'Saving...' : 'Save File'}</span>
          </button>
        </div>

        {error && <p role="alert">{error}</p>}
        <textarea
          style={{
            flex: 1,
            width: '100%',
            backgroundColor: 'var(--bg-surface)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-sm)',
            padding: 12,
            fontFamily: 'var(--font-mono)',
            fontSize: 12,
            color: 'var(--text-primary)',
            resize: 'none',
            marginTop: 10
          }}
          spellCheck={false}
          value={editingFile.content}
          onChange={e => setEditingFile({ ...editingFile, content: e.target.value })}
        />
      </div>
    );
  }

  return (
    <div className="files-screen">
      <div className="section-header-row">
        <h2 className="section-title">File explorer</h2>
        <button
          className="icon-button"
          onClick={() => onOpenDirectory(currentPath)}
          title="Refresh directory"
        >
          <FiRefreshCw size={13} />
          <span>Refresh</span>
        </button>
      </div>

      {error && <p role="alert">{error}</p>}
      <form className="breadcrumb-strip" onSubmit={e => {e.preventDefault(); onOpenDirectory(pathDraft);}}>
        <label htmlFor="explorer-path">Path</label>
        <input id="explorer-path" value={pathDraft} onChange={e => setPathDraft(e.target.value)} spellCheck={false}/>
        <button className="icon-button" type="submit">Go</button>
      </form>

      <div style={{ display: 'flex', flexDirection: 'column', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-sm)', overflow: 'hidden' }}>
        {files.map((file, idx) => (
          <div
            key={idx}
            className="file-row"
            role="button" tabIndex={0}
            onKeyDown={e => {if(e.key === 'Enter' || e.key === ' ') {e.preventDefault(); handleFileClick(file);}}}
            onClick={() => handleFileClick(file)}
          >
            <div className="file-meta-col">
              <div style={{ color: file.is_dir ? 'var(--accent-primary)' : 'var(--text-secondary)' }}>
                {file.is_dir ? <FiFolder size={16} /> : file.is_editable ? <FiFileText size={16} /> : <FiFile size={16} />}
              </div>
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                <span style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-primary)' }}>
                  {file.name}
                </span>
                <span className="file-meta-sub">
                  {file.is_dir ? 'Directory' : formatBytes(file.size_bytes)}
                  {file.is_editable ? ' • Tap to edit' : ''}
                </span>
              </div>
            </div>

            {file.is_editable && (
              <span
                style={{
                  fontSize: 11,
                  padding: '2px 6px',
                  borderRadius: 3,
                  border: '1px solid var(--border-subtle)',
                  color: 'var(--text-muted)'
                }}
              >
                Edit
              </span>
            )}
          </div>
        ))}

        {files.length === 0 && (
          <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-muted)', fontSize: 13 }}>
            Directory is empty.
          </div>
        )}
      </div>
    </div>
  );
};


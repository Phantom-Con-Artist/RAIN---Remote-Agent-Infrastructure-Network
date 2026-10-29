import React, { useState } from 'react';
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
  currentPath: string;
  files: FileEntry[];
  onOpenDirectory: (path: string) => Promise<void>;
  onReadFile: (path: string, name: string) => Promise<{ content: string; is_binary: boolean }>;
  onWriteFile: (path: string, content: string) => Promise<void>;
}

export const FilesView: React.FC<FilesViewProps> = ({
  currentPath,
  files,
  onOpenDirectory,
  onReadFile,
  onWriteFile,
}) => {
  const [editingFile, setEditingFile] = useState<{ path: string; name: string; content: string } | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const handleFileClick = async (entry: FileEntry) => {
    if (entry.is_dir) {
      await onOpenDirectory(entry.path);
    } else if (entry.is_editable) {
      const res = await onReadFile(entry.path, entry.name);
      if (res.is_binary) {
        alert('Binary files cannot be edited in the browser.');
        return;
      }
      setEditingFile({ path: entry.path, name: entry.name, content: res.content });
    }
  };

  const handleSave = async () => {
    if (!editingFile || isSaving) return;
    setIsSaving(true);
    try {
      await onWriteFile(editingFile.path, editingFile.content);
    } finally {
      setIsSaving(false);
    }
  };

  if (editingFile) {
    return (
      <div className="files-screen" style={{ height: '100%', padding: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <button className="icon-button" onClick={() => setEditingFile(null)}>
              <FiArrowLeft size={14} />
              <span>Back</span>
            </button>
            <span style={{ fontSize: 13, fontWeight: 600 }}>{editingFile.name}</span>
          </div>

          <button className="icon-button primary" onClick={handleSave} disabled={isSaving}>
            <FiSave size={14} />
            <span>{isSaving ? 'Saving...' : 'Save File'}</span>
          </button>
        </div>

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
            outline: 'none',
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
        <h2 className="section-title">Host File System</h2>
        <button
          className="icon-button"
          onClick={() => onOpenDirectory(currentPath)}
          title="Refresh directory"
        >
          <FiRefreshCw size={13} />
          <span>Refresh</span>
        </button>
      </div>

      <div className="breadcrumb-strip">
        <span>Path:</span>
        <span style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{currentPath}</span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-sm)', overflow: 'hidden' }}>
        {files.map((file, idx) => (
          <div
            key={idx}
            className="file-row"
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


import React, { useRef, useState, useEffect } from 'react';
import { FiPaperclip } from 'react-icons/fi';
import { apiRequest, getHostUrl, getAuthToken } from '../api';
import { ChatAttachment } from '../types';
import { MachinePicker } from './MachinePicker';
export function AttachmentPicker({ disabled, onAttach }: {
    disabled: boolean;
    onAttach: (attachment: ChatAttachment) => void;
}) {
    const fileInput = useRef<HTMLInputElement>(null), request = useRef<XMLHttpRequest | null>(null);
    const mounted = useRef(true);
    useEffect(() => { mounted.current = true; return () => { mounted.current = false; request.current?.abort(); }; }, []);
    const [hostPath, setHostPath] = useState(''), [machine, setMachine] = useState(''), [showHost, setShowHost] = useState(false), [error, setError] = useState(''), [busy, setBusy] = useState(false), [progress, setProgress] = useState('');
    async function upload(files: FileList) {
        setError('');
        setBusy(true);
        try {
            for (const file of Array.from(files)) {
                setProgress(`Uploading ${file.name}…`);
                const attachment = await new Promise<ChatAttachment>((resolve, reject) => {
                    const xhr = new XMLHttpRequest();
                    request.current = xhr;
                    xhr.open('POST', `${getHostUrl()}/api/v1/artifacts/upload?name=${encodeURIComponent(file.name)}`);
                    xhr.setRequestHeader('Content-Type', 'application/octet-stream');
                    if (getAuthToken())
                        xhr.setRequestHeader('X-Rain-Device-Token', getAuthToken());
                    xhr.upload.onprogress = e => {
                        if (e.lengthComputable)
                            setProgress(`${file.name} · ${Math.round(e.loaded / e.total * 100)}%`);
                    };
                    xhr.onload = () => {
                        try {
                            const response = JSON.parse(xhr.responseText);
                            if (xhr.status >= 400 || !response.success)
                                throw Error(response.error || 'Upload failed');
                            resolve(response.data);
                        }
                        catch (e) {
                            reject(e);
                        }
                    };
                    xhr.onerror = () => reject(Error('Upload interrupted. Choose the file again to retry.'));
                    xhr.onabort = () => reject(Error('Upload cancelled.'));
                    xhr.send(file);
                });
                if (mounted.current)
                    onAttach(attachment);
                else
                    break;
            }
        }
        catch (e) {
            setError(e instanceof Error ? e.message : 'Upload failed');
        }
        finally {
            request.current = null;
            setBusy(false);
            setProgress('');
            if (fileInput.current)
                fileInput.current.value = '';
        }
    }
    async function fromHost(e: React.FormEvent) {
        e.preventDefault();
        setError('');
        setBusy(true);
        setProgress('Preparing file…');
        try {
            const r = await apiRequest<ChatAttachment>('/api/v1/artifacts/from-file', { method: 'POST', body: JSON.stringify({ path: hostPath, machine: machine || null }), signal: AbortSignal.timeout(300000) });
            if (mounted.current)
                onAttach(r.data);
            setHostPath('');
            setShowHost(false);
        }
        catch (e) {
            setError(e instanceof Error ? e.message : 'Could not fetch file');
        }
        finally {
            setBusy(false);
            setProgress('');
        }
    }
    return <div className="attachment-picker space-y-3 p-3">
 <input ref={fileInput} type="file" hidden multiple onChange={e => {
            if (e.target.files?.length)
                upload(e.target.files);
        }}/>
 <button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={disabled || busy} onClick={() => fileInput.current?.click()}><FiPaperclip size={17}/><span>Upload files</span></button>
 <button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={disabled || busy} onClick={() => setShowHost(v => !v)}>Fetch from machine</button>
 {busy && <span role="status">{progress}</span>}{busy && request.current && <button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => request.current?.abort()}>Cancel upload</button>}
 {showHost && <form className="host-attachment-form flex flex-wrap gap-3 [&_label]:min-w-0 [&_label]:flex-1 [&_label]:text-xs" onSubmit={fromHost}><MachinePicker machine={machine} onChange={setMachine}/><label>File path<input className="chat-input-field min-h-11 w-full min-w-0 resize-none border-0! bg-transparent! px-3 py-2 text-base text-ink placeholder:text-ink-muted" value={hostPath} onChange={e => setHostPath(e.target.value)} required placeholder="Absolute path, home path, or selected host path"/></label><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={busy || !hostPath.trim()} type="submit">Fetch file</button><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" type="button" onClick={() => setShowHost(false)}>Close</button></form>}
 {error && <p role="alert">{error}</p>}
 </div>;
}
export function downloadAttachment(file: ChatAttachment) {
    if (!file.data_base64) {
        const a = document.createElement('a');
        a.href = `${getHostUrl()}/api/v1/artifacts/${encodeURIComponent(file.id)}/content?download=1`;
        a.download = file.name;
        a.click();
        return;
    }
    const bytes = Uint8Array.from(atob(file.data_base64), c => c.charCodeAt(0));
    const url = URL.createObjectURL(new Blob([bytes], { type: file.mime_type }));
    const a = document.createElement('a');
    a.href = url;
    a.download = file.name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
}

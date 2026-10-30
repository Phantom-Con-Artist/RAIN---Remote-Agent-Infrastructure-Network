import { useDialogFocus } from './useDialogFocus';
import React, { useState } from 'react';
import { FiUsers, FiX, FiCheck } from 'react-icons/fi';
import { AgentSession } from '../types';
interface NewGroupModalProps {
    isOpen: boolean;
    onClose: () => void;
    sessions: AgentSession[];
    onCreateGroup: (title: string, participantIds: string[]) => Promise<void>;
}
export const NewGroupModal: React.FC<NewGroupModalProps> = ({ isOpen, onClose, sessions, onCreateGroup, }) => {
    const dialogRef = useDialogFocus(isOpen, onClose);
    const [error, setError] = useState('');
    const [title, setTitle] = useState('');
    const [selectedIds, setSelectedIds] = useState<string[]>([]);
    const [isSubmitting, setIsSubmitting] = useState(false);
    const availableSessions = sessions.filter(s => ['online', 'working', 'thinking', 'waiting', 'responding'].includes(s.state));
    if (!isOpen)
        return null;
    const toggleSession = (id: string) => {
        setSelectedIds(prev => prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]);
    };
    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!title.trim() || isSubmitting)
            return;
        setError('');
        setIsSubmitting(true);
        try {
            await onCreateGroup(title.trim(), selectedIds);
            setTitle('');
            setSelectedIds([]);
            onClose();
        }
        catch (e: any) {
            setError(e.message);
        }
        finally {
            setIsSubmitting(false);
        }
    };
    return (<div ref={dialogRef} className="modal-overlay fixed inset-0 z-50 flex items-center justify-center overflow-y-auto bg-black/60 p-4 backdrop-blur-md" role="dialog" aria-modal="true" aria-label="New group conversation">
      <div className="modal-card w-full max-w-lg overflow-hidden rounded-2xl border border-border-subtle bg-canvas-surface/95 shadow-2xl backdrop-blur-xl">
        <div className="modal-header flex min-h-16 items-center justify-between gap-4 border-b border-border-chisel bg-canvas-elevated/50 px-5 py-3">
          <div className="[display:flex] [align-items:center] [gap:8px]">
            <FiUsers size={16} className="[color:var(--accent-primary)]"/>
            <span className="modal-title text-base font-medium">New Group Conversation</span>
          </div>
          <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" aria-label="Close group dialog" onClick={onClose}>
            <FiX size={15}/>
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body max-h-[70dvh] space-y-4 overflow-y-auto p-5">
            <div>
              <label className="[display:block] text-xs [font-weight:600] [color:var(--text-secondary)] [margin-bottom:6px]">
                Group Title
              </label>
              <input type="text" className="chat-input-field min-h-11 w-full min-w-0 resize-none border-0! bg-transparent! px-3 py-2 text-base text-ink placeholder:text-ink-muted" placeholder="e.g. Infrastructure Maintenance, Code Review..." value={title} onChange={e => setTitle(e.target.value)} autoFocus/>
            </div>

            {availableSessions.length > 0 && (<div>
                <label className="[display:block] text-xs [font-weight:600] [color:var(--text-secondary)] [margin-bottom:6px]">
                  Select Agent Participants
                </label>
                <div style={{
                maxHeight: 180
            }} className="[display:flex] [flex-direction:column] [gap:6px] [overflow-y:auto]">
                  {availableSessions.map(s => {
                const isChecked = selectedIds.includes(s.id);
                return (<button type="button" aria-pressed={isChecked} key={s.id} onClick={() => toggleSession(s.id)} style={{
                        backgroundColor: isChecked ? 'var(--bg-active)' : 'var(--bg-subtle)',
                        cursor: 'pointer'
                    }} className="[display:flex] [align-items:center] [justify-content:space-between] [padding:8px_12px] rounded-lg [border:1px_solid_var(--border-subtle)]">
                        <span className="text-sm [color:var(--text-primary)]">{s.name}</span>
                        {isChecked && <FiCheck size={14} className="[color:var(--accent-primary)]"/>}
                      </button>);
            })}
                </div>
              </div>)}
          </div>

          {error && <p role="alert" className="[padding:0_24px] [color:var(--danger-text)]">{error}</p>}
          <div className="modal-footer flex flex-wrap justify-end gap-3 border-t border-border-chisel px-5 py-4">
            <button type="button" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={onClose}>
              CANCEL
            </button>
            <button type="submit" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={!title.trim() || isSubmitting}>
              <span>{isSubmitting ? 'Creating...' : 'Create Group'}</span>
            </button>
          </div>
        </form>
      </div>
    </div>);
};

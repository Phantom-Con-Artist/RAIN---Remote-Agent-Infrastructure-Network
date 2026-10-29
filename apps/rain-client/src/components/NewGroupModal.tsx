import React, { useState } from 'react';
import { FiUsers, FiX, FiCheck } from 'react-icons/fi';
import { AgentSession } from '../types';

interface NewGroupModalProps {
  isOpen: boolean;
  onClose: () => void;
  sessions: AgentSession[];
  onCreateGroup: (title: string, participantIds: string[]) => Promise<void>;
}

export const NewGroupModal: React.FC<NewGroupModalProps> = ({
  isOpen,
  onClose,
  sessions,
  onCreateGroup,
}) => {
  const [title, setTitle] = useState('');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const toggleSession = (id: string) => {
    setSelectedIds(prev =>
      prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || isSubmitting) return;

    setIsSubmitting(true);
    try {
      await onCreateGroup(title.trim(), selectedIds);
      setTitle('');
      setSelectedIds([]);
      onClose();
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-card">
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <FiUsers size={16} style={{ color: 'var(--accent-primary)' }} />
            <span className="modal-title">New Group Conversation</span>
          </div>
          <button className="icon-button" onClick={onClose}>
            <FiX size={15} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            <div>
              <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>
                Group Title
              </label>
              <input
                type="text"
                className="chat-input-field"
                placeholder="e.g. Infrastructure Maintenance, Code Review..."
                value={title}
                onChange={e => setTitle(e.target.value)}
                autoFocus
              />
            </div>

            {sessions.length > 0 && (
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>
                  Select Agent Participants
                </label>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 180, overflowY: 'auto' }}>
                  {sessions.map(s => {
                    const isChecked = selectedIds.includes(s.id);
                    return (
                      <div
                        key={s.id}
                        onClick={() => toggleSession(s.id)}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          padding: '8px 12px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: isChecked ? 'var(--bg-active)' : 'var(--bg-subtle)',
                          border: '1px solid var(--border-subtle)',
                          cursor: 'pointer'
                        }}
                      >
                        <span style={{ fontSize: 13, color: 'var(--text-primary)' }}>{s.name}</span>
                        {isChecked && <FiCheck size={14} style={{ color: 'var(--accent-primary)' }} />}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          <div className="modal-footer">
            <button type="button" className="icon-button" onClick={onClose}>
              Cancel
            </button>
            <button
              type="submit"
              className="icon-button primary"
              disabled={!title.trim() || isSubmitting}
            >
              <span>{isSubmitting ? 'Creating...' : 'Create Group'}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

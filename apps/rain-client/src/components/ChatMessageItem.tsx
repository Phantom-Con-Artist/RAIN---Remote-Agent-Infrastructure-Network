import React, { useState } from 'react';
import { FiCheck, FiX, FiLock, FiBookmark, FiTag, FiStar } from 'react-icons/fi';
import type { ChatMessage, AgentSession } from '../types';
import { formatTime } from '../api';
import { AttachmentCard } from './AttachmentCard';
import { MessageText } from './MessageText';

export interface ChatMessageItemProps {
  message: ChatMessage;
  sessions: AgentSession[];
  onApproveJob: (id: string, approved: boolean) => Promise<void>;
  onTogglePin?: (id: string, pinned: boolean) => void;
  onToggleBookmark?: (id: string, bookmarked: boolean, note?: string) => void;
  onUpdateTags?: (id: string, tags: string[]) => void;
}

export function ChatMessageItem({
  message: msg,
  sessions,
  onApproveJob,
  onTogglePin,
  onToggleBookmark,
  onUpdateTags,
}: ChatMessageItemProps) {
  const [showTagInput, setShowTagInput] = useState(false);
  const [newTagText, setNewTagText] = useState('');
  const [showNoteInput, setShowNoteInput] = useState(false);
  const [noteText, setNoteText] = useState(msg.bookmark_note || '');

  const session = sessions.find(a => a.id === msg.sender_id);
  const human = msg.sender_type === 'human';
  const system = msg.sender_type === 'system';
  const accent = session?.provider.includes('claude')
    ? 'border-l-status-warning'
    : session?.provider.includes('agy')
    ? 'border-l-status-working'
    : 'border-l-status-online';

  const handleAddTag = (e: React.FormEvent) => {
    e.preventDefault();
    const tag = newTagText.trim().toLowerCase();
    if (!tag) {
      setShowTagInput(false);
      return;
    }
    const current = msg.tags || [];
    if (!current.includes(tag) && onUpdateTags) {
      onUpdateTags(msg.id, [...current, tag]);
    }
    setNewTagText('');
    setShowTagInput(false);
  };

  const handleRemoveTag = (tagToRemove: string) => {
    if (onUpdateTags) {
      onUpdateTags(msg.id, (msg.tags || []).filter(t => t !== tagToRemove));
    }
  };

  const handleSaveNote = (e: React.FormEvent) => {
    e.preventDefault();
    if (onToggleBookmark) {
      onToggleBookmark(msg.id, true, noteText.trim() || undefined);
    }
    setShowNoteInput(false);
  };

  return (
    <div
      className={`message-row group flex w-full flex-col gap-1.5 ${
        human ? 'human items-end' : system ? 'system items-center' : 'agent items-start'
      }`}
    >
      {!human && !system && (
        <div className="message-sender mb-1 flex max-w-full flex-wrap items-center gap-2 text-xs text-ink-secondary">
          <strong>{msg.sender_name}</strong>
          <span className="rounded border border-border-subtle bg-canvas-surface/70 px-2 py-1 font-mono text-[9px] uppercase">
            {session?.provider.replace('_chat', '') || 'AGENT'}
          </span>
          <span className="font-mono text-[9px] text-ink-muted">
            {session?.machine_name || 'HUB'} · MODEL: {session?.model || 'NOT_REPORTED'}
          </span>
        </div>
      )}

      {/* Badges for Pin and Bookmark */}
      {(msg.is_pinned || msg.is_bookmarked) && (
        <div className="flex flex-wrap items-center gap-2 px-1 text-[11px]">
          {msg.is_pinned && (
            <span className="inline-flex items-center gap-1 rounded bg-amber-500/20 px-2 py-0.5 font-mono text-[10px] text-amber-300 border border-amber-500/30 shadow-sm">
              <FiStar size={11} className="fill-amber-400 text-amber-400" /> PINNED
            </span>
          )}
          {msg.is_bookmarked && (
            <span
              className="inline-flex items-center gap-1 rounded bg-indigo-500/20 px-2 py-0.5 font-mono text-[10px] text-indigo-300 border border-indigo-500/30 shadow-sm cursor-pointer hover:bg-indigo-500/30 transition-colors"
              title={msg.bookmark_note || 'Bookmarked message'}
              onClick={() => {
                setNoteText(msg.bookmark_note || '');
                setShowNoteInput(prev => !prev);
              }}
            >
              <FiBookmark size={11} className="fill-indigo-400 text-indigo-400" />
              {msg.bookmark_note ? `NOTE: ${msg.bookmark_note}` : 'BOOKMARK'}
            </span>
          )}
        </div>
      )}

      <div
        data-sender={msg.sender_type}
        className={`message-bubble relative max-w-[94%] whitespace-pre-wrap break-words rounded-2xl px-4 py-3 text-sm leading-7 md:max-w-[86%] ${
          human
            ? 'rounded-br-sm border border-transparent bg-bubble-human text-bubble-human-ink'
            : system
            ? 'border-2 border-bubble-sudo-border bg-bubble-sudo text-ink'
            : 'rounded-bl-sm border border-bubble-agent-border border-l-[3px] bg-bubble-agent text-bubble-agent-ink ' +
              accent
        }`}
      >
        {msg.coworker && msg.coworker.kind !== 'discuss' && (
          <div className="mb-2 flex flex-wrap gap-3 font-mono text-[10px] uppercase text-current/80">
            <strong>{msg.coworker.kind.replace('_', ' ')}</strong>
            {msg.coworker.to.length > 0 && (
              <span>
                TO{' '}
                {msg.coworker.to
                  .map(id => (id === 'all' ? 'ALL' : sessions.find(s => s.id === id)?.name || id))
                  .join(', ')}
              </span>
            )}
          </div>
        )}
        {msg.coworker?.flair && (
          <span className="mb-2 inline-flex rounded border border-current/20 px-2 py-1 text-xs">
            {msg.coworker.flair}
          </span>
        )}
        {system && msg.sender_name.includes('CLI') ? (
          <details>
            <summary className="font-mono text-xs">CLI_RESULT</summary>
            <pre className="max-h-96 text-xs">{msg.text}</pre>
          </details>
        ) : (
          <MessageText text={msg.text} />
        )}
        {msg.attachments?.map(file => (
          <AttachmentCard key={file.id} file={file} />
        ))}
        {msg.job_delegation && (
          <div className="mt-4 space-y-3 rounded-xl border border-border-subtle bg-canvas-base/60 p-4 text-ink">
            <div className="flex flex-wrap items-center justify-between gap-3 font-mono text-[10px]">
              <span>
                {msg.job_delegation.source_machine} → {msg.job_delegation.target_machine}
              </span>
              <span className="rounded border border-border-subtle bg-canvas-elevated px-2 py-1 uppercase">
                {msg.job_delegation.status.replaceAll('_', ' ')}
              </span>
            </div>
            <p className="text-sm text-ink">{msg.job_delegation.task_instruction}</p>
            {msg.job_delegation.required_resources.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {msg.job_delegation.required_resources.map((r, i) => (
                  <span key={i} className="rounded border border-border-subtle px-2 py-1 font-mono text-[10px]">
                    {r}
                  </span>
                ))}
              </div>
            )}
            {msg.job_delegation.status === 'requires_approval' && !msg.job_delegation.is_approved && (
              <div className="rounded-xl border-2 border-bubble-sudo-border bg-bubble-sudo p-4">
                <div className="mb-3 flex items-center gap-2 font-mono text-xs text-ink">
                  <FiLock className="text-status-warning" />
                  PRIVILEGED_EXECUTION_REQUEST
                </div>
                <pre className="my-3 border-border-subtle bg-[#080a0f] text-xs text-white">
                  <code>{msg.job_delegation.sudo_command || 'Command not reported'}</code>
                </pre>
                <div className="flex flex-wrap gap-3">
                  <button
                    className="min-h-11 border border-status-online/40 bg-status-online/15 font-mono text-[10px] text-ink"
                    onClick={() => onApproveJob(msg.job_delegation!.id, true)}
                  >
                    <FiCheck />
                    AUTHORIZE_EXECUTION
                  </button>
                  <button
                    className="min-h-11 border border-status-danger/40 bg-status-danger/10 font-mono text-[10px] text-ink"
                    onClick={() => onApproveJob(msg.job_delegation!.id, false)}
                  >
                    <FiX />
                    REJECT_REQUEST
                  </button>
                </div>
              </div>
            )}
            {msg.job_delegation.result_summary && (
              <p className="rounded-lg border border-status-online/30 bg-status-online/10 p-3 text-xs text-ink">
                {msg.job_delegation.result_summary}
              </p>
            )}
          </div>
        )}

        {/* Tags */}
        {msg.tags && msg.tags.length > 0 && (
          <div className="mt-2.5 flex flex-wrap items-center gap-1.5 border-t border-border-subtle/40 pt-2">
            {msg.tags.map(t => (
              <span
                key={t}
                className="inline-flex items-center gap-1 rounded bg-canvas-surface/80 px-2 py-0.5 font-mono text-[10px] text-ink border border-border-subtle"
              >
                #{t}
                {onUpdateTags && (
                  <button
                    type="button"
                    onClick={() => handleRemoveTag(t)}
                    className="text-ink-muted hover:text-status-danger ml-0.5"
                    title={`Remove tag #${t}`}
                  >
                    ×
                  </button>
                )}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Note Edit Modal/Inline Input */}
      {showNoteInput && (
        <form onSubmit={handleSaveNote} className="flex items-center gap-2 px-2 py-1 text-xs">
          <input
            type="text"
            className="rounded border border-indigo-500/50 bg-canvas-surface px-2 py-1 font-mono text-[11px] text-ink outline-none"
            placeholder="Add a bookmark note..."
            value={noteText}
            onChange={e => setNoteText(e.target.value)}
            autoFocus
          />
          <button
            type="submit"
            className="rounded bg-indigo-600 px-2.5 py-1 font-mono text-[10px] text-white hover:bg-indigo-500"
          >
            Save Note
          </button>
          <button
            type="button"
            className="text-ink-muted hover:text-ink text-xs"
            onClick={() => setShowNoteInput(false)}
          >
            Cancel
          </button>
        </form>
      )}

      {/* Tag Add Inline Form */}
      {showTagInput && (
        <form onSubmit={handleAddTag} className="flex items-center gap-2 px-2 py-1 text-xs">
          <input
            type="text"
            className="rounded border border-border-subtle bg-canvas-surface px-2 py-1 font-mono text-[11px] text-ink outline-none"
            placeholder="tag name (e.g. todo, bug)..."
            value={newTagText}
            onChange={e => setNewTagText(e.target.value)}
            autoFocus
          />
          <button
            type="submit"
            className="rounded bg-canvas-elevated px-2 py-1 font-mono text-[10px] text-ink hover:border-border-muted"
          >
            Add
          </button>
          <button
            type="button"
            className="text-ink-muted hover:text-ink text-xs"
            onClick={() => setShowTagInput(false)}
          >
            Cancel
          </button>
        </form>
      )}

      {/* Message action controls on hover & timestamp */}
      <div className="message-time flex items-center gap-2.5 font-mono text-[10px] text-ink-muted">
        <time dateTime={msg.timestamp}>{formatTime(msg.timestamp)}</time>
        {human && (
          <span className="message-delivery inline-flex" aria-label={msg.status}>
            {msg.status === 'failed' ? (
              'FAILED'
            ) : msg.status === 'sent' ? (
              <FiCheck size={12} />
            ) : (
              <span className={`inline-flex -space-x-1 ${msg.status === 'read' ? 'text-sky-400' : ''}`}>
                <FiCheck size={12} />
                <FiCheck size={12} />
              </span>
            )}
          </span>
        )}

        {/* Action icons */}
        <div className="opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1.5 ml-1">
          {onTogglePin && (
            <button
              type="button"
              onClick={() => onTogglePin(msg.id, !msg.is_pinned)}
              className={`p-1 rounded hover:bg-canvas-elevated transition-colors ${
                msg.is_pinned ? 'text-amber-400' : 'text-ink-muted hover:text-amber-300'
              }`}
              title={msg.is_pinned ? 'Unpin message' : 'Pin message'}
            >
              <FiStar size={12} className={msg.is_pinned ? 'fill-amber-400' : ''} />
            </button>
          )}

          {onToggleBookmark && (
            <button
              type="button"
              onClick={() => {
                if (msg.is_bookmarked) {
                  onToggleBookmark(msg.id, false);
                } else {
                  setShowNoteInput(true);
                }
              }}
              className={`p-1 rounded hover:bg-canvas-elevated transition-colors ${
                msg.is_bookmarked ? 'text-indigo-400' : 'text-ink-muted hover:text-indigo-300'
              }`}
              title={msg.is_bookmarked ? 'Remove bookmark' : 'Bookmark message'}
            >
              <FiBookmark size={12} className={msg.is_bookmarked ? 'fill-indigo-400' : ''} />
            </button>
          )}

          {onUpdateTags && (
            <button
              type="button"
              onClick={() => setShowTagInput(prev => !prev)}
              className="p-1 rounded hover:bg-canvas-elevated text-ink-muted hover:text-ink transition-colors"
              title="Add / edit tags"
            >
              <FiTag size={12} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

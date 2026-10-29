import { TerminalPanel } from './TerminalPanel';
import React, { useState, useRef, useEffect } from 'react';
import {
  FiPlus,
  FiSend,
  FiArrowLeft,
  FiTrash2,
  FiStopCircle,
  FiUsers,
  FiUser,
  FiCheck,
  FiX,
  FiAlertTriangle,
  FiActivity,
  FiPaperclip
} from 'react-icons/fi';
import { Conversation, ChatMessage, AgentSession, ChatAttachment } from '../types';
import { formatTime } from '../api';

interface ChatsViewProps {
  conversations: Conversation[];
  activeConversationId: string | null;
  onSelectConversation: (id: string) => void;
  messages: ChatMessage[];
  sessions: AgentSession[];
  onSendMessage: (text: string, targetAgentId: string | null, attachments: ChatAttachment[]) => Promise<void>;
  onOpenNewGroup: () => void;
  onStopConversation: (id: string) => Promise<void>;
  onDeleteConversation: (id: string) => Promise<void>;
  onApproveJob: (jobId: string, approved: boolean) => Promise<void>;
  onTriggerBatteryEvacuate: () => Promise<void>;
  isMobileChatOpen: boolean;
  onCloseMobileChat: () => void;
}

export const ChatsView: React.FC<ChatsViewProps> = ({
  conversations,
  activeConversationId,
  onSelectConversation,
  messages,
  sessions,
  onSendMessage,
  onOpenNewGroup,
  onStopConversation,
  onDeleteConversation,
  onApproveJob,
  onTriggerBatteryEvacuate,
  isMobileChatOpen,
  onCloseMobileChat,
}) => {
  const [terminalId, setTerminalId] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'groups' | 'direct'>('all');
  const [inputText, setInputText] = useState('');
  const [targetSessionId, setTargetSessionId] = useState<string>('all');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const activeConv = conversations.find(c => c.id === activeConversationId) || null;

  const interactiveMembers = sessions.filter(s => s.interactive && activeConv?.participant_ids.includes(s.id));
  const directTerminal = activeConv && !activeConv.is_group ? interactiveMembers[0]?.id : undefined;
  useEffect(() => setTerminalId(directTerminal || null), [activeConv?.id, directTerminal]);
  const visibleMessages = messages.filter(m => m.sender_type === 'human' || !m.text.includes('\x1b'));

  const filteredConvs = conversations.filter(c => {
    if (filter === 'groups') return c.is_group;
    if (filter === 'direct') return !c.is_group;
    return true;
  });

  // Smooth scroll to bottom on new messages if near bottom
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const isNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 120;
    if (isNearBottom) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages.length]);

  const handleSend = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const text = inputText.trim();
    if (!text || isSubmitting || !activeConversationId) return;

    setIsSubmitting(true);
    try {
      await onSendMessage(text, targetSessionId === 'all' ? null : targetSessionId, []);
      setInputText('');
    } catch {
      // Keep the draft so a failed delivery can be retried.
    } finally {
      setIsSubmitting(false);
    }
  };

  const handlePromptClick = (prompt: string) => {
    if (prompt.includes('5%') || prompt.includes('Battery')) {
      onTriggerBatteryEvacuate();
    } else {
      setInputText(prompt);
    }
  };

  return (
    <div className={`chats-layout ${isMobileChatOpen ? 'mobile-chat-active' : 'mobile-roster-active'}`}>
      {/* 1. Conversations Roster Column */}
      <aside className="roster-column">
        <div className="roster-header">
          <div className="roster-filter-pills">
            <button
              className={`filter-tab ${filter === 'all' ? 'active' : ''}`}
              onClick={() => setFilter('all')}
            >
              All
            </button>
            <button
              className={`filter-tab ${filter === 'groups' ? 'active' : ''}`}
              onClick={() => setFilter('groups')}
            >
              Groups
            </button>
            <button
              className={`filter-tab ${filter === 'direct' ? 'active' : ''}`}
              onClick={() => setFilter('direct')}
            >
              Direct
            </button>
          </div>

          <button className="icon-button primary" onClick={onOpenNewGroup} title="New conversation">
            <FiPlus size={14} />
            <span>New</span>
          </button>
        </div>

        <div className="conversations-scroll">
          {filteredConvs.map(conv => (
            <div
              key={conv.id}
              className={`conversation-row ${conv.id === activeConversationId ? 'active' : ''}`}
              onClick={() => onSelectConversation(conv.id)}
            >
              <div className="conv-avatar-box">
                {conv.is_group ? <FiUsers size={16} /> : <FiUser size={16} />}
              </div>
              <div className="conv-info-col">
                <div className="conv-title-row">
                  <span className="conv-title">{conv.title}</span>
                  <span className="conv-time">{formatTime(conv.last_message_time)}</span>
                </div>
                <div className="conv-preview">{conv.last_message?.includes('\x1b') ? 'Terminal session' : conv.last_message || 'No messages yet'}</div>
              </div>
            </div>
          ))}

          {filteredConvs.length === 0 && (
            <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-muted)', fontSize: 13 }}>
              No conversations. Tap "+ New" to begin.
            </div>
          )}
        </div>
      </aside>

      {/* 2. Active Chat Panel */}
      <section className="chat-column">
        {activeConv ? (
          <>
            <div className="chat-header-bar">
              <div className="chat-header-left">
                <button
                  className="icon-button"
                  onClick={onCloseMobileChat}
                  title="Back to conversations"
                  style={{ display: isMobileChatOpen ? 'flex' : 'none' }}
                >
                  <FiArrowLeft size={16} />
                </button>

                <div className="chat-header-titles">
                  <span className="chat-header-title">{activeConv.title}</span>
                  <span className="chat-header-meta">
                    {activeConv.is_group
                      ? `${activeConv.participant_ids.length} members`
                      : '1-on-1 Direct session'}
                    {!activeConv.is_active && ' • (Paused)'}
                  </span>
                </div>
              </div>

              <div className="header-actions">
                {interactiveMembers.length > 0 && <select aria-label="Open agent terminal" value={terminalId || ''} onChange={e => setTerminalId(e.target.value || null)}><option value="">Conversation</option>{interactiveMembers.map(s=><option key={s.id} value={s.id}>{s.name} terminal</option>)}</select>}
                {activeConv.is_active && (
                  <button
                    className="icon-button"
                    onClick={() => onStopConversation(activeConv.id)}
                    title="Pause conversation session"
                  >
                    <FiStopCircle size={14} />
                    <span>Pause</span>
                  </button>
                )}
                <button
                  className="icon-button"
                  onClick={() => onDeleteConversation(activeConv.id)}
                  title="Delete conversation"
                >
                  <FiTrash2 size={14} />
                </button>
              </div>
            </div>

            {/* Messages Feed */}
            {terminalId ? <div className="chat-terminal"><TerminalPanel key={terminalId} id={terminalId}/></div> : <div className="chat-messages-scroll" ref={containerRef}>
              {visibleMessages.map(msg => (
                <div key={msg.id} className={`message-row ${msg.sender_type}`}>
                  {msg.sender_type !== 'human' && msg.sender_type !== 'system' && (
                    <span className="message-sender">{msg.sender_name}</span>
                  )}

                  <div className="message-bubble">
                    {msg.text}

                    {/* Job Delegation & Work Unit */}
                    {msg.job_delegation && (
                      <div className="delegation-card">
                        <div className="delegation-header">
                          <span className="delegation-route">
                            {msg.job_delegation.source_machine} → {msg.job_delegation.target_machine}
                          </span>
                          <span className={`status-badge ${msg.job_delegation.status}`}>
                            {msg.job_delegation.status.replace('_', ' ')}
                          </span>
                        </div>

                        <div style={{ fontSize: 13, color: 'var(--text-primary)' }}>
                          {msg.job_delegation.task_instruction}
                        </div>

                        {msg.job_delegation.required_resources.length > 0 && (
                          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                            {msg.job_delegation.required_resources.map((r, i) => (
                              <span
                                key={i}
                                style={{
                                  fontSize: 10,
                                  fontFamily: 'var(--font-mono)',
                                  padding: '2px 6px',
                                  borderRadius: 3,
                                  border: '1px solid var(--border-subtle)',
                                  backgroundColor: 'var(--bg-subtle)'
                                }}
                              >
                                {r}
                              </span>
                            ))}
                          </div>
                        )}

                        {/* Privileged Approval Action */}
                        {msg.job_delegation.status === 'requires_approval' && !msg.job_delegation.is_approved && (
                          <div
                            style={{
                              marginTop: 6,
                              padding: 10,
                              backgroundColor: 'var(--warning-surface)',
                              border: '1px solid var(--warning-border)',
                              borderRadius: 'var(--radius-sm)'
                            }}
                          >
                            <div style={{ fontSize: 12, color: 'var(--warning-text)', marginBottom: 6 }}>
                              Privileged execution requested:
                            </div>
                            <code
                              style={{
                                display: 'block',
                                padding: '4px 6px',
                                backgroundColor: 'var(--bg-canvas)',
                                borderRadius: 3,
                                fontFamily: 'var(--font-mono)',
                                fontSize: 11,
                                marginBottom: 8
                              }}
                            >
                              {msg.job_delegation.sudo_command || 'sudo task'}
                            </code>
                            <div style={{ display: 'flex', gap: 8 }}>
                              <button
                                className="icon-button primary"
                                onClick={() => onApproveJob(msg.job_delegation!.id, true)}
                              >
                                <FiCheck size={13} />
                                <span>Authorize</span>
                              </button>
                              <button
                                className="icon-button"
                                onClick={() => onApproveJob(msg.job_delegation!.id, false)}
                              >
                                <FiX size={13} />
                                <span>Reject</span>
                              </button>
                            </div>
                          </div>
                        )}

                        {msg.job_delegation.result_summary && (
                          <div
                            style={{
                              fontSize: 12,
                              color: 'var(--success-text)',
                              padding: '6px 8px',
                              backgroundColor: 'var(--success-surface)',
                              borderRadius: 4
                            }}
                          >
                            {msg.job_delegation.result_summary}
                          </div>
                        )}
                      </div>
                    )}
                  </div>

                  <span className="message-time">{formatTime(msg.timestamp)}</span>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>}

            {/* Quick Prompt Bar */}
            <div className="quick-prompts-row">
              <button
                className="quick-action-btn evacuate"
                onClick={() => handlePromptClick('5% Battery Evacuate')}
              >
                <FiAlertTriangle size={13} />
                <span>5% Battery Evacuate to GitHub</span>
              </button>
              <button
                className="quick-action-btn"
                onClick={() => handlePromptClick('Check system health and machine load')}
              >
                <FiActivity size={13} />
                <span>Check machine load</span>
              </button>
              <button
                className="quick-action-btn"
                onClick={() => handlePromptClick('git diff --stat')}
              >
                <span>Review git status</span>
              </button>
            </div>

            {/* Message Input Toolbar */}
            {!terminalId && <form className="chat-input-bar" onSubmit={handleSend}>
              <select
                value={targetSessionId}
                onChange={e => setTargetSessionId(e.target.value)}
                style={{
                  height: 36,
                  padding: '0 8px',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--bg-subtle)',
                  border: '1px solid var(--border-subtle)',
                  color: 'var(--text-primary)',
                  fontSize: 12
                }}
              >
                <option value="all">@All</option>
                {sessions.filter(s => activeConv.participant_ids.includes(s.id)).map(s => (
                  <option key={s.id} value={s.id}>
                    @{s.name}
                  </option>
                ))}
              </select>

              <input
                type="text"
                className="chat-input-field"
                placeholder={activeConv.is_active ? 'Message, @session, or /command…' : 'Conversation is paused'}
                disabled={!activeConv.is_active || isSubmitting}
                value={inputText}
                onChange={e => setInputText(e.target.value)}
              />

              <button
                type="submit"
                className="icon-button primary"
                disabled={!activeConv.is_active || !inputText.trim() || isSubmitting}
                title="Send message"
              >
                <FiSend size={15} />
              </button>
            </form>}
          </>
        ) : (
          <div
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--text-muted)',
              gap: 12
            }}
          >
            <FiUsers size={36} />
            <span style={{ fontSize: 14 }}>Select a conversation or create a new group.</span>
          </div>
        )}
      </section>
    </div>
  );
};


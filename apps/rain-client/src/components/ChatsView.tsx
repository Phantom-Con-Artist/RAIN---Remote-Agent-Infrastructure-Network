import { ChatMessageItem } from './ChatMessageItem';
import { MessageText } from './MessageText';
import { ChatComposer } from './ChatComposer';
import { ChatActivity } from './ChatActivity';
import { AttachmentPicker } from './AttachmentPicker';
import { TerminalPanel } from './TerminalPanel';
import { AttachmentCard } from './AttachmentCard';
import { GroupAwareness } from './GroupAwareness';
import React, { useState, useRef, useEffect } from 'react';
import { FiPlus, FiSend, FiArrowLeft, FiTrash2, FiStopCircle, FiUsers, FiUser, FiCheck, FiX, FiActivity, FiPaperclip } from 'react-icons/fi';
import { Conversation, ChatMessage, AgentSession, ChatAttachment } from '../types';
import { apiRequest, formatTime } from '../api';
interface ChatsViewProps {
    conversations: Conversation[];
    activeConversationId: string | null;
    onSelectConversation: (id: string) => void;
    messages: ChatMessage[];
    sessions: AgentSession[];
    onStartDirect: (id: string) => Promise<void>;
    onSendMessage: (text: string, targetAgentId: string | null, attachments: ChatAttachment[]) => Promise<void>;
    onOpenNewGroup: () => void;
    onStopConversation: (id: string) => Promise<void>;
    onResumeConversation: (id: string) => Promise<void>;
    onCollaborationChange: (id: string, enabled: boolean, maxTurns: number) => Promise<void>;
    onDeleteConversation: (id: string) => Promise<void>;
    onApproveJob: (jobId: string, approved: boolean) => Promise<void>;
    onTriggerBatteryEvacuate: () => Promise<void>;
    isMobileChatOpen: boolean;
    onCloseMobileChat: () => void;
}
export const ChatsView: React.FC<ChatsViewProps> = ({ conversations, activeConversationId, onSelectConversation, messages, sessions, onSendMessage, onStartDirect, onOpenNewGroup, onStopConversation, onResumeConversation, onCollaborationChange, onDeleteConversation, onApproveJob, isMobileChatOpen, onCloseMobileChat, }) => {
    const [pendingFiles, setPendingFiles] = useState<ChatAttachment[]>([]);
    const [attachmentError, setAttachmentError] = useState('');
    const [controlError, setControlError] = useState('');
    const [controlBusy, setControlBusy] = useState(false);
    const runControl = async (action: () => Promise<void>) => {
        setControlBusy(true);
        setControlError('');
        try {
            await action();
        }
        catch (error) {
            setControlError(error instanceof Error ? error.message : 'Could not update group. Try again.');
        }
        finally {
            setControlBusy(false);
        }
    };
    const [search, setSearch] = useState('');
    const [contactError, setContactError] = useState('');
    const [terminalId, setTerminalId] = useState<string | null>(null);
    const [filter, setFilter] = useState<'all' | 'groups' | 'direct'>('all');
    const [inputText, setInputText] = useState('');
    const [targetSessionId, setTargetSessionId] = useState<string>('all');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const pinnedToBottom = useRef(true);
    const currentId = useRef(activeConversationId);
    currentId.current = activeConversationId;
    const drafts = useRef<Record<string, {
        text: string;
        files: ChatAttachment[];
    }>>({});
    const previousId = useRef<string | null>(null);
    const draftState = useRef({ text: inputText, files: pendingFiles });
    const activeConv = conversations.find(c => c.id === activeConversationId) || null;
    const directMember = !activeConv?.is_group ? sessions.find(s => activeConv?.participant_ids.includes(s.id)) : undefined;
    const interactiveMembers = sessions.filter(s => s.interactive && activeConv?.participant_ids.includes(s.id));
    useEffect(() => {
        pinnedToBottom.current = true;
        setTerminalId(null);
        setTargetSessionId('all');
        if (previousId.current)
            drafts.current[previousId.current] = draftState.current;
        const draft = activeConv ? drafts.current[activeConv.id] : undefined;
        setInputText(draft?.text || '');
        setPendingFiles(draft?.files || []);
        previousId.current = activeConv?.id || null;
        setAttachmentError('');
        setControlError('');
    }, [activeConv?.id]);
    useEffect(() => { draftState.current = { text: inputText, files: pendingFiles }; }, [inputText, pendingFiles]);
    const workingMembers = sessions.filter(s => activeConv?.participant_ids.includes(s.id) && ['working', 'thinking', 'responding'].includes(s.state));
    const visibleMessages = messages.filter(m => m.sender_type === 'human' || !m.text.includes('\x1b'));
    const filteredConvs = conversations.filter(c => {
        if (search && !(`${c.title} ${c.last_message || ''}`).toLowerCase().includes(search.toLowerCase()))
            return false;
        if (filter === 'groups')
            return c.is_group;
        if (filter === 'direct')
            return !c.is_group;
        return true;
    });
    // Smooth scroll to bottom on new messages if near bottom
    useEffect(() => {
        const container = containerRef.current;
        if (!container)
            return;
        if (pinnedToBottom.current) {
            messagesEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        }
    }, [messages.length]);
    const handleSend = async (e?: React.FormEvent) => {
        if (e)
            e.preventDefault();
        const text = inputText.trim();
        if ((!text && !pendingFiles.length) || isSubmitting || !activeConversationId)
            return;
        const sendingId = activeConversationId;
        setIsSubmitting(true);
        try {
            await onSendMessage(text || 'Shared files.', targetSessionId === 'all' ? null : targetSessionId, pendingFiles);
            delete drafts.current[sendingId];
            if (currentId.current === sendingId) {
                setPendingFiles([]);
                setInputText('');
            }
        }
        catch {
            // Keep the draft so a failed delivery can be retried.
        }
        finally {
            setIsSubmitting(false);
        }
    };
    const handlePromptClick = (prompt: string) => setInputText(prompt);
    const [modeValue, setModeValue] = useState('casual');
    const [settingsOpen, setSettingsOpen] = useState(false);
    useEffect(() => { const media = window.matchMedia('(min-width: 601px)'); const change = () => setSettingsOpen(false); media.addEventListener('change', change); return () => media.removeEventListener('change', change); }, []);
    useEffect(() => setModeValue(activeConv?.collaboration?.mode || 'casual'), [activeConv?.id, activeConv?.collaboration?.mode]);
    const changeMode = (mode: string) => runControl(async () => { await apiRequest(`/api/v1/conversations/${activeConv!.id}/mode`, { method: 'PATCH', body: JSON.stringify({ mode }) }); setModeValue(mode); });
    return (<div className={`chats-layout flex h-full min-h-0 w-full overflow-hidden [&.mobile-chat-active>.roster-column]:hidden [&.mobile-roster-active>.chat-column]:hidden md:[&.mobile-chat-active>.roster-column]:flex md:[&.mobile-roster-active>.chat-column]:flex ${isMobileChatOpen ? "mobile-chat-active" : "mobile-roster-active"}`}>
      {/* 1. Conversations Roster Column */}
      <aside className="roster-column flex h-full min-h-0 w-full shrink-0 flex-col border-r border-border-subtle bg-canvas-surface/75 md:w-72 xl:w-80">
        <div className="messenger-heading flex h-20 shrink-0 items-center justify-between gap-3 px-4 [&_h1]:text-lg [&_button]:text-xs"><h1>Conversations</h1><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={onOpenNewGroup}><FiPlus size={18}/>CREATE_GROUP</button></div>
        <label className="conversation-search px-4 [&_input]:w-full"><span className="sr-only">Search conversations</span><input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search chats and agents"/></label>
        <div className="roster-header shrink-0 px-4 py-3">
          <div className="roster-filter-pills flex gap-1 rounded-xl bg-canvas-base p-1">
            <button className={`filter-tab flex-1 rounded-lg text-xs text-ink-muted [&.active]:bg-canvas-elevated [&.active]:text-ink ${filter === "all" ? "active" : ""}`} onClick={() => setFilter('all')}>
              All
            </button>
            <button className={`filter-tab flex-1 rounded-lg text-xs text-ink-muted [&.active]:bg-canvas-elevated [&.active]:text-ink ${filter === "groups" ? "active" : ""}`} onClick={() => setFilter('groups')}>
              Groups
            </button>
            <button className={`filter-tab flex-1 rounded-lg text-xs text-ink-muted [&.active]:bg-canvas-elevated [&.active]:text-ink ${filter === "direct" ? "active" : ""}`} onClick={() => setFilter('direct')}>
              Direct
            </button>
          </div>


        </div>

        <div className="conversations-scroll min-h-0 flex-1 overflow-y-auto p-2">
          {filteredConvs.map(conv => (<button type="button" key={conv.id} className={`conversation-row flex w-full items-center gap-3 rounded-xl border border-transparent p-3 text-left [&.active]:border-border-glow [&.active]:bg-canvas-hover ${conv.id === activeConversationId ? "active" : ""}`} onClick={() => onSelectConversation(conv.id)}>
              <div className="conv-avatar-box flex size-10 shrink-0 items-center justify-center rounded-xl border border-border-subtle bg-canvas-elevated text-ink-secondary">
                {conv.is_group ? <FiUsers size={16}/> : <FiUser size={16}/>}
              </div>
              <div className="conv-info-col min-w-0 flex-1">
                <div className="conv-title-row flex items-center justify-between gap-2">
                  <span className="conv-title truncate text-sm font-medium text-ink">{conv.title}</span>
                  <span className="conv-time shrink-0 font-mono text-[10px] text-ink-muted">{formatTime(conv.last_message_time)}</span>
                </div>
                <div className="conv-preview mt-1 truncate text-xs text-ink-muted">{conv.last_message?.includes('\x1b') ? 'Terminal session' : conv.last_message || 'No messages yet'}</div>
              </div>
            </button>))}

          {filteredConvs.length === 0 && (<div className="[padding:24px] [text-align:center] [color:var(--text-muted)] text-sm">
              No matching chats. Start a conversation with an agent below.
            </div>)}
          <div className="agent-contacts mt-4 space-y-2 border-t border-border-subtle px-2 py-4 [&>h3]:font-mono [&>h3]:text-xs [&>h3]:uppercase [&>h3]:tracking-wider [&>p]:py-4 [&>p]:text-xs"><h3>Agents</h3>{sessions.filter(s => !['stopped', 'completed', 'disconnected'].includes(s.state) && s.name.toLowerCase().includes(search.toLowerCase())).map(s => <button type="button" className="agent-contact flex w-full items-center gap-3 px-2 py-3 text-left [&_strong]:block [&_strong]:text-sm [&_small]:block" key={s.id} onClick={() => onStartDirect(s.id).catch(e => setContactError(e.message))}><span className="contact-avatar flex size-9 shrink-0 items-center justify-center rounded-lg bg-status-working/10 font-mono text-xs text-status-working">{s.name.slice(0, 2).toUpperCase()}</span><span><strong>{s.name}</strong><small>{s.machine_name} · {s.interactive ? 'Terminal mode' : s.state}</small></span></button>)}{!sessions.some(s => !['stopped', 'completed', 'disconnected'].includes(s.state)) && <p>Start an agent from Machines to begin chatting.</p>}{contactError && <p role="alert">{contactError}</p>}</div>
        </div>
      </aside>

      {/* 2. Active Chat Panel */}
      <section className="chat-column relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {activeConv ? (<>
            <div className="chat-header-bar z-10 flex flex-wrap md:flex-nowrap min-h-20 shrink-0 items-center justify-between gap-3 border-b border-border-chisel bg-canvas-surface/65 px-4 py-3 backdrop-blur-md md:px-6">
              <div className="chat-header-left flex w-full md:w-auto min-w-0 items-center gap-3">
                <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={onCloseMobileChat} title="Back to conversations" style={{
                display: isMobileChatOpen ? 'flex' : 'none'
            }}>
                  <FiArrowLeft size={16}/>
                </button>

                <div className="chat-header-titles min-w-0">
                  <span className="chat-header-title block truncate text-base font-medium">{activeConv.title}</span>
                  <span className="chat-header-meta block font-mono text-[10px] text-ink-muted">
                    {activeConv.is_group
                ? `${activeConv.participant_ids.length} members`
                : '1-on-1 Direct session'}
                    {!activeConv.is_active && ' • (Paused)'}
                  </span>
                </div>
              </div>

              <div className="header-actions flex max-w-full flex-nowrap items-center gap-2 overflow-x-auto md:overflow-visible">
                {directMember?.provider.endsWith('_chat') && <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={controlBusy} onClick={() => runControl(async () => {
                    const r = await apiRequest<{
                        id: string;
                    }>(`/api/v1/conversations/${activeConv.id}/native-terminal`, { method: 'POST' });
                    setTerminalId(r.data.id);
                })}>OPEN_NATIVE_CLI</button>}
                {terminalId && <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => setTerminalId(null)}>OPEN_CHANNEL</button>}
                {interactiveMembers.length > 0 && <select aria-label="Open agent terminal" value={terminalId || ''} onChange={e => setTerminalId(e.target.value || null)}><option value="">Conversation</option>{interactiveMembers.map(s => <option key={s.id} value={s.id}>{s.name} terminal</option>)}</select>}
                {!activeConv.is_active && <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" disabled={controlBusy} onClick={() => runControl(() => onResumeConversation(activeConv.id))}>RESUME_CHANNEL</button>}
                {activeConv.is_active && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => onStopConversation(activeConv.id)} title="Pause conversation session">
                    <FiStopCircle size={14}/>
                    <span>PAUSE_CHANNEL</span>
                  </button>)}
                <details className="chat-more relative [&>summary]:rounded-lg [&>summary]:border [&>summary]:border-border-subtle [&>summary]:px-3 [&[open]]:z-30 [&[open]>div]:absolute [&[open]>div]:right-0 [&[open]>div]:min-w-56 [&[open]>div]:rounded-xl [&[open]>div]:border [&[open]>div]:border-border-subtle [&[open]>div]:bg-canvas-elevated [&[open]>div]:p-3"><summary>OPERATIONS</summary><div><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => onDeleteConversation(activeConv.id)} title="Delete conversation">
                  <FiTrash2 size={14}/> Delete conversation
                </button></div></details>
              </div>
            </div>

            {activeConv.is_group && <details className="coworker-controls shrink-0 border-b border-border-chisel px-4 [&_summary]:text-xs [&[open]]:max-h-[35vh] [&[open]]:overflow-y-auto" open={settingsOpen} onToggle={e => setSettingsOpen(e.currentTarget.open)}><summary>Channel settings · {modeValue.charAt(0).toUpperCase() + modeValue.slice(1)}</summary>
              <div className="coworker-mode flex flex-wrap items-center gap-3 py-3 text-xs">
                <label>Mode <select aria-label="Conversation mode" value={modeValue} disabled={controlBusy} onChange={e => changeMode(e.target.value)}><option value="casual">Casual</option><option value="goal">Goal</option><option value="evidence">Evidence</option></select></label>
                <label><input type="checkbox" checked={activeConv.collaboration?.enabled ?? false} disabled={controlBusy} onChange={e => runControl(() => onCollaborationChange(activeConv.id, e.target.checked, activeConv.collaboration?.max_turns ?? 0))}/>
                  Autonomous channel</label>
                <label>Discussion <select aria-label="Channel discussion limit" value={activeConv.collaboration?.max_turns ?? 0} disabled={controlBusy} onChange={e => runControl(() => onCollaborationChange(activeConv.id, activeConv.collaboration?.enabled ?? true, Number(e.target.value)))}>
                  {[0, 4, 8, 16, 32].map(n => <option key={n} value={n}>{n === 0 ? 'Continuous' : `${n} replies`}</option>)}
                  {activeConv.collaboration && ![0, 4, 8, 16, 32].includes(activeConv.collaboration.max_turns) && <option value={activeConv.collaboration.max_turns}>{activeConv.collaboration.max_turns}</option>}
                </select></label>
              </div>
              {modeValue === 'evidence' && <p>Verify claims independently, cite sources, and challenge unsupported agreement. Sources remain open to review.</p>}
              {modeValue === 'goal' && <p>Work toward your stated outcome, review results, and surface decisions that need you.</p>}
              {activeConv.collaboration?.human_goal && <details className="coworker-brief py-4 text-xs text-ink-secondary"><summary>Channel context</summary><p>{activeConv.collaboration.human_goal}</p></details>}
              <p>{activeConv.collaboration?.enabled
                    ? activeConv.collaboration.max_turns === 0 ? 'Open conversation · members may reply in parallel. Mention an agent to direct a message.' : `Open conversation · ${activeConv.collaboration.remaining_turns} replies remain in this session.`
                    : 'Autonomous discussion is off. Your messages still reach the selected agents.'}</p>
              {activeConv.collaboration?.waiting_for_human && <div className="coworker-human-request rounded-lg border border-status-warning/30 bg-bubble-sudo p-3 text-sm" role="status">
                <details><summary>Your input is needed</summary><p>{activeConv.collaboration.human_request}</p></details>
                <span>Reply below to continue the discussion.</span>
              </div>}
            </details>}
            {controlError && <p className="coworker-control-error" role="alert">{controlError}</p>}

            {directMember?.provider.endsWith('_chat') && <GroupAwareness key={`group-awareness:${activeConv.id}`} agentId={directMember.id}/>}
            <ChatActivity key={`activity:${activeConv.id}`} id={activeConv.id} members={sessions.filter(s => activeConv.participant_ids.includes(s.id))}/>
            {/* Messages Feed */}
            {terminalId ? <div className="chat-terminal min-h-0 flex-1 overflow-auto p-4"><TerminalPanel key={terminalId} id={terminalId} onClose={() => setTerminalId(null)}/></div> : <div className="chat-messages-scroll flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto overscroll-contain px-4 py-6 md:px-8 [&>*]:mx-auto [&>*]:w-full [&>*]:max-w-4xl" ref={containerRef} onScroll={e => { const el = e.currentTarget; pinnedToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 120; }}>
              {interactiveMembers.length > 0 && <div className="native-chat-notice rounded-xl border border-status-warning/30 bg-bubble-sudo p-4 text-xs">{interactiveMembers.map(s => s.name).join(', ')} uses terminal mode. Open its terminal above, or start a chat adapter from Machines for message replies.</div>}
              {visibleMessages.map((msg, index) => <React.Fragment key={msg.id}>{(index === 0 || new Date(visibleMessages[index - 1].timestamp).toDateString() !== new Date(msg.timestamp).toDateString()) && <div className="epoch-divider flex items-center justify-center py-3"><span className="rounded-full border border-border-chisel bg-canvas-surface/70 px-4 py-2 font-mono text-[10px] tracking-wider text-ink-muted">{new Date(msg.timestamp).toDateString() === new Date().toDateString() ? 'TODAY' : new Date(msg.timestamp).toLocaleDateString(undefined, { day: '2-digit', month: 'short', year: 'numeric' }).toUpperCase()}</span></div>}<ChatMessageItem message={msg} sessions={sessions} onApproveJob={onApproveJob}/></React.Fragment>)}
              {workingMembers.length > 0 && <div className="typing-state flex items-center gap-3 py-2 font-mono text-xs text-ink-muted" role="status"><span className="inline-flex items-center gap-1" aria-hidden="true">{[0, 1, 2].map(i => <i key={i} className="size-1.5 rounded-full bg-status-working motion-safe:animate-agent" style={{ animationDelay: `${i * 150}ms` }}/>)}</span>{workingMembers.map(s => s.name).join(', ')} is executing a turn…</div>}
              <div ref={messagesEndRef}/>
            </div>}

            {/* Quick Prompt Bar */}
            <div className="quick-prompts-row mx-auto flex w-full max-w-4xl shrink-0 gap-2 overflow-x-auto px-4 py-2">
              <button className="quick-action-btn min-h-11 shrink-0 rounded-full border border-border-subtle bg-canvas-surface/70 px-4 font-mono text-[10px] text-ink-secondary" onClick={() => handlePromptClick('Probe approved peers and report actual reachability. Do not invent telemetry.')}>PROBE_PEERS</button>
              <button className="quick-action-btn min-h-11 shrink-0 rounded-full border border-border-subtle bg-canvas-surface/70 px-4 font-mono text-[10px] text-ink-secondary" onClick={() => handlePromptClick('Check system health and machine load')}>
                <FiActivity size={13}/>
                <span>CHECK_MACHINE_LOAD</span>
              </button>
              <button className="quick-action-btn min-h-11 shrink-0 rounded-full border border-border-subtle bg-canvas-surface/70 px-4 font-mono text-[10px] text-ink-secondary" onClick={() => handlePromptClick('git diff --stat')}>
                <span>INSPECT_GIT_STATUS</span>
              </button>
            </div>

            {!terminalId && <details className="attachment-toolbar mx-auto w-full max-w-4xl shrink-0 px-4 [&>summary]:flex [&>summary]:items-center [&>summary]:gap-2 [&>summary]:text-xs"><summary><FiPaperclip size={14}/>Attach files{pendingFiles.length ? ` · ${pendingFiles.length}` : ''}</summary><AttachmentPicker key={activeConv.id} disabled={!activeConv.is_active || isSubmitting || false} onAttach={file => { setPendingFiles(previous => [...previous, file]); setAttachmentError(''); }}/>{pendingFiles.map(file => <div className="pending-attachment flex items-center justify-between gap-3 text-xs" key={file.id}><span>{file.name}</span><button type="button" aria-label={`Remove attachment ${file.name}`} onClick={() => setPendingFiles(files => files.filter(f => f.id !== file.id))}>REMOVE</button></div>)}{attachmentError && <p role="alert">{attachmentError}</p>}</details>}
            {/* Message Input Toolbar */}
            {!terminalId && <form className="chat-input-bar mx-auto mb-4 flex w-[calc(100%-2rem)] max-w-4xl shrink-0 items-end gap-2 rounded-2xl border border-border-subtle bg-canvas-surface/90 p-2 backdrop-blur-xl [&>select]:max-w-24 [&>select]:text-xs [&>button[type=submit]]:size-11 [&>button[type=submit]]:rounded-full [&>button[type=submit]]:border-status-online [&>button[type=submit]]:bg-status-online [&>button[type=submit]]:text-white" onSubmit={handleSend}>
              <select aria-label="Message recipient" value={targetSessionId} onChange={e => setTargetSessionId(e.target.value)} className="[height:36px] [padding:0_8px] rounded-lg [background-color:var(--bg-subtle)] [border:1px_solid_var(--border-subtle)] [color:var(--text-primary)] text-xs">
                <option value="all">@All</option>
                {sessions.filter(s => activeConv.participant_ids.includes(s.id)).map(s => (<option key={s.id} value={s.id}>
                    @{s.name}
                  </option>))}
              </select>

              <button type="button" className="size-11 shrink-0 rounded-xl text-ink-secondary" aria-label="Attach artifact" onClick={() => { const panel = document.querySelector<HTMLDetailsElement>('.attachment-toolbar'); if (panel)
                panel.open = !panel.open; }}><FiPaperclip size={18}/></button>
              <ChatComposer key={activeConv.id} id={activeConv.id} members={sessions.filter(s => activeConv.participant_ids.includes(s.id))} group={activeConv.is_group} value={inputText} onChange={setInputText} onSend={() => handleSend()} disabled={!activeConv.is_active || isSubmitting} placeholder={!activeConv.is_active ? 'Conversation is paused' : activeConv.is_group ? 'Message this channel…' : 'Message or /help…'}/>

              <button type="submit" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={!activeConv.is_active || (!inputText.trim() && !pendingFiles.length) || isSubmitting} title="Dispatch message" aria-label="Dispatch message">
                <FiSend size={15}/>
              </button>
            </form>}
          </>) : (<div className="[flex:1] [display:flex] [flex-direction:column] [align-items:center] [justify-content:center] [color:var(--text-muted)] [gap:12px]">
            <FiUsers size={36}/>
            <div className="messenger-empty max-w-md space-y-4 text-center [&_h2]:text-xl [&_p]:text-sm"><h2>Your agents, one conversation away</h2><p>Choose an agent to start a chat, or create a group to work with several agents together.</p><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" onClick={onOpenNewGroup}>CREATE_GROUP</button></div>
          </div>)}
      </section>
    </div>);
};

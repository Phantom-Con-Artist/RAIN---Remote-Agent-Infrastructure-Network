import {MessageText} from './MessageText';
import {ChatComposer} from './ChatComposer';
import {ChatActivity} from './ChatActivity';
import {AttachmentPicker} from './AttachmentPicker';
import { TerminalPanel } from './TerminalPanel';
import {AttachmentCard} from './AttachmentCard';
import { GroupAwareness } from './GroupAwareness';
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
  FiActivity, FiPaperclip
} from 'react-icons/fi';
import { Conversation, ChatMessage, AgentSession, ChatAttachment } from '../types';
import { apiRequest, formatTime } from '../api';

interface ChatsViewProps {
  conversations: Conversation[];
  activeConversationId: string | null;
  onSelectConversation: (id: string) => void;
  messages: ChatMessage[];
  sessions: AgentSession[];
  onStartDirect: (id:string) => Promise<void>;
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

export const ChatsView: React.FC<ChatsViewProps> = ({
  conversations,
  activeConversationId,
  onSelectConversation,
  messages,
  sessions,
  onSendMessage,
  onStartDirect,
  onOpenNewGroup,
  onStopConversation,
  onResumeConversation,
  onCollaborationChange,
  onDeleteConversation,
  onApproveJob,

  isMobileChatOpen,
  onCloseMobileChat,
}) => {
  const [pendingFiles,setPendingFiles]=useState<ChatAttachment[]>([]);
  const [attachmentError,setAttachmentError]=useState('');
  const [controlError, setControlError] = useState('');
  const [controlBusy, setControlBusy] = useState(false);
  const runControl = async (action: () => Promise<void>) => {
    setControlBusy(true); setControlError('');
    try { await action(); } catch (error) { setControlError(error instanceof Error ? error.message : 'Could not update group. Try again.'); }
    finally { setControlBusy(false); }
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

  const currentId=useRef(activeConversationId);currentId.current=activeConversationId;
  const drafts=useRef<Record<string,{text:string;files:ChatAttachment[]}>>({});
  const previousId=useRef<string|null>(null);
  const draftState=useRef({text:inputText,files:pendingFiles});
  const activeConv = conversations.find(c => c.id === activeConversationId) || null;

  const directMember = !activeConv?.is_group ? sessions.find(s => activeConv?.participant_ids.includes(s.id)) : undefined;
  const interactiveMembers = sessions.filter(s => s.interactive && activeConv?.participant_ids.includes(s.id));
  useEffect(() => {pinnedToBottom.current=true;setTerminalId(null); setTargetSessionId('all'); if(previousId.current)drafts.current[previousId.current]=draftState.current;const draft=activeConv?drafts.current[activeConv.id]:undefined;setInputText(draft?.text||'');setPendingFiles(draft?.files||[]);previousId.current=activeConv?.id||null;setAttachmentError('');setControlError('');}, [activeConv?.id]);
  useEffect(()=>{draftState.current={text:inputText,files:pendingFiles}},[inputText,pendingFiles]);
  const workingMembers = sessions.filter(s=>activeConv?.participant_ids.includes(s.id)&&['working','thinking','responding'].includes(s.state));
  const visibleMessages = messages.filter(m => m.sender_type === 'human' || !m.text.includes('\x1b'));

  const filteredConvs = conversations.filter(c => {
    if (search && !(`${c.title} ${c.last_message||''}`).toLowerCase().includes(search.toLowerCase())) return false;
    if (filter === 'groups') return c.is_group;
    if (filter === 'direct') return !c.is_group;
    return true;
  });

  // Smooth scroll to bottom on new messages if near bottom
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    if (pinnedToBottom.current) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth', block:'nearest' });
    }
  }, [messages.length]);

  const handleSend = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const text = inputText.trim();
    if ((!text&&!pendingFiles.length) || isSubmitting || !activeConversationId) return;

    const sendingId=activeConversationId;
    setIsSubmitting(true);
    try {
      await onSendMessage(text||'Shared files.', targetSessionId === 'all' ? null : targetSessionId, pendingFiles);
      delete drafts.current[sendingId];
      if(currentId.current===sendingId){setPendingFiles([]);setInputText('');}
    } catch {
      // Keep the draft so a failed delivery can be retried.
    } finally {
      setIsSubmitting(false);
    }
  };

  const handlePromptClick = (prompt:string) => setInputText(prompt);
  const [modeValue,setModeValue]=useState('casual');
  const [settingsOpen,setSettingsOpen]=useState(false);
  useEffect(()=>{const media=window.matchMedia('(min-width: 601px)');const change=()=>setSettingsOpen(false);media.addEventListener('change',change);return()=>media.removeEventListener('change',change)},[]);
  useEffect(()=>setModeValue(activeConv?.collaboration?.mode||'casual'),[activeConv?.id,activeConv?.collaboration?.mode]);
  const changeMode=(mode:string)=>runControl(async()=>{await apiRequest(`/api/v1/conversations/${activeConv!.id}/mode`,{method:'PATCH',body:JSON.stringify({mode})});setModeValue(mode)});

  return (
    <div className={`chats-layout ${isMobileChatOpen ? 'mobile-chat-active' : 'mobile-roster-active'}`}>
      {/* 1. Conversations Roster Column */}
      <aside className="roster-column">
        <div className="messenger-heading"><h1>Conversations</h1><button className="icon-button" onClick={onOpenNewGroup}><FiPlus size={18}/>New group</button></div>
        <label className="conversation-search"><span className="sr-only">Search conversations</span><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search chats and agents"/></label>
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


        </div>

        <div className="conversations-scroll">
          {filteredConvs.map(conv => (
            <button type="button"
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
            </button>
          ))}

          {filteredConvs.length === 0 && (
            <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-muted)', fontSize: 13 }}>
              No matching chats. Start a conversation with an agent below.
            </div>
          )}
          <div className="agent-contacts"><h3>Agents</h3>{sessions.filter(s=>!['stopped','completed','disconnected'].includes(s.state)&&s.name.toLowerCase().includes(search.toLowerCase())).map(s=><button type="button" className="agent-contact" key={s.id} onClick={()=>onStartDirect(s.id).catch(e=>setContactError(e.message))}><span className="contact-avatar">{s.name.slice(0,2).toUpperCase()}</span><span><strong>{s.name}</strong><small>{s.machine_name} · {s.interactive?'Terminal mode':s.state}</small></span></button>)}{!sessions.some(s=>!['stopped','completed','disconnected'].includes(s.state))&&<p>Start an agent from Machines to begin chatting.</p>}{contactError&&<p role="alert">{contactError}</p>}</div>
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
                {directMember?.provider.endsWith('_chat')&&<button className="icon-button" disabled={controlBusy} onClick={()=>runControl(async()=>{const r=await apiRequest<{id:string}>(`/api/v1/conversations/${activeConv.id}/native-terminal`,{method:'POST'});setTerminalId(r.data.id)})}>Native CLI</button>}
                {terminalId&&<button className="icon-button" onClick={()=>setTerminalId(null)}>Back to chat</button>}
                {interactiveMembers.length > 0 && <select aria-label="Open agent terminal" value={terminalId || ''} onChange={e => setTerminalId(e.target.value || null)}><option value="">Conversation</option>{interactiveMembers.map(s=><option key={s.id} value={s.id}>{s.name} terminal</option>)}</select>}
                {!activeConv.is_active && <button className="icon-button" disabled={controlBusy} onClick={() => runControl(() => onResumeConversation(activeConv.id))}>Resume</button>}
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
                <details className="chat-more"><summary>Options</summary><div><button
                  className="icon-button"
                  onClick={() => onDeleteConversation(activeConv.id)}
                  title="Delete conversation"
                >
                  <FiTrash2 size={14} /> Delete conversation
                </button></div></details>
              </div>
            </div>

            {activeConv.is_group && <details className="coworker-controls" open={settingsOpen} onToggle={e=>setSettingsOpen(e.currentTarget.open)}><summary>Channel settings · {modeValue.charAt(0).toUpperCase()+modeValue.slice(1)}</summary>
              <div className="coworker-mode">
                <label>Mode <select aria-label="Conversation mode" value={modeValue} disabled={controlBusy} onChange={e=>changeMode(e.target.value)}><option value="casual">Casual</option><option value="goal">Goal</option><option value="evidence">Evidence</option></select></label>
                <label><input type="checkbox" checked={activeConv.collaboration?.enabled ?? false} disabled={controlBusy}
                  onChange={e => runControl(() => onCollaborationChange(activeConv.id, e.target.checked, activeConv.collaboration?.max_turns ?? 0))}/>
                  Autonomous channel</label>
                <label>Discussion <select aria-label="Channel discussion limit" value={activeConv.collaboration?.max_turns ?? 0} disabled={controlBusy}
                  onChange={e => runControl(() => onCollaborationChange(activeConv.id, activeConv.collaboration?.enabled ?? true, Number(e.target.value)))}>
                  {[0, 4, 8, 16, 32].map(n => <option key={n} value={n}>{n===0?'Continuous':`${n} replies`}</option>)}
                  {activeConv.collaboration && ![0, 4, 8, 16, 32].includes(activeConv.collaboration.max_turns) && <option value={activeConv.collaboration.max_turns}>{activeConv.collaboration.max_turns}</option>}
                </select></label>
              </div>
              {modeValue==='evidence'&&<p>Verify claims independently, cite sources, and challenge unsupported agreement. Sources remain open to review.</p>}
              {modeValue==='goal'&&<p>Work toward your stated outcome, review results, and surface decisions that need you.</p>}
              {activeConv.collaboration?.human_goal && <details className="coworker-brief"><summary>Channel context</summary><p>{activeConv.collaboration.human_goal}</p></details>}
              <p>{activeConv.collaboration?.enabled
                ? activeConv.collaboration.max_turns===0 ? 'Open conversation · members may reply in parallel. Mention an agent to direct a message.' : `Open conversation · ${activeConv.collaboration.remaining_turns} replies remain in this session.`
                : 'Autonomous discussion is off. Your messages still reach the selected agents.'}</p>
              {activeConv.collaboration?.waiting_for_human && <div className="coworker-human-request" role="status">
                <details><summary>Your input is needed</summary><p>{activeConv.collaboration.human_request}</p></details>
                <span>Reply below to continue the discussion.</span>
              </div>}
            </details>}
            {controlError && <p className="coworker-control-error" role="alert">{controlError}</p>}

            {directMember?.provider.endsWith('_chat') && <GroupAwareness key={`group-awareness:${activeConv.id}`} agentId={directMember.id}/>}
            <ChatActivity key={`activity:${activeConv.id}`} id={activeConv.id} members={sessions.filter(s=>activeConv.participant_ids.includes(s.id))}/>
            {/* Messages Feed */}
            {terminalId ? <div className="chat-terminal"><TerminalPanel key={terminalId} id={terminalId}/></div> : <div className="chat-messages-scroll" ref={containerRef} onScroll={e=>{const el=e.currentTarget;pinnedToBottom.current=el.scrollHeight-el.scrollTop-el.clientHeight<120}}>
              {interactiveMembers.length>0 && <div className="native-chat-notice">{interactiveMembers.map(s=>s.name).join(', ')} uses terminal mode. Open its terminal above, or start a chat adapter from Machines for message replies.</div>}
              {visibleMessages.map(msg => (
                <div key={msg.id} className={`message-row ${msg.sender_type}`}>
                  {msg.sender_type !== 'human' && msg.sender_type !== 'system' && (
                    <span className="message-sender">{msg.sender_name}</span>
                  )}

                  <div className="message-bubble">
                    {msg.coworker && msg.coworker.kind!=='discuss' && <div className="coworker-message-meta">
                      <strong>{({discuss: 'Discussion', plan: 'Plan', handoff: 'Handoff', human_input: 'Needs your input', complete: 'Outcome'})[msg.coworker.kind]}</strong>
                      {msg.coworker.to.length > 0 && <span>To {msg.coworker.to.map(id => id === 'all' ? 'all coworkers' : sessions.find(s => s.id === id)?.name ?? id).join(', ')}</span>}
                    </div>}
                    {msg.coworker?.flair&&<span className="message-flair">{msg.coworker.flair}</span>}
                    {msg.sender_type==='system'&&msg.sender_name.includes('CLI')?<details className="cli-result"><summary>CLI result</summary><pre>{msg.text}</pre></details>:<MessageText text={msg.text}/>}
                    {msg.attachments?.map(file=><AttachmentCard key={file.id} file={file}/>)}

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

                  <span className="message-time">{formatTime(msg.timestamp)} {msg.sender_type==='human'&&<span aria-label={msg.status} className="message-delivery">{msg.status==='failed'?'Failed':msg.status==='sent'?'Sent':'Delivered'}</span>}</span>
                </div>
              ))}
              {workingMembers.length>0&&<div className="typing-state" role="status">{workingMembers.map(s=>s.name).join(', ')} {workingMembers.length===1?'is':'are'} working…</div>}
              <div ref={messagesEndRef} />
            </div>}

            {/* Quick Prompt Bar */}
            <div className="quick-prompts-row">
              <button className="quick-action-btn" onClick={()=>handlePromptClick('Let’s chat. What movie would you recommend, and why?')}>Talk about movies</button>
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

            {!terminalId&&<details className="attachment-toolbar"><summary><FiPaperclip size={14}/>Attach files{pendingFiles.length?` · ${pendingFiles.length}`:''}</summary><AttachmentPicker key={activeConv.id} disabled={!activeConv.is_active||isSubmitting||false} onAttach={file=>{setPendingFiles(previous=>[...previous,file]);setAttachmentError('')}}/>{pendingFiles.map(file=><div className="pending-attachment" key={file.id}><span>{file.name}</span><button type="button" aria-label={`Remove attachment ${file.name}`} onClick={()=>setPendingFiles(files=>files.filter(f=>f.id!==file.id))}>Remove</button></div>)}{attachmentError&&<p role="alert">{attachmentError}</p>}</details>}
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

              <ChatComposer key={activeConv.id} id={activeConv.id} members={sessions.filter(s=>activeConv.participant_ids.includes(s.id))} group={activeConv.is_group} value={inputText} onChange={setInputText} onSend={()=>handleSend()} disabled={!activeConv.is_active||isSubmitting} placeholder={!activeConv.is_active?'Conversation is paused':activeConv.is_group?'Message this channel…':'Message or /help…'}/>

              <button
                type="submit"
                className="icon-button primary"
                disabled={!activeConv.is_active || (!inputText.trim()&&!pendingFiles.length) || isSubmitting}
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
            <div className="messenger-empty"><h2>Your agents, one conversation away</h2><p>Choose an agent to start a chat, or create a group to work with several agents together.</p><button className="icon-button primary" onClick={onOpenNewGroup}>Create a group</button></div>
          </div>
        )}
      </section>
    </div>
  );
};


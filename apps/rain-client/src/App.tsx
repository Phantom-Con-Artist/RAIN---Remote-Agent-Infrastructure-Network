import { HomePage } from './components/HomePage';
import { GuidePage } from './components/GuidePage';
import { FilesWorkspace } from './components/FilesWorkspace';
import { AgentWorkbench } from './components/AgentWorkbench';
import React, { useState, useEffect, useCallback, useRef } from 'react';
import { TabType, ThemeMode, Conversation, ChatMessage, AgentSession, MachineOverview, InstalledApp, NetworkMeshStatus, HubTopologyStatus, ChatAttachment } from './types';
import { apiRequest, getHostUrl } from './api';
import { Header } from './components/Header';
import { ChatsView } from './components/ChatsView';
import { MachinesView } from './components/MachinesView';
import { MeshView } from './components/MeshView';
import { NetworkWorkspace } from './components/NetworkWorkspace';
import { PairingModal } from './components/PairingModal';
import { NewGroupModal } from './components/NewGroupModal';
export const App: React.FC = () => {
    // Theme state
    const [theme, setTheme] = useState<ThemeMode>(() => {
        return (localStorage.getItem('rain_theme') as ThemeMode) || 'dark';
    });
    useEffect(() => {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('rain_theme', theme);
    }, [theme]);
    const toggleTheme = () => {
        setTheme(prev => (prev === 'dark' ? 'light' : 'dark'));
    };
    // Navigation tab
    const [fileDirty, setFileDirty] = useState(false);
    const changeMachine = (machine: string) => { if (!fileDirty || window.confirm('Discard unsaved file changes before switching machines?'))
        setSelectedMachine(machine); };
    const [selectedMachine, setSelectedMachine] = useState('');
    const routeTab = (): TabType => { const route = location.hash.slice(1).split('/')[0]; return ({ network: 'mesh', chats: 'chats', machines: 'machines', files: 'files', guide: 'guide', home: 'home' } as Record<string, TabType>)[route] || 'home'; };
    const [currentTab, setCurrentTabState] = useState<TabType>(routeTab);
    const setCurrentTab = (tab: TabType) => { const hash = tab === 'mesh' ? 'network/cloud' : tab; if (location.hash !== `#${hash}`)
        location.hash = hash; setCurrentTabState(tab); };
    useEffect(() => { const listener = () => setCurrentTabState(routeTab()); window.addEventListener('hashchange', listener); return () => window.removeEventListener('hashchange', listener); }, []);
    // Connection & Telemetry
    const [connectionStatus, setConnectionStatus] = useState<'online' | 'offline' | 'connecting'>('connecting');
    const [latencyMs, setLatencyMs] = useState<number>(0);
    const [connectionError, setConnectionError] = useState('');
    const [meshStatus, setMeshStatus] = useState<NetworkMeshStatus | null>(null);
    // Chats state
    const [conversations, setConversations] = useState<Conversation[]>([]);
    const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [sessions, setSessions] = useState<AgentSession[]>([]);
    const [isMobileChatOpen, setIsMobileChatOpen] = useState(false);
    const activeConversationRef = useRef(activeConversationId);
    activeConversationRef.current = activeConversationId;
    const messagesRequest = useRef(0);
    // Machines state
    const [machines, setMachines] = useState<MachineOverview[]>([]);
    const [installedApps, setInstalledApps] = useState<InstalledApp[]>([]);
    // Topology state
    const [topology, setTopology] = useState<HubTopologyStatus | null>(null);
    // Modals
    const [showPairingModal, setShowPairingModal] = useState(false);
    const [showNewGroupModal, setShowNewGroupModal] = useState(false);
    // Global Toast
    const [toastMessage, setToastMessage] = useState<string | null>(null);
    const toastTimeoutRef = useRef<any>(null);
    const showToast = useCallback((msg: string) => {
        setToastMessage(msg);
        if (toastTimeoutRef.current)
            clearTimeout(toastTimeoutRef.current);
        toastTimeoutRef.current = setTimeout(() => {
            setToastMessage(null);
        }, 3000);
    }, []);
    // 1. Data Loaders
    const loadConversations = useCallback(async () => {
        try {
            const res = await apiRequest<Conversation[]>('/api/v1/conversations');
            setConversations(res.data);
            setLatencyMs(res.latencyMs);
            setConnectionStatus('online');
            setConnectionError('');
            setActiveConversationId(prev => {
                if (res.data.length === 0)
                    return null;
                if (!prev || !res.data.some(c => c.id === prev)) {
                    return res.data[0].id;
                }
                return prev;
            });
        }
        catch (error) {
            setConnectionStatus('offline');
            setConnectionError(error instanceof Error ? error.message : 'Hub unavailable');
        }
    }, []);
    const loadMessages = useCallback(async (convId: string) => {
        const request = ++messagesRequest.current;
        try {
            const res = await apiRequest<ChatMessage[]>(`/api/v1/conversations/${convId}/messages`);
            if (request !== messagesRequest.current || activeConversationRef.current !== convId)
                return;
            setMessages(res.data);
            setLatencyMs(res.latencyMs);
        }
        catch (e) {
            console.error('Failed to load messages', e);
        }
    }, []);
    const loadSessions = useCallback(async () => {
        try {
            const res = await apiRequest<AgentSession[]>('/api/v1/sessions');
            setSessions(res.data);
        }
        catch { }
    }, []);
    const loadMachines = useCallback(async () => {
        try {
            const res = await apiRequest<MachineOverview[]>('/api/v1/machines/overview');
            setMachines(res.data);
            setLatencyMs(res.latencyMs);
        }
        catch { }
    }, []);
    const loadApps = useCallback(async () => {
        try {
            const res = await apiRequest<InstalledApp[]>('/api/v1/apps/installed');
            setInstalledApps(res.data);
        }
        catch { }
    }, []);
    const loadTopology = useCallback(async () => {
        try {
            const res = await apiRequest<HubTopologyStatus>('/api/v1/hub/topology');
            setTopology(res.data);
        }
        catch { }
    }, []);
    const loadMeshStatus = useCallback(async () => {
        try {
            const res = await apiRequest<NetworkMeshStatus>('/api/v1/network/status');
            setMeshStatus(res.data);
        }
        catch { }
    }, []);
    // 2. Initial Setup
    useEffect(() => {
        loadConversations();
        loadSessions();
        loadMachines();
        loadApps();
        loadTopology();
        loadMeshStatus();
    }, [loadConversations, loadSessions, loadMachines, loadApps, loadTopology, loadMeshStatus]);
    // Load messages whenever active conversation changes
    useEffect(() => {
        if (activeConversationId) {
            loadMessages(activeConversationId);
        }
        else {
            setMessages([]);
        }
    }, [activeConversationId, loadMessages]);
    // 3. Intelligent Background Polling (3 seconds)
    useEffect(() => {
        const timer = setInterval(() => {
            loadSessions();
            loadConversations();
            if (currentTab === 'chats' && activeConversationId) {
                loadMessages(activeConversationId);
            }
            if (currentTab === 'machines') {
                loadMachines();
            }
        }, 3000);
        return () => clearInterval(timer);
    }, [currentTab, activeConversationId, loadSessions, loadMessages, loadMachines]);
    // 4. Action Handlers
    const handleSelectConversation = (id: string) => {
        messagesRequest.current++;
        setMessages([]);
        setActiveConversationId(id);
        setIsMobileChatOpen(true);
    };
    const handleOpenDirect = async (id: string) => {
        const res = await apiRequest<Conversation>('/api/v1/conversations/direct', { method: 'POST', body: JSON.stringify({ agent_id: id }) });
        await loadConversations();
        messagesRequest.current++;
        setMessages([]);
        setActiveConversationId(res.data.id);
        setCurrentTab('chats');
        setIsMobileChatOpen(true);
    };
    const handleSendMessage = async (text: string, targetAgentId: string | null, attachments: ChatAttachment[]) => {
        if (!activeConversationId)
            return;
        try {
            await apiRequest(`/api/v1/conversations/${activeConversationId}/messages`, {
                method: 'POST',
                body: JSON.stringify({
                    text,
                    target_session_id: targetAgentId,
                    attachments,
                }),
            });
            await loadMessages(activeConversationId);
            await loadConversations();
        }
        catch (e: any) {
            showToast(`Send failed: ${e.message}`);
            throw e;
        }
    };
    const handleCreateGroup = async (title: string, participantIds: string[]) => {
        try {
            const res = await apiRequest<Conversation>('/api/v1/conversations/create', {
                method: 'POST',
                body: JSON.stringify({
                    title,
                    is_group: true,
                    participant_ids: participantIds,
                }),
            });
            await loadConversations();
            setActiveConversationId(res.data.id);
            setIsMobileChatOpen(true);
            showToast(`Group "${title}" created`);
        }
        catch (e: any) {
            showToast(`Failed to create group: ${e.message}`);
            throw e;
        }
    };
    const handleStopConversation = async (id: string) => {
        try {
            await apiRequest(`/api/v1/conversations/${id}/stop`, { method: 'POST' });
            showToast('Conversation session paused');
            await loadConversations();
            await loadMessages(id);
        }
        catch (e: any) {
            showToast(`Failed: ${e.message}`);
        }
    };
    const handleResumeConversation = async (id: string) => {
        await apiRequest(`/api/v1/conversations/${id}/resume`, { method: 'POST' });
        await loadConversations();
        await loadMessages(id);
    };
    const handleCollaboration = async (id: string, enabled: boolean, maxTurns: number) => {
        await apiRequest(`/api/v1/conversations/${id}/collaboration`, {
            method: 'PATCH', body: JSON.stringify({ enabled, max_turns: maxTurns }),
        });
        await loadConversations();
        await loadMessages(id);
    };
    const handleDeleteConversation = async (id: string) => {
        const group = conversations.find(c => c.id === id)?.is_group;
        if (!window.confirm(group ? 'Delete this group? Members will retain its shared history, but group discussion will end. Agent sessions keep running.' : 'Delete this direct conversation? Agent sessions keep running.'))
            return;
        try {
            await apiRequest(`/api/v1/conversations/${id}`, { method: 'DELETE' });
            showToast('Conversation deleted');
            setActiveConversationId(null);
            setIsMobileChatOpen(false);
            await loadConversations();
        }
        catch (e: any) {
            showToast(`Failed: ${e.message}`);
        }
    };
    const handleApproveJob = async (jobId: string, approved: boolean) => {
        try {
            await apiRequest(`/api/v1/jobs/${jobId}/approve`, {
                method: 'POST',
                body: JSON.stringify({ approved, sudo_password: null }),
            });
            showToast(approved ? 'Task authorized' : 'Task rejected');
            if (activeConversationId) {
                await loadMessages(activeConversationId);
            }
        }
        catch (e: any) {
            showToast(`Approval failed: ${e.message}`);
        }
    };
    const handleTriggerBatteryEvacuate = async () => {
        try {
            showToast('Triggering autonomous repository evacuation...');
            await apiRequest('/api/v1/power/failover', { method: 'POST' });
            if (activeConversationId) {
                await loadMessages(activeConversationId);
            }
            showToast('Work safely checkpointed and pushed to GitHub');
        }
        catch (e: any) {
            showToast(`Evacuation failed: ${e.message}`);
        }
    };
    const handleLaunchApp = async (appId: string) => {
        try {
            const res = await apiRequest<string>('/api/v1/apps/launch', {
                method: 'POST',
                body: JSON.stringify({ app_id: appId }),
            });
            showToast(res.data);
        }
        catch (e: any) {
            showToast(`Launch failed: ${e.message}`);
        }
    };
    const handleExecuteTerminal = async (cmd: string, isSudo: boolean, password?: string) => {
        const res = await apiRequest<any>('/api/v1/terminal/execute', {
            method: 'POST',
            body: JSON.stringify({
                command: cmd,
                is_sudo: isSudo,
                sudo_password: password || null,
                cwd: '.',
            }),
        });
        return res.data;
    };
    const handleRevokeDevice = async (deviceId: string) => {
        try {
            await apiRequest(`/api/v1/hub/devices/${deviceId}/revoke`, { method: 'POST' });
            showToast(`Device ${deviceId} revoked`);
            await loadTopology();
        }
        catch (e: any) {
            showToast(`Failed: ${e.message}`);
        }
    };
    const handleOpenPairing = () => setShowPairingModal(true);
    return (<div className="app-container flex h-dvh min-h-0 w-full flex-col overflow-hidden text-ink">
      {/* 1. Header Bar */}
      <Header currentTab={currentTab} onNavigate={setCurrentTab} title={{ home: 'Home', guide: 'Guide', chats: 'Conversations', machines: 'Machines & sessions', files: 'Files', mesh: 'Network & dataflow' }[currentTab]} theme={theme} onToggleTheme={toggleTheme} connectionStatus={connectionStatus} latencyMs={latencyMs} meshStatus={meshStatus} onOpenPairing={handleOpenPairing}/>

        {connectionStatus === 'offline' && <aside className="hub-connection-notice fixed left-1/2 top-20 z-40 w-[calc(100%-2rem)] max-w-xl -translate-x-1/2 rounded-xl border border-status-warning/40 bg-canvas-elevated/95 p-4 text-ink backdrop-blur-xl [&_p]:my-2 [&_p]:text-sm" role="status"><strong>Hub unavailable</strong><p>{connectionError} Displayed information may be out of date. This client cannot confirm the server’s current activity.</p><button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => { setConnectionStatus('connecting'); loadConversations(); }}>Retry connection</button></aside>}
      {/* 2. Main Content Screens */}
      <main className={`app-main relative flex min-h-0 flex-1 overflow-hidden [&.page-home]:overflow-y-auto [&.page-guide]:overflow-y-auto page-${currentTab}`} id="main-content" tabIndex={-1}>
        {currentTab === 'home' && <HomePage sessions={sessions} conversations={conversations} onNavigate={setCurrentTab} onOpenConversation={id => { handleSelectConversation(id); setCurrentTab('chats'); }} connectionStatus={connectionStatus}/>}
        {currentTab === 'guide' && <GuidePage onNavigate={setCurrentTab}/>}

        {currentTab === 'chats' && (<ChatsView conversations={conversations} activeConversationId={activeConversationId} onSelectConversation={handleSelectConversation} messages={messages} sessions={sessions} onSendMessage={handleSendMessage} onStartDirect={handleOpenDirect} onOpenNewGroup={() => setShowNewGroupModal(true)} onStopConversation={handleStopConversation} onResumeConversation={handleResumeConversation} onCollaborationChange={handleCollaboration} onDeleteConversation={handleDeleteConversation} onApproveJob={handleApproveJob} onTriggerBatteryEvacuate={handleTriggerBatteryEvacuate} isMobileChatOpen={isMobileChatOpen} onCloseMobileChat={() => setIsMobileChatOpen(false)}/>)}

        {currentTab === 'machines' && <div className="machines-workspace mx-auto h-full w-full max-w-7xl overflow-y-auto px-4 py-6 md:px-6"><AgentWorkbench machine={selectedMachine} onMachineChange={changeMachine} sessions={sessions} onRefresh={loadSessions} onOpenChat={handleOpenDirect}/>
          <details className="host-utilities mt-6 rounded-xl border border-border-subtle bg-canvas-surface/60 p-4 [&>summary]:font-mono [&>summary]:text-xs [&>summary]:uppercase"><summary>Hub hardware & utilities</summary><MachinesView machines={machines} installedApps={installedApps} onLaunchApp={handleLaunchApp} onExecuteTerminal={handleExecuteTerminal} onTriggerBatteryEvacuate={handleTriggerBatteryEvacuate}/></details>
        </div>}

        <div hidden={currentTab !== 'files'} className="file-view-wrapper min-h-0 min-w-0 flex-1"><FilesWorkspace machine={selectedMachine} onMachineChange={changeMachine} onDirtyChange={setFileDirty}/></div>

        {currentTab === 'mesh' && (<NetworkWorkspace onOpenConversation={id => { if (conversations.some(c => c.id === id)) {
            handleSelectConversation(id);
            setCurrentTab('chats');
        }
        else {
            showToast('This conversation is archived. Its history is available in member direct chats.');
        } }} connections={<MeshView topology={topology} meshStatus={meshStatus} latencyMs={latencyMs} onOpenPairing={handleOpenPairing} onRevokeDevice={handleRevokeDevice} onRefreshTopology={loadTopology} onHostUrlChanged={() => {
                    // Reload clears every hub-specific view, selection and pending request.
                    if (getHostUrl() !== window.location.origin)
                        window.location.assign(getHostUrl() + '/#network/connections');
                    else
                        window.location.reload();
                }}/>}/>)}
      </main>

      {/* 4. Global Modals */}
      <PairingModal isOpen={showPairingModal} onClose={() => setShowPairingModal(false)}/>

      <NewGroupModal isOpen={showNewGroupModal} onClose={() => setShowNewGroupModal(false)} sessions={sessions} onCreateGroup={handleCreateGroup}/>

      {/* 5. Toast Banner */}
      {toastMessage && <div className="toast-container fixed bottom-6 left-1/2 z-50 w-max max-w-[calc(100%-2rem)] -translate-x-1/2 rounded-xl border border-border-glow bg-canvas-elevated/95 px-5 py-3 text-sm text-ink backdrop-blur-xl">{toastMessage}</div>}
    </div>);
};

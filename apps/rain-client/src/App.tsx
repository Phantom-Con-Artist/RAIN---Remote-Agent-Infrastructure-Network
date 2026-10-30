import {HomePage} from './components/HomePage';
import {GuidePage} from './components/GuidePage';
import { FilesWorkspace } from './components/FilesWorkspace';
import { AgentWorkbench } from './components/AgentWorkbench';
import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  TabType,
  ThemeMode,
  Conversation,
  ChatMessage,
  AgentSession,
  MachineOverview,
  InstalledApp,
  NetworkMeshStatus,
  HubTopologyStatus,
  PairingTicket,
  ChatAttachment
} from './types';
import { apiRequest, setAuthToken } from './api';
import { Header } from './components/Header';
import { ChatsView } from './components/ChatsView';
import { MachinesView } from './components/MachinesView';
import { MeshView } from './components/MeshView';
import {NetworkWorkspace} from './components/NetworkWorkspace';
import { PairingModal } from './components/PairingModal';
import { NewGroupModal } from './components/NewGroupModal';

export const App: React.FC = () => {
  // Theme state
  const [theme, setTheme] = useState<ThemeMode>(() => {
    return (localStorage.getItem('rain_theme') as ThemeMode) || 'light';
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
  const changeMachine = (machine:string) => {if(!fileDirty||window.confirm('Discard unsaved file changes before switching machines?'))setSelectedMachine(machine);};
  const [selectedMachine, setSelectedMachine] = useState('');
  const routeTab=():TabType=>{const route=location.hash.slice(1).split('/')[0];return ({network:'mesh',chats:'chats',machines:'machines',files:'files',guide:'guide',home:'home'} as Record<string,TabType>)[route]||'home'};
  const [currentTab, setCurrentTabState] = useState<TabType>(routeTab);
  const setCurrentTab=(tab:TabType)=>{const hash=tab==='mesh'?'network/topology':tab;if(location.hash!==`#${hash}`)location.hash=hash;setCurrentTabState(tab)};
  useEffect(()=>{const listener=()=>setCurrentTabState(routeTab());window.addEventListener('hashchange',listener);return()=>window.removeEventListener('hashchange',listener)},[]);

  // Connection & Telemetry
  const [connectionStatus, setConnectionStatus] = useState<'online' | 'offline' | 'connecting'>('connecting');
  const [latencyMs, setLatencyMs] = useState<number>(5);
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
  const [pairingTicket, setPairingTicket] = useState<PairingTicket | null>(null);

  // Modals
  const [showPairingModal, setShowPairingModal] = useState(false);
  const [showNewGroupModal, setShowNewGroupModal] = useState(false);

  // Global Toast
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const toastTimeoutRef = useRef<any>(null);

  const showToast = useCallback((msg: string) => {
    setToastMessage(msg);
    if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current);
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

      setActiveConversationId(prev => {
        if (res.data.length === 0) return null;
        if (!prev || !res.data.some(c => c.id === prev)) {
          return res.data[0].id;
        }
        return prev;
      });
    } catch {
      setConnectionStatus('offline');
    }
  }, []);

  const loadMessages = useCallback(async (convId: string) => {
    const request = ++messagesRequest.current;
    try {
      const res = await apiRequest<ChatMessage[]>(`/api/v1/conversations/${convId}/messages`);
      if (request !== messagesRequest.current || activeConversationRef.current !== convId) return;
      setMessages(res.data);
      setLatencyMs(res.latencyMs);
    } catch (e) {
      console.error('Failed to load messages', e);
    }
  }, []);

  const loadSessions = useCallback(async () => {
    try {
      const res = await apiRequest<AgentSession[]>('/api/v1/sessions');
      setSessions(res.data);
    } catch {}
  }, []);

  const loadMachines = useCallback(async () => {
    try {
      const res = await apiRequest<MachineOverview[]>('/api/v1/machines/overview');
      setMachines(res.data);
      setLatencyMs(res.latencyMs);
    } catch {}
  }, []);

  const loadApps = useCallback(async () => {
    try {
      const res = await apiRequest<InstalledApp[]>('/api/v1/apps/installed');
      setInstalledApps(res.data);
    } catch {}
  }, []);

  const loadTopology = useCallback(async () => {
    try {
      const res = await apiRequest<HubTopologyStatus>('/api/v1/hub/topology');
      setTopology(res.data);
    } catch {}
  }, []);

  const loadMeshStatus = useCallback(async () => {
    try {
      const res = await apiRequest<NetworkMeshStatus>('/api/v1/network/status');
      setMeshStatus(res.data);
    } catch {}
  }, []);

  const requestPairingTicket = useCallback(async () => {
    try {
      const res = await apiRequest<PairingTicket>('/api/v1/auth/pair/ticket', { method: 'POST' });
      setPairingTicket(res.data);
    } catch {}
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
    } else {
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
    const res = await apiRequest<Conversation>('/api/v1/conversations/direct', {method:'POST', body:JSON.stringify({agent_id:id})});
    await loadConversations(); messagesRequest.current++; setMessages([]); setActiveConversationId(res.data.id); setCurrentTab('chats'); setIsMobileChatOpen(true);
  };

  const handleSendMessage = async (text: string, targetAgentId: string | null, attachments: ChatAttachment[]) => {
    if (!activeConversationId) return;
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
    } catch (e: any) {
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
    } catch (e: any) {
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
    } catch (e: any) {
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
    if (!window.confirm(group ? 'Delete this group? Members will retain its shared history, but group discussion will end. Agent sessions keep running.' : 'Delete this direct conversation? Agent sessions keep running.')) return;
    try {
      await apiRequest(`/api/v1/conversations/${id}`, { method: 'DELETE' });
      showToast('Conversation deleted');
      setActiveConversationId(null);
      setIsMobileChatOpen(false);
      await loadConversations();
    } catch (e: any) {
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
    } catch (e: any) {
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
    } catch (e: any) {
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
    } catch (e: any) {
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
    } catch (e: any) {
      showToast(`Failed: ${e.message}`);
    }
  };

  const handleConfirmPin = async (pin: string) => {
    const res = await apiRequest<any>('/api/v1/auth/pair/confirm', {
      method: 'POST',
      body: JSON.stringify({
        pairing_code: pin,
        client_device_id: 'dev-moto-edge-60',
        client_device_name: 'Motorola edge 60 fusion',
        client_platform: 'android',
      }),
    });
    if (res.data.token) {
      setAuthToken(res.data.token);
      showToast('Device paired successfully');
      await loadTopology();
    }
  };

  const handleRequestOtp = async (email: string) => {
    await apiRequest('/api/v1/auth/otp/email/request', {
      method: 'POST',
      body: JSON.stringify({ email }),
    });
    showToast('Verification code sent');
  };

  const handleVerifyOtp = async (email: string, code: string) => {
    const res = await apiRequest<any>('/api/v1/auth/otp/email/verify', {
      method: 'POST',
      body: JSON.stringify({
        email,
        otp_code: code,
        client_device_id: 'dev-email-mobile',
        client_device_name: 'Motorola edge 60 fusion',
        client_platform: 'android',
      }),
    });
    if (res.data.token) {
      setAuthToken(res.data.token);
      showToast('Authenticated via OTP');
      await loadTopology();
    }
  };

  const handleOpenPairing = () => {
    setShowPairingModal(true);
    requestPairingTicket();
  };

  return (
    <div className="app-container">
      {/* 1. Header Bar */}
      <Header
        currentTab={currentTab} onNavigate={setCurrentTab}
        title={{home:'Home',guide:'Guide',chats: 'Conversations', machines: 'Machines & sessions', files: 'Files', mesh: 'Network & dataflow'}[currentTab]}
        theme={theme}
        onToggleTheme={toggleTheme}
        connectionStatus={connectionStatus}
        latencyMs={latencyMs}
        meshStatus={meshStatus}
        onOpenPairing={handleOpenPairing}
      />

      {/* 2. Main Content Screens */}
      <main className={`app-main page-${currentTab}`} id="main-content" tabIndex={-1}>
        {currentTab==='home'&&<HomePage sessions={sessions} conversations={conversations} onNavigate={setCurrentTab} onOpenConversation={id=>{handleSelectConversation(id);setCurrentTab('chats')}} connectionStatus={connectionStatus}/>}
        {currentTab==='guide'&&<GuidePage onNavigate={setCurrentTab}/>}

        {currentTab === 'chats' && (
          <ChatsView
            conversations={conversations}
            activeConversationId={activeConversationId}
            onSelectConversation={handleSelectConversation}
            messages={messages}
            sessions={sessions}
            onSendMessage={handleSendMessage}
            onStartDirect={handleOpenDirect}
            onOpenNewGroup={() => setShowNewGroupModal(true)}
            onStopConversation={handleStopConversation}
            onResumeConversation={handleResumeConversation}
            onCollaborationChange={handleCollaboration}
            onDeleteConversation={handleDeleteConversation}
            onApproveJob={handleApproveJob}
            onTriggerBatteryEvacuate={handleTriggerBatteryEvacuate}
            isMobileChatOpen={isMobileChatOpen}
            onCloseMobileChat={() => setIsMobileChatOpen(false)}
          />
        )}

        {currentTab === 'machines' && <div className="machines-workspace"><AgentWorkbench machine={selectedMachine} onMachineChange={changeMachine} sessions={sessions} onRefresh={loadSessions} onOpenChat={handleOpenDirect} />
          <details className="host-utilities"><summary>Hub hardware & utilities</summary><MachinesView
            machines={machines}
            installedApps={installedApps}
            onLaunchApp={handleLaunchApp}
            onExecuteTerminal={handleExecuteTerminal}
            onTriggerBatteryEvacuate={handleTriggerBatteryEvacuate}
          /></details>
        </div>}

        <div hidden={currentTab!=='files'} className="file-view-wrapper"><FilesWorkspace machine={selectedMachine} onMachineChange={changeMachine} onDirtyChange={setFileDirty}/></div>

        {currentTab === 'mesh' && (
          <NetworkWorkspace onOpenConversation={id=>{if(conversations.some(c=>c.id===id)){handleSelectConversation(id);setCurrentTab('chats');}else{showToast('This conversation is archived. Its history is available in member direct chats.');}}} connections={<MeshView
            topology={topology}
            meshStatus={meshStatus}
            latencyMs={latencyMs}
            onOpenPairing={handleOpenPairing}
            onRevokeDevice={handleRevokeDevice}
            onRefreshTopology={loadTopology}
            onHostUrlChanged={() => {
              loadConversations();
              loadTopology();
            }}
          />} />
        )}
      </main>

      {/* 4. Global Modals */}
      <PairingModal
        isOpen={showPairingModal}
        onClose={() => setShowPairingModal(false)}
        ticket={pairingTicket}
        onConfirmPin={handleConfirmPin}
        onRequestOtp={handleRequestOtp}
        onVerifyOtp={handleVerifyOtp}
      />

      <NewGroupModal
        isOpen={showNewGroupModal}
        onClose={() => setShowNewGroupModal(false)}
        sessions={sessions}
        onCreateGroup={handleCreateGroup}
      />

      {/* 5. Toast Banner */}
      {toastMessage && <div className="toast-container">{toastMessage}</div>}
    </div>
  );
};


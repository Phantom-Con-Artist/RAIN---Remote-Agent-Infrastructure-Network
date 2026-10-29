import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  FiMessageSquare,
  FiServer,
  FiFolder,
  FiShare2,
} from 'react-icons/fi';
import {
  TabType,
  ThemeMode,
  Conversation,
  ChatMessage,
  AgentSession,
  MachineOverview,
  InstalledApp,
  FileEntry,
  NetworkMeshStatus,
  HubTopologyStatus,
  PairingTicket,
  ChatAttachment
} from './types';
import { apiRequest, setAuthToken } from './api';
import { Header } from './components/Header';
import { ChatsView } from './components/ChatsView';
import { MachinesView } from './components/MachinesView';
import { FilesView } from './components/FilesView';
import { MeshView } from './components/MeshView';
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
  const [currentTab, setCurrentTab] = useState<TabType>('chats');

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

  // Machines state
  const [machines, setMachines] = useState<MachineOverview[]>([]);
  const [installedApps, setInstalledApps] = useState<InstalledApp[]>([]);

  // Files state
  const [currentPath, setCurrentPath] = useState<string>('~');
  const [files, setFiles] = useState<FileEntry[]>([]);

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
    try {
      const res = await apiRequest<ChatMessage[]>(`/api/v1/conversations/${convId}/messages`);
      setMessages(prev => {
        // Prevent unnecessary state update if identical
        if (
          prev.length === res.data.length &&
          (res.data.length === 0 || prev[prev.length - 1]?.id === res.data[res.data.length - 1]?.id)
        ) {
          return prev;
        }
        return res.data;
      });
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

  const loadDirectory = useCallback(async (pathStr: string) => {
    try {
      const res = await apiRequest<FileEntry[]>(`/api/v1/fs/list?path=${encodeURIComponent(pathStr)}`);
      setCurrentPath(pathStr);
      setFiles(res.data);
    } catch (e: any) {
      showToast(`Error opening directory: ${e.message}`);
    }
  }, [showToast]);

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
    loadDirectory('~');
    loadTopology();
    loadMeshStatus();
  }, [loadConversations, loadSessions, loadMachines, loadApps, loadDirectory, loadTopology, loadMeshStatus]);

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
    setActiveConversationId(id);
    setIsMobileChatOpen(true);
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

  const handleDeleteConversation = async (id: string) => {
    if (!window.confirm('Delete this conversation?')) return;
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
        cwd: currentPath === '~' ? '.' : currentPath,
      }),
    });
    return res.data;
  };

  const handleReadFile = async (pathStr: string, name: string) => {
    const res = await apiRequest<{ content: string; is_binary: boolean }>(
      `/api/v1/fs/read?path=${encodeURIComponent(pathStr)}`
    );
    return res.data;
  };

  const handleWriteFile = async (pathStr: string, content: string) => {
    try {
      await apiRequest('/api/v1/fs/write', {
        method: 'POST',
        body: JSON.stringify({ path: pathStr, content }),
      });
      showToast('File saved to machine');
    } catch (e: any) {
      showToast(`Save failed: ${e.message}`);
    }
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
        theme={theme}
        onToggleTheme={toggleTheme}
        connectionStatus={connectionStatus}
        latencyMs={latencyMs}
        meshStatus={meshStatus}
        onOpenPairing={handleOpenPairing}
      />

      {/* 2. Main Content Screens */}
      <main className="app-main">
        {currentTab === 'chats' && (
          <ChatsView
            conversations={conversations}
            activeConversationId={activeConversationId}
            onSelectConversation={handleSelectConversation}
            messages={messages}
            sessions={sessions}
            onSendMessage={handleSendMessage}
            onOpenNewGroup={() => setShowNewGroupModal(true)}
            onStopConversation={handleStopConversation}
            onDeleteConversation={handleDeleteConversation}
            onApproveJob={handleApproveJob}
            onTriggerBatteryEvacuate={handleTriggerBatteryEvacuate}
            isMobileChatOpen={isMobileChatOpen}
            onCloseMobileChat={() => setIsMobileChatOpen(false)}
          />
        )}

        {currentTab === 'machines' && (
          <MachinesView
            machines={machines}
            installedApps={installedApps}
            onLaunchApp={handleLaunchApp}
            onExecuteTerminal={handleExecuteTerminal}
            onTriggerBatteryEvacuate={handleTriggerBatteryEvacuate}
          />
        )}

        {currentTab === 'files' && (
          <FilesView
            currentPath={currentPath}
            files={files}
            onOpenDirectory={loadDirectory}
            onReadFile={handleReadFile}
            onWriteFile={handleWriteFile}
          />
        )}

        {currentTab === 'mesh' && (
          <MeshView
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
          />
        )}
      </main>

      {/* 3. Bottom Navigation Bar */}
      <nav className="bottom-nav">
        <button
          className={`nav-tab-button ${currentTab === 'chats' ? 'active' : ''}`}
          onClick={() => setCurrentTab('chats')}
        >
          <FiMessageSquare className="nav-tab-icon" />
          <span>Messages</span>
        </button>

        <button
          className={`nav-tab-button ${currentTab === 'machines' ? 'active' : ''}`}
          onClick={() => {
            setCurrentTab('machines');
            loadMachines();
          }}
        >
          <FiServer className="nav-tab-icon" />
          <span>Machines</span>
        </button>

        <button
          className={`nav-tab-button ${currentTab === 'files' ? 'active' : ''}`}
          onClick={() => {
            setCurrentTab('files');
            loadDirectory(currentPath);
          }}
        >
          <FiFolder className="nav-tab-icon" />
          <span>Files</span>
        </button>

        <button
          className={`nav-tab-button ${currentTab === 'mesh' ? 'active' : ''}`}
          onClick={() => {
            setCurrentTab('mesh');
            loadTopology();
            loadMeshStatus();
          }}
        >
          <FiShare2 className="nav-tab-icon" />
          <span>Network</span>
        </button>
      </nav>

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


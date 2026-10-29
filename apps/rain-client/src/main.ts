// RAIN — Remote Agent Infrastructure Network
// WhatsApp-grade AI Agent Messenger & Remote Machine Workbench

interface ChatAttachment {
  id: string;
  name: string;
  mime_type: string;
  size_bytes: number;
  preview_content?: string;
  data_base64?: string;
}

interface JobDelegation {
  id: string;
  source_agent_id: string;
  source_machine: string;
  target_agent_id: string;
  target_machine: string;
  task_instruction: string;
  required_resources: string[];
  status: 'proposed' | 'accepted' | 'transferring' | 'running' | 'requires_approval' | 'completed' | 'failed';
  progress_percent: number;
  shared_artifacts: ChatAttachment[];
  requires_sudo: boolean;
  sudo_command?: string;
  is_approved: boolean;
  result_summary?: string;
  created_at: string;
}

interface MachineResourceCatalog {
  machine_id: string;
  machine_name: string;
  platform: string;
  cpu_cores: number;
  ram_total_gb: number;
  ram_free_gb: number;
  gpu_model?: string;
  network_speed_mbps: number;
  tags: string[];
}

interface ChatMessage {
  id: string;
  conversation_id: string;
  sender_id: string;
  sender_name: string;
  sender_type: 'human' | 'agent' | 'system';
  text: string;
  status: 'sent' | 'delivered' | 'read' | 'failed';
  attachments: ChatAttachment[];
  job_delegation?: JobDelegation;
  timestamp: string;
}

interface Conversation {
  id: string;
  title: string;
  is_group: boolean;
  is_active: boolean;
  participant_ids: string[];
  last_message: string | null;
  last_message_time: string | null;
  unread_count: number;
}

interface AgentSession {
  id: string;
  name: string;
  provider: string;
  machine_name: string;
  state: 'online' | 'thinking' | 'working' | 'responding' | 'waiting' | 'completed' | 'stopped' | 'disconnected' | 'error';
  cwd: string;
  subagents: string[];
}

interface BatteryStatus {
  percent: number;
  is_charging: boolean;
  is_ac_connected: boolean;
  power_source: string;
  time_remaining_minutes?: number;
  is_critical: boolean;
}

interface HubSpokeNode {
  device_id: string;
  device_name: string;
  platform: string;
  is_online: boolean;
  enrolled_at: string;
  last_seen_at: string;
  ip_address?: string;
  role: string;
}

interface HubTopologyStatus {
  hub_name: string;
  hub_node_id: string;
  hub_listen_addr: string;
  is_hub_online: boolean;
  uptime_seconds: number;
  active_spokes_count: number;
  spokes: HubSpokeNode[];
}

interface PairingTicket {
  pairing_code: string;
  deep_link: string;
  qr_svg: string;
  host_name: string;
  host_listen_addr: string;
  expires_in_seconds: number;
}

interface MachineResourceCatalog {
  machine_id: string;
  machine_name: string;
  platform: string;
  cpu_cores: number;
  ram_total_gb: number;
  ram_free_gb: number;
  gpu_model?: string;
  network_speed_mbps: number;
  tags: string[];
  battery?: BatteryStatus;
}

interface MachineOverview {
  id: string;
  name: string;
  platform: 'linux' | 'macos' | 'windows' | 'android' | 'ios' | 'unknown';
  is_online: boolean;
  ip_address: string;
  cpu_usage: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
  active_agents_count: number;
  uptime_seconds: number;
  battery?: BatteryStatus;
}

interface InstalledApp {
  id: string;
  name: string;
  executable: string;
  icon_name: string;
  category: string;
}

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size_bytes: number;
  modified_time: string | null;
  is_editable: boolean;
}

interface NetworkMeshStatus {
  current_transport: 'direct_usb' | 'lan_mdns' | 'wireguard_p2p' | 'encrypted_relay';
  node_id: string;
  latency_ms: number;
  bytes_sent: number;
  bytes_received: number;
  is_e2ee: boolean;
  available_transports: string[];
}

type TabType = 'chats' | 'machines' | 'files' | 'mesh';

// Application State
const state = {
  currentTab: 'chats' as TabType,
  hostUrl: localStorage.getItem('rain_host_url') || 'http://localhost:7890',
  authToken: localStorage.getItem('rain_auth_token') || '',
  connectionStatus: 'connecting' as 'online' | 'offline' | 'connecting',
  latencyMs: 7,

  // Chats
  conversations: [] as Conversation[],
  activeConversationId: null as string | null,
  messages: [] as ChatMessage[],
  sessions: [] as AgentSession[],
  chatFilter: 'all' as 'all' | 'groups' | 'direct',
  targetSessionId: 'all',
  isMobileChatOpen: false,

  // Machines & Personal Cloud
  machines: [] as MachineOverview[],
  installedApps: [] as InstalledApp[],
  jobs: [] as JobDelegation[],
  resources: [] as MachineResourceCatalog[],
  terminalLogs: [] as Array<{
    cmd: string;
    stdout: string;
    stderr: string;
    exit_code: number;
    duration_ms: number;
    is_sudo: boolean;
  }>,
  sudoPassword: '',
  isSudoChecked: false,

  // Files
  currentPath: '~',
  fileEntries: [] as FileEntry[],
  editingFile: null as { path: string; name: string; content: string } | null,
  isEditorDirty: false,

  // Mesh & Transport
  meshStatus: null as NetworkMeshStatus | null,

  // Modals & UI
  showNewGroupModal: false,
  showSudoModal: false,
  pendingSudoCommand: '',
  showAttachmentMenu: false,
  pendingAttachment: null as ChatAttachment | null,
  toastMessage: null as string | null,

  // Topology & Pairing
  hubTopology: null as HubTopologyStatus | null,
  showPairingModal: false,
  pairingTab: 'host' as 'host' | 'pin' | 'email',
  pairingTicket: null as PairingTicket | null,
  pairingPinInput: '',
  pairingEmailInput: '',
  pairingEmailOtpInput: '',
  emailOtpSentMessage: null as string | null,
  isPairingLoading: false,
};

// Utilities
function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return '';
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

let toastTimer: any = null;
function showToast(msg: string) {
  state.toastMessage = msg;
  let banner = document.getElementById('rain-toast-banner');
  if (!banner) {
    banner = document.createElement('div');
    banner.id = 'rain-toast-banner';
    banner.className = 'toast-banner';
    document.body.appendChild(banner);
  }
  banner.textContent = msg;
  banner.style.display = 'block';

  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    if (banner) banner.style.display = 'none';
    state.toastMessage = null;
  }, 3000);
}

// Surgical UI Telemetry Updates (No layout reflow or page recreation)
function updateNetworkTelemetryUI() {
  const latencyBadge = document.querySelector('.mesh-latency');
  if (latencyBadge) {
    latencyBadge.textContent = state.connectionStatus === 'online' ? `${state.latencyMs}ms` : 'Offline';
  }
  const dot = document.querySelector('.status-indicator-dot');
  if (dot) {
    dot.className = `status-indicator-dot ${state.connectionStatus === 'online' ? 'dot-online' : 'dot-offline'}`;
  }
  const meshLatency = document.querySelector('.mesh-stat-value-latency');
  if (meshLatency) {
    meshLatency.textContent = `${state.latencyMs} ms`;
  }
}

// API Network Client
async function apiRequest<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const base = state.hostUrl.replace(/\/+$/, '');
  const url = `${base}${endpoint}`;
  const start = performance.now();

  const headers = new Headers(options.headers || {});
  headers.set('Content-Type', 'application/json');
  if (state.authToken) {
    headers.set('Authorization', `Bearer ${state.authToken}`);
  }

  try {
    const res = await fetch(url, { ...options, headers });
    state.latencyMs = Math.round(performance.now() - start);
    const json = await res.json();
    if (!json.success) throw new Error(json.error || 'Server error');
    state.connectionStatus = 'online';
    updateNetworkTelemetryUI();
    return json.data as T;
  } catch (err: any) {
    state.connectionStatus = 'offline';
    updateNetworkTelemetryUI();
    throw err;
  }
}

// Data Loaders
async function loadConversations() {
  try {
    const convs = await apiRequest<Conversation[]>('/api/v1/conversations');
    state.conversations = convs;
    if (convs.length === 0) {
      state.activeConversationId = null;
      state.messages = [];
    } else if (!state.activeConversationId || !convs.some(c => c.id === state.activeConversationId)) {
      state.activeConversationId = convs[0].id;
    }
  } catch (e) {
    console.error('Failed to load conversations', e);
  }
}

function updateConversationPreview(convId: string, msgs: ChatMessage[]) {
  const item = document.querySelector(`.conv-item[data-conv-id="${convId}"]`);
  if (!item || msgs.length === 0) return;
  const last = msgs[msgs.length - 1];
  const prev = item.querySelector('.conv-preview');
  if (prev) {
    prev.textContent = last.text ? (last.text.length > 50 ? last.text.slice(0, 50) + '...' : last.text) : (last.job_delegation ? 'Distributed cloud job' : 'Attachment');
  }
  const time = item.querySelector('.conv-time');
  if (time && last.timestamp) {
    time.textContent = formatTime(last.timestamp);
  }
}

async function loadMessages(convId: string, forceRender = false) {
  try {
    const msgs = await apiRequest<ChatMessage[]>(`/api/v1/conversations/${convId}/messages`);
    
    // Check if nothing changed to prevent unnecessary DOM mutations
    const isSame = !forceRender &&
      state.activeConversationId === convId &&
      state.messages.length === msgs.length &&
      (msgs.length === 0 || (
        state.messages.length > 0 &&
        msgs[msgs.length - 1].id === state.messages[state.messages.length - 1]?.id &&
        msgs[msgs.length - 1].status === state.messages[state.messages.length - 1]?.status &&
        msgs[msgs.length - 1].job_delegation?.status === state.messages[state.messages.length - 1]?.job_delegation?.status &&
        msgs[msgs.length - 1].job_delegation?.progress_percent === state.messages[state.messages.length - 1]?.job_delegation?.progress_percent
      ));

    if (isSame) {
      return;
    }

    state.messages = msgs;

    const container = document.getElementById('chat-messages-container');
    if (container && state.currentTab === 'chats' && state.activeConversationId === convId) {
      const wasNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 120;
      container.innerHTML = msgs.map(renderMessageBubble).join('');
      attachJobActionHandlers();
      if (wasNearBottom || forceRender) {
        scrollToBottom();
      }
    } else if (forceRender && document.getElementById('app')) {
      render();
    }

    updateConversationPreview(convId, msgs);
  } catch (e) {
    console.error('Failed to load messages', e);
  }
}

function openConversation(convId: string) {
  state.activeConversationId = convId;
  state.isMobileChatOpen = true;

  document.querySelectorAll('.conv-item').forEach(item => {
    item.classList.toggle('conv-active', item.getAttribute('data-conv-id') === convId);
  });

  const layout = document.querySelector('.chats-screen-layout');
  if (layout) {
    layout.classList.remove('mobile-show-roster');
    layout.classList.add('mobile-show-chat');
  }

  const conv = state.conversations.find(c => c.id === convId);
  const titleEl = document.querySelector('.chat-header-title');
  if (titleEl && conv) {
    titleEl.textContent = conv.title;
  }
  const tagEl = document.querySelector('.chat-header-title-row .badge-tag');
  if (tagEl && conv) {
    tagEl.textContent = conv.is_group ? 'Group' : 'Direct';
  }
  const subEl = document.querySelector('.chat-header-sub');
  if (subEl && conv) {
    subEl.textContent = conv.is_group ? `● ${conv.participant_ids.length - 1} agents connected` : '● 1-on-1 Direct Agent Session';
  }

  const container = document.getElementById('chat-messages-container');
  if (container) {
    loadMessages(convId, true);
  } else {
    render();
    loadMessages(convId, true);
  }
}

function closeMobileChat() {
  state.isMobileChatOpen = false;
  const layout = document.querySelector('.chats-screen-layout');
  if (layout) {
    layout.classList.remove('mobile-show-chat');
    layout.classList.add('mobile-show-roster');
  }
}

async function loadSessions() {
  try {
    const sess = await apiRequest<AgentSession[]>('/api/v1/sessions');
    state.sessions = sess;
  } catch (e) {
    console.error('Failed to load agent sessions', e);
  }
}

async function loadMachinesOverview(forceRender = false) {
  try {
    const data = await apiRequest<MachineOverview[]>('/api/v1/machines/overview');
    state.machines = data;
    if (state.currentTab === 'machines') {
      const grid = document.querySelector('.machines-grid');
      if (grid) {
        grid.innerHTML = renderMachinesCards();
        const evacBtn = document.getElementById('btn-evacuate-now-click');
        if (evacBtn) {
          evacBtn.addEventListener('click', () => {
            triggerBatteryFailover();
          });
        }
      } else if (forceRender) {
        render();
      }
    }
  } catch (e) {
    console.error('Failed to load machines', e);
  }
}

async function loadInstalledApps() {
  try {
    const data = await apiRequest<InstalledApp[]>('/api/v1/apps/installed');
    state.installedApps = data;
  } catch (e) {
    console.error('Failed to load apps', e);
  }
}

async function loadJobs() {
  try {
    const data = await apiRequest<JobDelegation[]>('/api/v1/jobs');
    state.jobs = data;
  } catch (e) {
    console.error('Failed to load jobs', e);
  }
}

async function loadMeshResources() {
  try {
    const data = await apiRequest<MachineResourceCatalog[]>('/api/v1/mesh/resources');
    state.resources = data;
  } catch (e) {
    console.error('Failed to load mesh resources', e);
  }
}

async function loadHubTopology() {
  try {
    const data = await apiRequest<HubTopologyStatus>('/api/v1/hub/topology');
    state.hubTopology = data;
    render();
  } catch (e) {
    console.error('Failed to load hub topology', e);
  }
}

async function requestPairingTicket() {
  try {
    state.isPairingLoading = true;
    render();
    const data = await apiRequest<PairingTicket>('/api/v1/auth/pair/ticket', { method: 'POST' });
    state.pairingTicket = data;
  } catch (e: any) {
    showToast('Failed to generate pairing ticket: ' + e.message);
  } finally {
    state.isPairingLoading = false;
    render();
  }
}

async function submitPinClaim() {
  if (!state.pairingPinInput.trim()) return;
  try {
    state.isPairingLoading = true;
    render();
    const res = await apiRequest<any>('/api/v1/auth/pair/confirm', {
      method: 'POST',
      body: JSON.stringify({
        client_device_id: 'device-mobile-claimer',
        pairing_code: state.pairingPinInput.trim(),
        client_device_name: 'Motorola edge 60 fusion',
        client_platform: 'android'
      })
    });
    if (res.token) {
      state.authToken = res.token;
      localStorage.setItem('rain_auth_token', res.token);
      showToast('🎉 Paired successfully with Hub!');
      state.showPairingModal = false;
      await loadHubTopology();
    }
  } catch (e: any) {
    showToast('PIN pairing failed: ' + e.message);
  } finally {
    state.isPairingLoading = false;
    render();
  }
}

async function requestEmailOtp() {
  if (!state.pairingEmailInput.trim()) return;
  try {
    state.isPairingLoading = true;
    render();
    const res = await apiRequest<string>('/api/v1/auth/otp/email/request', {
      method: 'POST',
      body: JSON.stringify({
        email: state.pairingEmailInput.trim(),
        device_name: 'Android Mobile'
      })
    });
    state.emailOtpSentMessage = res;
    showToast('OTP sent! Verification code generated.');
  } catch (e: any) {
    showToast('Failed to request OTP: ' + e.message);
  } finally {
    state.isPairingLoading = false;
    render();
  }
}

async function submitEmailOtpVerify() {
  if (!state.pairingEmailOtpInput.trim()) return;
  try {
    state.isPairingLoading = true;
    render();
    const res = await apiRequest<any>('/api/v1/auth/otp/email/verify', {
      method: 'POST',
      body: JSON.stringify({
        email: state.pairingEmailInput.trim(),
        otp_code: state.pairingEmailOtpInput.trim(),
        client_device_id: 'dev-email-mobile',
        client_device_name: 'Motorola edge 60 fusion',
        client_platform: 'android'
      })
    });
    if (res.token) {
      state.authToken = res.token;
      localStorage.setItem('rain_auth_token', res.token);
      showToast('🎉 Authenticated via Email OTP!');
      state.showPairingModal = false;
      await loadHubTopology();
    }
  } catch (e: any) {
    showToast('OTP verification failed: ' + e.message);
  } finally {
    state.isPairingLoading = false;
    render();
  }
}

async function revokeSpokeDevice(deviceId: string) {
  try {
    await apiRequest<boolean>(`/api/v1/hub/devices/${deviceId}/revoke`, { method: 'POST' });
    showToast(`Device ${deviceId} revoked.`);
    await loadHubTopology();
  } catch (e: any) {
    showToast('Failed to revoke device: ' + e.message);
  }
}

async function triggerBatteryFailover() {
  try {
    showToast('⚡ Triggering Autonomous 5% Battery Evacuation...');
    await apiRequest<string>('/api/v1/power/failover', { method: 'POST' });
    if (state.activeConversationId) {
      await loadMessages(state.activeConversationId);
    }
    showToast('✅ Autonomous battery evacuation executed to Git repository!');
  } catch (e: any) {
    showToast('Failed: ' + e.message);
  }
}

async function respondToApproval(jobId: string, approved: boolean) {
  try {
    await apiRequest(`/api/v1/jobs/${jobId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ approved, sudo_password: null }),
    });
    showToast(approved ? 'Task authorized & executed!' : 'Task rejected');
    await loadJobs();
    if (state.activeConversationId) {
      await loadMessages(state.activeConversationId, true);
    }
  } catch (e: any) {
    showToast(`Approval failed: ${e.message}`);
  }
}

function attachJobActionHandlers() {
  document.querySelectorAll('[data-approve-job]').forEach(btn => {
    const newBtn = btn.cloneNode(true) as HTMLElement;
    btn.parentNode?.replaceChild(newBtn, btn);
    newBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const jobId = newBtn.getAttribute('data-approve-job');
      if (jobId) respondToApproval(jobId, true);
    });
  });

  document.querySelectorAll('[data-reject-job]').forEach(btn => {
    const newBtn = btn.cloneNode(true) as HTMLElement;
    btn.parentNode?.replaceChild(newBtn, btn);
    newBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const jobId = newBtn.getAttribute('data-reject-job');
      if (jobId) respondToApproval(jobId, false);
    });
  });
}

async function loadDirectory(pathStr: string) {
  try {
    const data = await apiRequest<FileEntry[]>(`/api/v1/fs/list?path=${encodeURIComponent(pathStr)}`);
    state.currentPath = pathStr;
    state.fileEntries = data;
    render();
  } catch (e: any) {
    showToast(`Error opening directory: ${e.message}`);
  }
}

async function loadFileContent(filePath: string, fileName: string) {
  try {
    const data = await apiRequest<{ content: string; is_binary: boolean }>(`/api/v1/fs/read?path=${encodeURIComponent(filePath)}`);
    if (data.is_binary) {
      showToast('Cannot open binary file for editing');
      return;
    }
    state.editingFile = {
      path: filePath,
      name: fileName,
      content: data.content,
    };
    state.isEditorDirty = false;
    render();
  } catch (e: any) {
    showToast(`Error reading file: ${e.message}`);
  }
}

async function saveFileContent() {
  if (!state.editingFile) return;
  try {
    await apiRequest('/api/v1/fs/write', {
      method: 'POST',
      body: JSON.stringify({
        path: state.editingFile.path,
        content: state.editingFile.content,
      }),
    });
    state.isEditorDirty = false;
    showToast(`File saved to machine: ${state.editingFile.name}`);
    render();
  } catch (e: any) {
    showToast(`Failed to save: ${e.message}`);
  }
}

async function loadMeshStatus() {
  try {
    const data = await apiRequest<NetworkMeshStatus>('/api/v1/network/status');
    state.meshStatus = data;
  } catch (e) {
    console.error('Failed to load mesh status', e);
  }
}

// Actions
async function sendMessage(text: string) {
  if ((!text.trim() && !state.pendingAttachment) || !state.activeConversationId) return;

  const convId = state.activeConversationId;
  const target = state.targetSessionId === 'all' ? null : state.targetSessionId;
  const attachments = state.pendingAttachment ? [state.pendingAttachment] : [];

  state.pendingAttachment = null;
  state.showAttachmentMenu = false;

  const textInput = document.getElementById('chat-text-input') as HTMLInputElement;
  if (textInput) {
    textInput.value = '';
  }

  const attPreview = document.querySelector('.attachment-preview-banner');
  if (attPreview) {
    attPreview.remove();
  }

  try {
    await apiRequest(`/api/v1/conversations/${convId}/messages`, {
      method: 'POST',
      body: JSON.stringify({
        text,
        target_session_id: target,
        attachments,
      }),
    });

    await loadMessages(convId, true);
    await loadConversations();
  } catch (e: any) {
    showToast(`Send failed: ${e.message}`);
  }
}

async function createNewGroup(title: string, participantIds: string[]) {
  if (!title.trim()) {
    showToast('Please enter a group title');
    return;
  }
  try {
    const newConv = await apiRequest<Conversation>('/api/v1/conversations/create', {
      method: 'POST',
      body: JSON.stringify({
        title,
        is_group: true,
        participant_ids: participantIds,
      }),
    });
    state.showNewGroupModal = false;
    await loadConversations();
    state.activeConversationId = newConv.id;
    state.isMobileChatOpen = true;
    await loadMessages(newConv.id);
    showToast(`Group '${title}' created!`);
    render();
  } catch (e: any) {
    showToast(`Failed to create group: ${e.message}`);
  }
}

async function startDirectChat(agentId: string) {
  try {
    const conv = await apiRequest<Conversation>('/api/v1/conversations/direct', {
      method: 'POST',
      body: JSON.stringify({ agent_id: agentId }),
    });
    await loadConversations();
    state.activeConversationId = conv.id;
    state.isMobileChatOpen = true;
    await loadMessages(conv.id);
    render();
  } catch (e: any) {
    showToast(`Error opening direct chat: ${e.message}`);
  }
}

async function stopActiveConversation() {
  if (!state.activeConversationId) return;
  try {
    await apiRequest(`/api/v1/conversations/${state.activeConversationId}/stop`, {
      method: 'POST',
    });
    showToast('Conversation session stopped');
    await loadConversations();
    await loadMessages(state.activeConversationId);
    render();
  } catch (e: any) {
    showToast(`Failed to stop group: ${e.message}`);
  }
}

async function deleteActiveConversation() {
  if (!state.activeConversationId) return;
  if (!confirm('Are you sure you want to delete this conversation?')) return;
  try {
    await apiRequest(`/api/v1/conversations/${state.activeConversationId}`, {
      method: 'DELETE',
    });
    showToast('Conversation deleted');
    state.activeConversationId = null;
    state.isMobileChatOpen = false;
    await loadConversations();
    if (state.conversations.length > 0) {
      state.activeConversationId = state.conversations[0].id;
      await loadMessages(state.conversations[0].id);
    }
    render();
  } catch (e: any) {
    showToast(`Failed to delete: ${e.message}`);
  }
}

async function launchApp(appId: string) {
  try {
    const res = await apiRequest<string>('/api/v1/apps/launch', {
      method: 'POST',
      body: JSON.stringify({ app_id: appId }),
    });
    showToast(res);
  } catch (e: any) {
    showToast(`Launch failed: ${e.message}`);
  }
}

async function executeTerminalCommand(command: string, isSudo: boolean, sudoPassword?: string) {
  if (!command.trim()) return;
  try {
    const res = await apiRequest<{
      exit_code: number;
      stdout: string;
      stderr: string;
      duration_ms: number;
    }>('/api/v1/terminal/execute', {
      method: 'POST',
      body: JSON.stringify({
        command,
        is_sudo: isSudo,
        sudo_password: sudoPassword || null,
        cwd: state.currentPath === '~' ? '.' : state.currentPath,
      }),
    });

    state.terminalLogs.unshift({
      cmd: command,
      stdout: res.stdout,
      stderr: res.stderr,
      exit_code: res.exit_code,
      duration_ms: res.duration_ms,
      is_sudo: isSudo,
    });
    state.showSudoModal = false;
    state.sudoPassword = '';
    render();
  } catch (e: any) {
    showToast(`Execution failed: ${e.message}`);
  }
}

function scrollToBottom() {
  setTimeout(() => {
    const container = document.getElementById('chat-messages-container');
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, 50);
}

// Polling Loop
let pollTimer: any = null;
function startPolling() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = setInterval(async () => {
    try {
      await loadSessions();
      if (state.currentTab === 'chats' && state.activeConversationId) {
        await loadMessages(state.activeConversationId, false);
      }
      if (state.currentTab === 'machines') {
        await loadMachinesOverview(false);
      }
    } catch {}
  }, 3000);
}

// RENDERERS
function render() {
  const app = document.getElementById('app');
  if (!app) return;

  app.innerHTML = `
    <!-- Top Bar -->
    <header class="app-header">
      <div class="header-left">
        <div class="rain-brand">
          <span class="rain-cloud-logo">🌧️</span>
          <div class="brand-text">
            <span class="brand-name">RAIN</span>
            <span class="brand-sub">Agent & Machine Mesh</span>
          </div>
        </div>
      </div>
      <div class="header-right">
        <button class="btn-pair-invite-top" id="btn-open-pairing" title="Pair Device or Invite Coworker/Friend">
          <span class="pair-icon">🔗</span>
          <span class="pair-text">Pair Hub</span>
        </button>
        <div class="mesh-pill" id="btn-mesh-pill">
          <span class="status-indicator-dot ${state.connectionStatus === 'online' ? 'dot-online' : 'dot-offline'}"></span>
          <span class="mesh-latency">${state.connectionStatus === 'online' ? `${state.latencyMs}ms` : 'Offline'}</span>
          <span class="transport-tag">${state.meshStatus ? state.meshStatus.current_transport.replace('_', ' ').toUpperCase() : 'USB'}</span>
        </div>
      </div>
    </header>

    <!-- Main Content Area -->
    <main class="app-body">
      ${renderTabContent()}
    </main>

    <!-- Bottom Navigation Bar (Mobile / Responsive) -->
    <nav class="bottom-nav">
      <button class="nav-item ${state.currentTab === 'chats' ? 'nav-active' : ''}" data-tab="chats">
        <span class="nav-icon">💬</span>
        <span class="nav-label">Chats</span>
      </button>
      <button class="nav-item ${state.currentTab === 'machines' ? 'nav-active' : ''}" data-tab="machines">
        <span class="nav-icon">🖥️</span>
        <span class="nav-label">Machines</span>
      </button>
      <button class="nav-item ${state.currentTab === 'files' ? 'nav-active' : ''}" data-tab="files">
        <span class="nav-icon">📁</span>
        <span class="nav-label">Files</span>
      </button>
      <button class="nav-item ${state.currentTab === 'mesh' ? 'nav-active' : ''}" data-tab="mesh">
        <span class="nav-icon">🌐</span>
        <span class="nav-label">Mesh</span>
      </button>
    </nav>

    <!-- Modals & Overlays -->
    ${renderModals()}
    <div id="rain-toast-banner" class="toast-banner" style="display: none;"></div>
  `;

  attachEventHandlers();
}

function renderTabContent(): string {
  switch (state.currentTab) {
    case 'chats':
      return renderChatsView();
    case 'machines':
      return renderMachinesView();
    case 'files':
      return renderFilesView();
    case 'mesh':
      return renderMeshView();
  }
}

// 1. CHATS TAB
function renderChatsView(): string {
  const activeConv = state.conversations.find(c => c.id === state.activeConversationId);
  const filteredConvs = state.conversations.filter(c => {
    if (state.chatFilter === 'groups') return c.is_group;
    if (state.chatFilter === 'direct') return !c.is_group;
    return true;
  });

  return `
    <div class="chats-screen-layout ${state.isMobileChatOpen ? 'mobile-show-chat' : 'mobile-show-roster'}">
      <!-- Roster Column -->
      <section class="roster-panel">
        <!-- Agent Presence Stories Row -->
        ${state.sessions.length > 0 ? `
          <div class="presence-stories-section">
            <div class="stories-label">ACTIVE AGENTS (TAP FOR 1-ON-1)</div>
            <div class="stories-scroll">
              ${state.sessions.map(s => `
                <div class="story-avatar-item" data-direct-agent="${s.id}">
                  <div class="avatar-ring ring-${s.state}">
                    <span class="avatar-icon">🧠</span>
                    <span class="story-dot dot-${s.state}"></span>
                  </div>
                  <span class="story-name">${s.name.split('—')[1]?.trim() || s.name.slice(0, 10)}</span>
                </div>
              `).join('')}
            </div>
          </div>
        ` : ''}

        <!-- Filter & New Group Bar -->
        <div class="roster-toolbar">
          <div class="filter-pills">
            <button class="filter-pill ${state.chatFilter === 'all' ? 'pill-active' : ''}" data-filter="all">All</button>
            <button class="filter-pill ${state.chatFilter === 'groups' ? 'pill-active' : ''}" data-filter="groups">Groups</button>
            <button class="filter-pill ${state.chatFilter === 'direct' ? 'pill-active' : ''}" data-filter="direct">Direct</button>
          </div>
          <button class="btn-new-group" id="btn-open-new-group">+ New Group</button>
        </div>

        <!-- Conversation List -->
        <div class="conversation-list">
          ${filteredConvs.map(conv => `
            <div class="conv-item ${conv.id === state.activeConversationId ? 'conv-active' : ''} ${!conv.is_active ? 'conv-stopped' : ''}" data-conv-id="${conv.id}">
              <div class="conv-avatar">
                <span>${conv.is_group ? '👥' : '👤'}</span>
              </div>
              <div class="conv-details">
                <div class="conv-top-line">
                  <span class="conv-title">${conv.title}</span>
                  <span class="conv-time">${conv.last_message_time ? formatTime(conv.last_message_time) : ''}</span>
                </div>
                <div class="conv-preview-line">
                  <span class="conv-preview">${conv.last_message || 'No messages yet'}</span>
                  ${!conv.is_active ? '<span class="badge-stopped">STOPPED</span>' : ''}
                </div>
              </div>
            </div>
          `).join('')}
          ${filteredConvs.length === 0 ? '<div class="empty-roster">No conversations found. Tap "+ New Group" to start one.</div>' : ''}
        </div>
      </section>

      <!-- Active Chat Column -->
      <section class="active-chat-panel">
        ${activeConv ? `
          <div class="chat-header">
            <button class="mobile-back-btn" id="btn-back-to-roster">‹</button>
            <div class="chat-header-info">
              <div class="chat-header-title-row">
                <span class="chat-header-title">${activeConv.title}</span>
                <span class="badge-tag">${activeConv.is_group ? 'Group' : 'Direct'}</span>
                ${!activeConv.is_active ? '<span class="badge-stopped">Stopped</span>' : ''}
              </div>
              <span class="chat-header-sub">
                ${activeConv.is_group ? `● ${activeConv.participant_ids.length - 1} agents connected` : '● 1-on-1 Direct Agent Session'}
              </span>
            </div>
            <div class="chat-header-actions">
              ${activeConv.is_active ? `
                <button class="btn-icon-action" id="btn-stop-group" title="Stop Group / Suspend Agents">🛑</button>
              ` : ''}
              <button class="btn-icon-action btn-danger" id="btn-delete-group" title="Delete Conversation">🗑️</button>
            </div>
          </div>

          <!-- Message Flow -->
          <div class="chat-messages-container" id="chat-messages-container">
            ${state.messages.map(msg => renderMessageBubble(msg)).join('')}
          </div>

          <!-- Quick Action Prompt Pills -->
          <div class="quick-prompts-bar">
            <button class="prompt-pill prompt-pill-evac" data-prompt="⚠️ Battery at 5%: execute autonomous git failover and evacuate task">🔋 5% Battery Evacuate</button>
            <button class="prompt-pill" data-prompt="Check API health">⚡ Check API health</button>
            <button class="prompt-pill" data-prompt="Review git diff">⚡ Review git diff</button>
          </div>

          <!-- Attachment Preview Box (if selected) -->
          ${state.pendingAttachment ? `
            <div class="attachment-preview-banner">
              <span class="att-icon">${state.pendingAttachment.mime_type.startsWith('image') ? '🖼️' : '📎'}</span>
              <span class="att-name">${state.pendingAttachment.name}</span>
              <span class="att-size">(${formatBytes(state.pendingAttachment.size_bytes)})</span>
              <button class="att-remove" id="btn-remove-attachment">✕</button>
            </div>
          ` : ''}

          <!-- Chat Input Controls -->
          <div class="chat-input-toolbar">
            <div class="input-controls-row">
              <select class="agent-dispatch-select" id="agent-dispatch-select">
                <option value="all">@All</option>
                ${state.sessions.map(s => `
                  <option value="${s.id}" ${state.targetSessionId === s.id ? 'selected' : ''}>@${s.name}</option>
                `).join('')}
              </select>

              <button class="btn-attachment" id="btn-toggle-attachments" title="Add Image or File">📎</button>
              
              <input 
                type="text" 
                id="chat-text-input" 
                class="chat-text-input" 
                placeholder="${activeConv.is_active ? 'Message agents or delegate task...' : 'Conversation is stopped'}" 
                ${!activeConv.is_active ? 'disabled' : ''}
              />

              <button class="btn-send-message" id="btn-send-message" ${!activeConv.is_active ? 'disabled' : ''}>
                <span>➤</span>
              </button>
            </div>
          </div>
        ` : `
          <div class="no-chat-selected">
            <span class="watermark-icon">🌧️</span>
            <h3>RAIN Agent Messenger</h3>
            <p>Select a conversation or tap an agent above to start a direct session.</p>
          </div>
        `}
      </section>
    </div>
  `;
}

function renderMessageBubble(msg: ChatMessage): string {
  const isHuman = msg.sender_type === 'human';
  const isSystem = msg.sender_type === 'system';

  if (isSystem) {
    return `
      <div class="system-message-row">
        <span class="system-bubble">${msg.text}</span>
      </div>
    `;
  }

  return `
    <div class="message-bubble-row ${isHuman ? 'row-human' : 'row-agent'}">
      <div class="message-bubble ${isHuman ? 'bubble-human' : 'bubble-agent'}">
        ${!isHuman ? `
          <div class="bubble-sender-title">
            <span class="sender-name">${msg.sender_name}</span>
          </div>
        ` : ''}
        
        <!-- Attached Images or Artifacts -->
        ${msg.attachments && msg.attachments.length > 0 ? `
          <div class="bubble-attachments-box">
            ${msg.attachments.map(att => {
              if (att.mime_type.startsWith('image/')) {
                return `
                  <div class="image-attachment-card">
                    <img src="${att.data_base64 || ('data:image/svg+xml;utf8,' + encodeURIComponent('<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"120\"><rect width=\"200\" height=\"120\" fill=\"#161F31\"/><text x=\"50%\" y=\"50%\" dominant-baseline=\"middle\" text-anchor=\"middle\" fill=\"#06B6D4\" font-family=\"sans-serif\">' + att.name + '</text></svg>'))}" alt="${att.name}" class="attached-img-preview" />
                    <span class="img-caption">${att.name}</span>
                  </div>
                `;
              } else {
                return `
                  <div class="artifact-attachment-card">
                    <div class="artifact-header">
                      <span>📎 ${att.name}</span>
                      <span class="artifact-size">${formatBytes(att.size_bytes)}</span>
                    </div>
                    ${att.preview_content ? `<pre class="artifact-preview"><code>${att.preview_content}</code></pre>` : ''}
                  </div>
                `;
              }
            }).join('')}
          </div>
        ` : ''}

        <div class="bubble-text">${msg.text}</div>

        <!-- Cross-Machine Job Delegation Card -->
        ${msg.job_delegation ? `
          <div class="job-delegation-card">
            <div class="job-card-header">
              <span class="job-badge-icon">🔄</span>
              <div class="job-route-wrap">
                <span class="job-route-label">Distributed Cloud Job</span>
                <span class="job-route-path">${msg.job_delegation.source_machine} ➔ ${msg.job_delegation.target_machine}</span>
              </div>
              <span class="job-status-pill status-${msg.job_delegation.status}">
                ${msg.job_delegation.status.replace('_', ' ').toUpperCase()}
              </span>
            </div>

            <div class="job-task-text">${msg.job_delegation.task_instruction}</div>

            <div class="job-tags-row">
              ${msg.job_delegation.required_resources.map(r => `<span class="job-resource-tag">⚡ ${r}</span>`).join('')}
            </div>

            <!-- Live Progress Bar -->
            <div class="job-progress-wrap">
              <div class="progress-bar-bg">
                <div class="progress-bar-fill ${msg.job_delegation.status === 'completed' ? 'fill-emerald' : 'fill-cyan'}" style="width: ${msg.job_delegation.progress_percent}%"></div>
              </div>
              <span class="job-progress-text">${msg.job_delegation.progress_percent}% complete</span>
            </div>

            <!-- Gated Approval Controls -->
            ${msg.job_delegation.status === 'requires_approval' && !msg.job_delegation.is_approved ? `
              <div class="job-approval-prompt">
                <div class="approval-warning-text">🛑 Sudo root command proposed on ${msg.job_delegation.target_machine}:</div>
                <code class="approval-cmd-preview">${msg.job_delegation.sudo_command || 'sudo task'}</code>
                <div class="approval-actions-row">
                  <button class="btn-job-approve" data-approve-job="${msg.job_delegation.id}">✅ Authorize Sudo Task</button>
                  <button class="btn-job-reject" data-reject-job="${msg.job_delegation.id}">❌ Reject</button>
                </div>
              </div>
            ` : ''}

            ${msg.job_delegation.result_summary ? `
              <div class="job-result-summary">
                <span class="summary-icon">✅</span>
                <span>${msg.job_delegation.result_summary}</span>
              </div>
            ` : ''}
          </div>
        ` : ''}

        <div class="bubble-footer">
          <span class="bubble-timestamp">${formatTime(msg.timestamp)}</span>
          ${isHuman ? `<span class="delivery-check">✓✓</span>` : ''}
        </div>
      </div>
    </div>
  `;
}

function renderMachinesCards(): string {
  return state.machines.map(m => {
    const memUsed = formatBytes(m.memory_used_bytes);
    const memTotal = formatBytes(m.memory_total_bytes);
    const diskUsed = formatBytes(m.disk_used_bytes);
    const diskTotal = formatBytes(m.disk_total_bytes);
    const memPercent = Math.round((m.memory_used_bytes / (m.memory_total_bytes || 1)) * 100);

    return `
      <div class="machine-card">
        <div class="machine-card-header">
          <div class="machine-identity">
            <span class="platform-icon">${m.platform === 'linux' ? '🐧' : m.platform === 'macos' ? '🍎' : '💻'}</span>
            <div>
              <h3 class="machine-name">${m.name}</h3>
              <span class="machine-ip">${m.ip_address}</span>
            </div>
          </div>
          <span class="badge-online">ONLINE</span>
        </div>

        <!-- Hardware Telemetry Gauges -->
        <div class="telemetry-box">
          <div class="gauge-row">
            <span class="gauge-label">CPU (${m.cpu_usage.toFixed(1)}%)</span>
            <div class="progress-bar-bg">
              <div class="progress-bar-fill fill-cpu" style="width: ${Math.min(100, Math.max(5, m.cpu_usage))}%"></div>
            </div>
          </div>
          <div class="gauge-row">
            <span class="gauge-label">RAM (${memPercent}%)</span>
            <div class="progress-bar-bg">
              <div class="progress-bar-fill fill-ram" style="width: ${memPercent}%"></div>
            </div>
            <span class="gauge-sub">${memUsed} / ${memTotal}</span>
          </div>
          <div class="gauge-row">
            <span class="gauge-label">DISK STORAGE</span>
            <span class="gauge-sub">${diskUsed} / ${diskTotal}</span>
          </div>

          ${m.battery ? `
            <div class="gauge-row battery-gauge-row">
              <div class="battery-head-row">
                <span class="gauge-label">POWER SUPPLY</span>
                <span class="battery-pill-badge ${m.battery.is_critical ? 'badge-critical-power' : m.battery.is_ac_connected ? 'badge-ac-power' : 'badge-battery-power'}">
                  ${m.battery.is_critical ? '⚠️ 5% CRITICAL (DISCHARGING)' : m.battery.is_ac_connected ? `⚡ ${m.battery.percent}% (${m.battery.power_source})` : `🔋 ${m.battery.percent}% (${m.battery.power_source})`}
                </span>
              </div>
              <div class="progress-bar-bg">
                <div class="progress-bar-fill ${m.battery.is_critical ? 'fill-battery-critical' : 'fill-battery'}" style="width: ${m.battery.percent}%"></div>
              </div>
              ${m.battery.is_critical ? `
                <div class="battery-failover-action-bar">
                  <span class="time-left-notice">⏱️ ~${m.battery.time_remaining_minutes || 8} min before shutdown</span>
                  <button class="btn-evacuate-now" id="btn-evacuate-now-click">⚡ Evacuate Task to Git Remote</button>
                </div>
              ` : ''}
            </div>
          ` : ''}
        </div>

        <div class="machine-card-footer">
          <span>⏱️ Uptime: ${formatUptime(m.uptime_seconds)}</span>
          <span>🤖 ${m.active_agents_count} agents</span>
        </div>
      </div>
    `;
  }).join('');
}

// 2. MACHINES TAB
function renderMachinesView(): string {
  return `
    <div class="tab-screen-container">
      <div class="section-title-row">
        <h2>Remote Machines & Workstations</h2>
        <span class="nodes-badge">${state.machines.length} active nodes</span>
      </div>

      <!-- Active Machines Cards Grid -->
      <div class="machines-grid">
        ${renderMachinesCards()}
      </div>

      <!-- Installed Applications Shelf -->
      <div class="apps-shelf-section">
        <div class="shelf-header">
          <h3>Launch Applications on Host</h3>
          <span class="shelf-desc">Execute desktop or background applications directly from mobile</span>
        </div>
        <div class="apps-grid">
          ${state.installedApps.map(app => `
            <div class="app-card" data-launch-app="${app.id}">
              <div class="app-icon-wrap">
                <span class="app-icon">${app.id === 'code' ? '💻' : app.id === 'firefox' ? '🦊' : app.id === 'blender' ? '🎨' : app.id === 'godot' ? '🎮' : '⚡'}</span>
              </div>
              <div class="app-info">
                <span class="app-title">${app.name}</span>
                <span class="app-category">${app.category}</span>
              </div>
              <button class="btn-launch-pill">Launch</button>
            </div>
          `).join('')}
        </div>
      </div>

      <!-- Remote Terminal & Sudo Command Runner -->
      <div class="terminal-runner-section">
        <div class="terminal-header">
          <h3>Host Terminal & Privileged Sudo Runner</h3>
          <span class="terminal-desc">Run commands remotely with optional sudo root elevation</span>
        </div>

        <div class="terminal-controls-box">
          <div class="command-input-row">
            <input type="text" id="terminal-cmd-input" class="terminal-input" placeholder="e.g. systemctl status rain-daemon, df -h, uname -a" />
            <button class="btn-terminal-run" id="btn-run-terminal">Execute</button>
          </div>

          <div class="terminal-options-row">
            <label class="sudo-toggle-label">
              <input type="checkbox" id="check-sudo" ${state.isSudoChecked ? 'checked' : ''} />
              <span>Run as sudo (Privileged)</span>
            </label>
            <div class="quick-commands">
              <button class="quick-cmd-btn" data-quick-cmd="uname -a">uname -a</button>
              <button class="quick-cmd-btn" data-quick-cmd="df -h">df -h</button>
              <button class="quick-cmd-btn" data-quick-cmd="free -m">free -m</button>
              <button class="quick-cmd-btn" data-quick-cmd="docker ps">docker ps</button>
            </div>
          </div>
        </div>

        <!-- Terminal Output Console -->
        <div class="terminal-console-output">
          ${state.terminalLogs.map(log => `
            <div class="terminal-entry">
              <div class="entry-cmd-bar">
                <span class="prompt-sym">$</span>
                <span class="executed-cmd">${log.is_sudo ? 'sudo ' : ''}${log.cmd}</span>
                <span class="entry-meta">exit: ${log.exit_code} (${log.duration_ms}ms)</span>
              </div>
              ${log.stdout ? `<pre class="stdout-box">${log.stdout}</pre>` : ''}
              ${log.stderr ? `<pre class="stderr-box">${log.stderr}</pre>` : ''}
            </div>
          `).join('')}
          ${state.terminalLogs.length === 0 ? '<div class="terminal-empty">Output from executed commands will appear here.</div>' : ''}
        </div>
      </div>

      <!-- Decentralized Personal Cloud & Shared Resources -->
      <div class="cloud-resources-section">
        <div class="shelf-header">
          <h3>Decentralized Personal Cloud & Resource Mesh</h3>
          <span class="shelf-desc">Hardware accelerators and specialized runtimes shared across your machines</span>
        </div>
        <div class="resources-grid">
          ${state.resources.map(res => `
            <div class="resource-card">
              <div class="res-card-top">
                <span class="res-machine-title">${res.machine_name}</span>
                <span class="res-platform-tag">${res.platform.toUpperCase()}</span>
              </div>
              <div class="res-specs">
                <span>🖥️ ${res.cpu_cores} Cores CPU</span>
                <span>🧠 ${res.ram_free_gb} GB Free / ${res.ram_total_gb} GB</span>
                ${res.gpu_model ? `<span class="res-gpu">⚡ ${res.gpu_model}</span>` : ''}
                <span>📶 ${res.network_speed_mbps} Mbps Link</span>
              </div>
              <div class="res-tags">
                ${res.tags.map(t => `<span class="res-tag">${t}</span>`).join('')}
              </div>
            </div>
          `).join('')}
        </div>
      </div>

      <!-- Active Cross-Machine Delegated Jobs -->
      <div class="delegated-jobs-section">
        <div class="shelf-header">
          <h3>Cross-Machine Delegated Jobs</h3>
          <span class="shelf-desc">Autonomous tasks executing or transferring between your devices</span>
        </div>
        <div class="jobs-list">
          ${state.jobs.map(job => `
            <div class="job-list-card">
              <div class="job-card-top">
                <span class="job-route-path">${job.source_machine} ➔ ${job.target_machine}</span>
                <span class="job-status-pill status-${job.status}">${job.status.replace('_', ' ').toUpperCase()}</span>
              </div>
              <div class="job-task-text">${job.task_instruction}</div>
              <div class="job-progress-wrap">
                <div class="progress-bar-bg">
                  <div class="progress-bar-fill ${job.status === 'completed' ? 'fill-emerald' : 'fill-cyan'}" style="width: ${job.progress_percent}%"></div>
                </div>
                <span class="job-progress-text">${job.progress_percent}%</span>
              </div>
              ${job.result_summary ? `<div class="job-res-sub">✅ ${job.result_summary}</div>` : ''}
            </div>
          `).join('')}
          ${state.jobs.length === 0 ? '<div class="terminal-empty">No cross-machine jobs active.</div>' : ''}
        </div>
      </div>
    </div>
  `;
}

// 3. FILES TAB
function renderFilesView(): string {
  if (state.editingFile) {
    return renderEditorView();
  }

  return `
    <div class="tab-screen-container">
      <div class="files-header-bar">
        <h2>Remote File Explorer</h2>
        <button class="btn-refresh-files" id="btn-refresh-files">🔄 Refresh</button>
      </div>

      <!-- Breadcrumbs Bar -->
      <div class="breadcrumbs-row">
        <span class="bc-label">Path:</span>
        <span class="bc-path">${state.currentPath}</span>
      </div>

      <!-- File List -->
      <div class="file-browser-list">
        ${state.fileEntries.map(entry => `
          <div class="file-item-row" data-file-path="${entry.path}" data-is-dir="${entry.is_dir}" data-is-editable="${entry.is_editable}" data-file-name="${entry.name}">
            <span class="file-icon">${entry.name === '..' ? '⤴️' : entry.is_dir ? '📁' : entry.is_editable ? '📝' : '📄'}</span>
            <div class="file-meta-col">
              <span class="file-name">${entry.name}</span>
              <span class="file-sub-meta">
                ${entry.is_dir ? 'Directory' : formatBytes(entry.size_bytes)}
                ${entry.is_editable ? ' • Tap to Edit' : ''}
              </span>
            </div>
            ${entry.is_editable ? '<span class="action-tag">Edit</span>' : ''}
          </div>
        `).join('')}
      </div>
    </div>
  `;
}

function renderEditorView(): string {
  if (!state.editingFile) return '';

  return `
    <div class="editor-screen-container">
      <div class="editor-top-bar">
        <div class="editor-title-wrap">
          <button class="btn-editor-back" id="btn-close-editor">‹ Back</button>
          <span class="editor-file-name">${state.editingFile.name}</span>
          <span class="editor-file-path">${state.editingFile.path}</span>
        </div>
        <button class="btn-save-file" id="btn-save-file">💾 Save to Machine</button>
      </div>

      <div class="editor-body">
        <textarea id="file-editor-textarea" class="file-editor-textarea" spellcheck="false">${state.editingFile.content}</textarea>
      </div>
    </div>
  `;
}

// 4. MESH TAB
function renderMeshView(): string {
  const mesh = state.meshStatus;

  return `
    <div class="tab-screen-container">
      <div class="section-title-row">
        <h2>Network-Agnostic Mesh</h2>
        <span class="badge-online">E2EE ACTIVE</span>
      </div>

      <div class="mesh-info-card">
        <div class="mesh-stat-row">
          <span class="mesh-stat-title">Current Transport:</span>
          <span class="mesh-stat-value text-accent">⚡ ${mesh ? mesh.current_transport.replace('_', ' ').toUpperCase() : 'DIRECT USB TUNNEL'}</span>
        </div>
        <div class="mesh-stat-row">
          <span class="mesh-stat-title">Cryptographic Node ID:</span>
          <span class="mesh-stat-value font-mono">${mesh ? mesh.node_id : 'ed25519:7a4f91b8e3c0'}</span>
        </div>
        <div class="mesh-stat-row">
          <span class="mesh-stat-title">Round-trip Latency:</span>
          <span class="mesh-stat-value mesh-stat-value-latency">${state.latencyMs} ms</span>
        </div>
        <div class="mesh-stat-row">
          <span class="mesh-stat-title">Encryption Suite:</span>
          <span class="mesh-stat-value">ChaCha20-Poly1305 (Noise Protocol)</span>
        </div>
      </div>

      <!-- Star Topology Hub & Spokes Cluster -->
      <div class="star-topology-section">
        <div class="star-header-row">
          <div>
            <h3>Star Topology Cluster</h3>
            <span class="topology-sub">One central Sovereign Hub coordinating all peer spokes</span>
          </div>
          <button class="btn-mesh-add-spoke" id="btn-mesh-add-spoke">
            <span>+ Pair New Spoke / QR</span>
          </button>
        </div>

        <!-- Central Hub Card -->
        <div class="hub-center-card">
          <div class="hub-badge-row">
            <span class="hub-role-badge">⭐ CENTRAL HUB SERVER</span>
            <span class="hub-status-live">🟢 ONLINE</span>
          </div>
          <h4 class="hub-title">${state.hubTopology?.hub_name || 'RAIN-Primary-Hub'}</h4>
          <div class="hub-details-grid">
            <div><span class="lbl">Listen Address:</span> <code>${state.hubTopology?.hub_listen_addr || '0.0.0.0:7890'}</code></div>
            <div><span class="lbl">Node ID:</span> <code>${state.hubTopology?.hub_node_id || 'ed25519:7a4f91b8'}</code></div>
            <div><span class="lbl">Active Spokes:</span> <strong>${state.hubTopology?.active_spokes_count || 0} connected</strong></div>
            <div><span class="lbl">Uptime:</span> <strong>${formatUptime(state.hubTopology?.uptime_seconds || 0)}</strong></div>
          </div>
        </div>

        <!-- Enrolled Spoke Devices -->
        <div class="spokes-list-container">
          <div class="spokes-list-title">CONNECTED SPOKE DEVICES (${state.hubTopology?.spokes?.length || 0})</div>
          <div class="spokes-grid">
            ${(state.hubTopology?.spokes && state.hubTopology.spokes.length > 0) ? state.hubTopology.spokes.map((s: any) => `
              <div class="spoke-card ${s.is_online ? 'spoke-online' : 'spoke-offline'}">
                <div class="spoke-card-left">
                  <span class="spoke-icon">${s.platform === 'android' ? '📱' : s.platform === 'macos' ? '🍎' : '💻'}</span>
                  <div class="spoke-info">
                    <div class="spoke-title-row">
                      <span class="spoke-name">${s.device_name}</span>
                      <span class="spoke-role-tag ${s.role === 'admin' ? 'role-admin' : 'role-collab'}">${s.role.toUpperCase()}</span>
                    </div>
                    <span class="spoke-addr">${s.ip_address || 'Connected'}</span>
                  </div>
                </div>
                <div class="spoke-card-right">
                  <span class="spoke-dot ${s.is_online ? 'dot-active' : 'dot-dormant'}"></span>
                  ${s.role !== 'admin' ? `
                    <button class="btn-revoke-action" data-revoke-device="${s.device_id}" title="Revoke Device Access">Revoke</button>
                  ` : '<span class="this-device-tag">This Device</span>'}
                </div>
              </div>
            `).join('') : `
              <div class="empty-spokes" style="padding: 16px; color: var(--text-muted); font-size: 13px; grid-column: 1 / -1; text-align: center; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px dashed var(--border-subtle);">
                No spoke worker nodes enrolled yet. Use "+ Pair New Spoke" above to connect another machine or mobile device.
              </div>
            `}
          </div>
        </div>
      </div>

      <!-- Autonomous Transport Fallback Hierarchy -->
      <div class="hierarchy-section">
        <h3>Autonomous Multi-Tier Transport Hierarchy</h3>
        <p class="hierarchy-desc">RAIN routes seamlessly across whatever link is currently reachable:</p>

        <div class="tier-card tier-active">
          <div class="tier-header">
            <span class="tier-badge">Tier 1 • Active</span>
            <span class="tier-name">Direct Local / USB Tether</span>
          </div>
          <span class="tier-detail">&lt;1ms latency • Zero-setup reverse tunnel • Offline capable</span>
        </div>

        <div class="tier-card">
          <div class="tier-header">
            <span class="tier-badge tier-ready">Tier 2 • Ready</span>
            <span class="tier-name">LAN Multicast (mDNS ZeroConf)</span>
          </div>
          <span class="tier-detail">Automatic discovery on local Wi-Fi • No manual IP entry</span>
        </div>

        <div class="tier-card">
          <div class="tier-header">
            <span class="tier-badge tier-ready">Tier 3 • Ready</span>
            <span class="tier-name">NAT-Traversing P2P Overlay</span>
          </div>
          <span class="tier-detail">Direct encrypted UDP hole punching over 5G/LTE cellular</span>
        </div>

        <div class="tier-card">
          <div class="tier-header">
            <span class="tier-badge tier-ready">Tier 4 • Standby</span>
            <span class="tier-name">Zero-Trust Fallback Relay</span>
          </div>
          <span class="tier-detail">E2E encrypted WebSocket tunnel for strict corporate firewalls</span>
        </div>
      </div>

      <!-- Host Settings -->
      <div class="host-config-section">
        <h3>Host Endpoint Connection</h3>
        <div class="host-input-row">
          <input type="text" id="host-url-input" class="host-input" value="${state.hostUrl}" />
          <button class="btn-save-host" id="btn-save-host">Update</button>
        </div>
      </div>
    </div>
  `;
}

// MODALS
function renderModals(): string {
  let html = '';

  // New Group Modal
  if (state.showNewGroupModal) {
    html += `
      <div class="modal-backdrop">
        <div class="modal-card">
          <div class="modal-header">
            <h3>Create Agent Group</h3>
            <button class="modal-close-btn" id="btn-close-group-modal">✕</button>
          </div>
          <div class="modal-body">
            <label class="modal-label">Group Name</label>
            <input type="text" id="new-group-title" class="modal-text-input" placeholder="e.g. Mobile QA Strike Team" />

            <label class="modal-label" style="margin-top: 16px;">Select Agents to Include</label>
            <div class="group-agents-select-list">
              ${state.sessions.map(s => `
                <label class="agent-check-row">
                  <input type="checkbox" class="agent-cb" value="${s.id}" checked />
                  <span class="cb-agent-name">${s.name}</span>
                  <span class="cb-machine-tag">${s.machine_name}</span>
                </label>
              `).join('')}
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn-modal-cancel" id="btn-cancel-group">Cancel</button>
            <button class="btn-modal-submit" id="btn-submit-new-group">Create Group</button>
          </div>
        </div>
      </div>
    `;
  }

  // Sudo Password Modal
  if (state.showSudoModal) {
    html += `
      <div class="modal-backdrop">
        <div class="modal-card">
          <div class="modal-header">
            <h3>Sudo Authentication Required</h3>
            <button class="modal-close-btn" id="btn-close-sudo-modal">✕</button>
          </div>
          <div class="modal-body">
            <p class="modal-desc">This command requires root elevation on the host machine:</p>
            <pre class="sudo-command-preview"><code>sudo ${state.pendingSudoCommand}</code></pre>
            <label class="modal-label" style="margin-top: 14px;">Host Password</label>
            <input type="password" id="sudo-password-input" class="modal-text-input" placeholder="Enter sudo password" />
          </div>
          <div class="modal-footer">
            <button class="btn-modal-cancel" id="btn-cancel-sudo">Cancel</button>
            <button class="btn-modal-submit" id="btn-confirm-sudo">Authorize & Execute</button>
          </div>
        </div>
      </div>
    `;
  }

  // Attachment Menu
  if (state.showAttachmentMenu) {
    html += `
      <div class="modal-backdrop" id="backdrop-attachment">
        <div class="attachment-popup-menu">
          <div class="menu-title">Attach to Message</div>
          <button class="menu-action-btn" id="btn-attach-image">
            <span>🖼️ Send Image (UI Screenshot)</span>
          </button>
          <button class="menu-action-btn" id="btn-attach-artifact">
            <span>📝 Send Code Artifact (JSON / Diff)</span>
          </button>
        </div>
      </div>
    `;
  }

  // Pairing & Sovereign Hub Modal
  if (state.showPairingModal) {
    html += `
      <div class="modal-backdrop">
        <div class="modal-card modal-pairing-card">
          <div class="modal-header">
            <div>
              <h3>Pair Devices & Sovereign Hub</h3>
              <span class="modal-header-sub">Setup once with QR/PIN. Automatic permanent reconnection thereafter.</span>
            </div>
            <button class="modal-close-btn" id="btn-close-pairing-modal">✕</button>
          </div>

          <!-- Pairing Tabs -->
          <div class="pairing-tabs-bar">
            <button class="pair-tab-btn ${state.pairingTab === 'host' ? 'pair-tab-active' : ''}" data-pair-tab="host">
              📱 Host Mode (QR / Invite)
            </button>
            <button class="pair-tab-btn ${state.pairingTab === 'pin' ? 'pair-tab-active' : ''}" data-pair-tab="pin">
              🔢 Enter 6-Digit PIN
            </button>
            <button class="pair-tab-btn ${state.pairingTab === 'email' ? 'pair-tab-active' : ''}" data-pair-tab="email">
              ✉️ Email OTP
            </button>
          </div>

          <div class="modal-body pairing-modal-body">
            ${state.pairingTab === 'host' ? `
              <div class="pairing-host-view">
                <p class="pair-guide-text">Scan with your phone or share with a friend to establish an E2E encrypted link:</p>
                <div class="qr-render-box">
                  ${state.pairingTicket?.qr_svg || '<div class="qr-loading">Generating secure QR code...</div>'}
                </div>
                <div class="pin-code-showcase">
                  <span class="pin-label">ONE-TIME 6-DIGIT MACHINE PIN</span>
                  <div class="pin-digits-row">
                    ${(state.pairingTicket?.pairing_code || '849201').split('').map(d => `<span class="pin-box">${d}</span>`).join('')}
                  </div>
                  <button class="btn-copy-code" id="btn-copy-pairing-code">📋 Copy 6-Digit PIN</button>
                </div>
                <div class="link-copy-box">
                  <input type="text" readonly class="pair-link-input" value="${state.pairingTicket?.deep_link || 'rain://pair?code=849201&host=Home-Hub'}" />
                  <button class="btn-copy-link" id="btn-copy-pairing-link">Copy Link</button>
                </div>
                <span class="pair-expires-tag">⏳ Code valid for 10 minutes • Permanent cryptographic token exchanged on claim</span>
              </div>
            ` : state.pairingTab === 'pin' ? `
              <div class="pairing-pin-view">
                <p class="pair-guide-text">Enter the 6-digit PIN displayed on your primary Hub machine or terminal:</p>
                <div class="pin-input-group">
                  <input type="text" id="input-pairing-pin" class="large-pin-input" maxlength="6" placeholder="000000" value="${state.pairingPinInput}" />
                  <button class="btn-modal-submit btn-claim-pin" id="btn-submit-pin" ${state.isPairingLoading ? 'disabled' : ''}>
                    ${state.isPairingLoading ? 'Pairing...' : 'Verify PIN & Pair'}
                  </button>
                </div>
                <p class="pin-sub-help">Your device will receive a persistent Ed25519 token and will reconnect automatically whenever you launch the app.</p>
              </div>
            ` : `
              <div class="pairing-email-view">
                <p class="pair-guide-text">Invite an office coworker or friend via email OTP without sharing terminal access:</p>
                <div class="email-input-group">
                  <input type="email" id="input-pairing-email" class="modal-text-input" placeholder="colleague@company.com" value="${state.pairingEmailInput}" />
                  <button class="btn-email-action" id="btn-request-email-otp">Send Code</button>
                </div>
                ${state.emailOtpSentMessage ? `
                  <div class="email-otp-verify-box">
                    <span class="otp-sent-banner">${state.emailOtpSentMessage}</span>
                    <input type="text" id="input-pairing-email-otp" class="modal-text-input" maxlength="6" placeholder="Enter 6-digit OTP" value="${state.pairingEmailOtpInput}" style="margin-top:10px;text-align:center;letter-spacing:4px;font-size:18px;font-weight:bold;" />
                    <button class="btn-modal-submit" id="btn-verify-email-otp" style="margin-top:10px;width:100%;">Verify & Authorize Device</button>
                  </div>
                ` : ''}
              </div>
            `}
          </div>
          <div class="modal-footer">
            <button class="btn-modal-cancel" id="btn-close-pairing-footer">Close</button>
          </div>
        </div>
      </div>
    `;
  }

  return html;
}

// EVENT HANDLERS
function attachEventHandlers() {
  // Navigation tabs
  document.querySelectorAll('.nav-item').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = btn.getAttribute('data-tab') as TabType;
      if (tab) {
        state.currentTab = tab;
        if (tab === 'machines') {
          loadMachinesOverview();
          loadInstalledApps();
          loadJobs();
          loadMeshResources();
        } else if (tab === 'files') {
          loadDirectory(state.currentPath);
        } else if (tab === 'mesh') {
          loadMeshStatus();
        }
        render();
      }
    });
  });

  // Mesh pill top click
  const meshPill = document.getElementById('btn-mesh-pill');
  if (meshPill) {
    meshPill.addEventListener('click', () => {
      state.currentTab = 'mesh';
      loadMeshStatus();
      render();
    });
  }

  // Filter pills
  document.querySelectorAll('.filter-pill').forEach(btn => {
    btn.addEventListener('click', () => {
      state.chatFilter = btn.getAttribute('data-filter') as any;
      render();
    });
  });

  // Conversation selection
  document.querySelectorAll('.conv-item').forEach(item => {
    item.addEventListener('click', () => {
      const id = item.getAttribute('data-conv-id');
      if (id) {
        openConversation(id);
      }
    });
  });

  // Story avatars (direct chat)
  document.querySelectorAll('.story-avatar-item').forEach(item => {
    item.addEventListener('click', () => {
      const agentId = item.getAttribute('data-direct-agent');
      if (agentId) {
        startDirectChat(agentId);
      }
    });
  });

  // Mobile back button
  const backBtn = document.getElementById('btn-back-to-roster');
  if (backBtn) {
    backBtn.addEventListener('click', () => {
      closeMobileChat();
    });
  }

  // Open New Group Modal
  const newGroupBtn = document.getElementById('btn-open-new-group');
  if (newGroupBtn) {
    newGroupBtn.addEventListener('click', () => {
      state.showNewGroupModal = true;
      render();
    });
  }

  // Submit New Group
  const submitGroupBtn = document.getElementById('btn-submit-new-group');
  if (submitGroupBtn) {
    submitGroupBtn.addEventListener('click', () => {
      const titleInput = document.getElementById('new-group-title') as HTMLInputElement;
      const title = titleInput?.value || '';
      const cbs = document.querySelectorAll('.agent-cb:checked') as NodeListOf<HTMLInputElement>;
      const pIds: string[] = [];
      cbs.forEach(cb => pIds.push(cb.value));
      createNewGroup(title, pIds);
    });
  }

  // Close Group Modal
  const closeGroupBtn = document.getElementById('btn-close-group-modal');
  const cancelGroupBtn = document.getElementById('btn-cancel-group');
  if (closeGroupBtn) closeGroupBtn.addEventListener('click', () => { state.showNewGroupModal = false; render(); });
  if (cancelGroupBtn) cancelGroupBtn.addEventListener('click', () => { state.showNewGroupModal = false; render(); });

  // Stop Group
  const stopGroupBtn = document.getElementById('btn-stop-group');
  if (stopGroupBtn) {
    stopGroupBtn.addEventListener('click', () => {
      stopActiveConversation();
    });
  }

  // Delete Group
  const deleteGroupBtn = document.getElementById('btn-delete-group');
  if (deleteGroupBtn) {
    deleteGroupBtn.addEventListener('click', () => {
      deleteActiveConversation();
    });
  }

  // Send message
  const sendBtn = document.getElementById('btn-send-message');
  const textInput = document.getElementById('chat-text-input') as HTMLInputElement;
  if (sendBtn && textInput) {
    sendBtn.addEventListener('click', () => {
      sendMessage(textInput.value);
    });
    textInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        sendMessage(textInput.value);
      }
    });
  }

  // Target Agent Selector
  const agentSelect = document.getElementById('agent-dispatch-select') as HTMLSelectElement;
  if (agentSelect) {
    agentSelect.addEventListener('change', () => {
      state.targetSessionId = agentSelect.value;
    });
  }

  // Quick prompt pills
  document.querySelectorAll('.prompt-pill').forEach(btn => {
    btn.addEventListener('click', () => {
      const prompt = btn.getAttribute('data-prompt');
      if (prompt) sendMessage(prompt);
    });
  });

  // Toggle Attachment Menu
  const attToggleBtn = document.getElementById('btn-toggle-attachments');
  if (attToggleBtn) {
    attToggleBtn.addEventListener('click', () => {
      state.showAttachmentMenu = !state.showAttachmentMenu;
      render();
    });
  }

  // Attachment Menu items
  const attachImgBtn = document.getElementById('btn-attach-image');
  if (attachImgBtn) {
    attachImgBtn.addEventListener('click', () => {
      state.pendingAttachment = {
        id: `att-${Date.now()}`,
        name: 'mobile-screen-capture.png',
        mime_type: 'image/png',
        size_bytes: 42000,
        data_base64: 'data:image/svg+xml;utf8,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="300" height="160" viewBox="0 0 300 160"><rect width="300" height="160" rx="12" fill="#101623" stroke="#06B6D4" stroke-width="2"/><circle cx="150" cy="65" r="28" fill="#06B6D4" opacity="0.2"/><text x="150" y="73" text-anchor="middle" fill="#06B6D4" font-size="24">📱</text><text x="150" y="118" text-anchor="middle" fill="#F8FAFC" font-size="13" font-family="sans-serif" font-weight="bold">Mobile Screen Capture</text><text x="150" y="138" text-anchor="middle" fill="#94A3B8" font-size="11" font-family="sans-serif">Verified 1220x2712 Viewport</text></svg>'),
      };
      state.showAttachmentMenu = false;
      render();
    });
  }

  const attachArtBtn = document.getElementById('btn-attach-artifact');
  if (attachArtBtn) {
    attachArtBtn.addEventListener('click', () => {
      state.pendingAttachment = {
        id: `att-${Date.now()}`,
        name: 'agent-config.json',
        mime_type: 'application/json',
        size_bytes: 1024,
        preview_content: '{\n  "version": "0.2.0",\n  "transport": "direct_usb",\n  "active_nodes": 3\n}',
      };
      state.showAttachmentMenu = false;
      render();
    });
  }

  // Remove pending attachment
  const removeAttBtn = document.getElementById('btn-remove-attachment');
  if (removeAttBtn) {
    removeAttBtn.addEventListener('click', () => {
      state.pendingAttachment = null;
      render();
    });
  }

  // Close attachment backdrop
  const attBackdrop = document.getElementById('backdrop-attachment');
  if (attBackdrop) {
    attBackdrop.addEventListener('click', (e) => {
      if (e.target === attBackdrop) {
        state.showAttachmentMenu = false;
        render();
      }
    });
  }

  // Launch App Cards
  document.querySelectorAll('[data-launch-app]').forEach(card => {
    card.addEventListener('click', () => {
      const appId = card.getAttribute('data-launch-app');
      if (appId) launchApp(appId);
    });
  });

  // Terminal Runner
  const runTermBtn = document.getElementById('btn-run-terminal');
  const termInput = document.getElementById('terminal-cmd-input') as HTMLInputElement;
  const sudoCheck = document.getElementById('check-sudo') as HTMLInputElement;

  if (sudoCheck) {
    sudoCheck.addEventListener('change', () => {
      state.isSudoChecked = sudoCheck.checked;
    });
  }

  if (runTermBtn && termInput) {
    const handleExec = () => {
      const cmd = termInput.value.trim();
      if (!cmd) return;
      if (state.isSudoChecked) {
        state.pendingSudoCommand = cmd;
        state.showSudoModal = true;
        render();
      } else {
        executeTerminalCommand(cmd, false);
      }
    };

    runTermBtn.addEventListener('click', handleExec);
    termInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') handleExec();
    });
  }

  // Quick Terminal Commands
  document.querySelectorAll('.quick-cmd-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const cmd = btn.getAttribute('data-quick-cmd');
      if (cmd) {
        if (termInput) termInput.value = cmd;
        executeTerminalCommand(cmd, state.isSudoChecked);
      }
    });
  });

  // Sudo Modal confirm
  const confirmSudoBtn = document.getElementById('btn-confirm-sudo');
  if (confirmSudoBtn) {
    confirmSudoBtn.addEventListener('click', () => {
      const pwdInput = document.getElementById('sudo-password-input') as HTMLInputElement;
      const pwd = pwdInput?.value || '';
      executeTerminalCommand(state.pendingSudoCommand, true, pwd);
    });
  }

  const cancelSudoBtn = document.getElementById('btn-cancel-sudo');
  const closeSudoBtn = document.getElementById('btn-close-sudo-modal');
  if (cancelSudoBtn) cancelSudoBtn.addEventListener('click', () => { state.showSudoModal = false; render(); });
  if (closeSudoBtn) closeSudoBtn.addEventListener('click', () => { state.showSudoModal = false; render(); });

  // Files Refresh
  const refreshFilesBtn = document.getElementById('btn-refresh-files');
  if (refreshFilesBtn) {
    refreshFilesBtn.addEventListener('click', () => {
      loadDirectory(state.currentPath);
    });
  }

  // File item click
  document.querySelectorAll('.file-item-row').forEach(row => {
    row.addEventListener('click', () => {
      const path = row.getAttribute('data-file-path');
      const isDir = row.getAttribute('data-is-dir') === 'true';
      const isEditable = row.getAttribute('data-is-editable') === 'true';
      const name = row.getAttribute('data-file-name') || '';

      if (!path) return;

      if (isDir) {
        loadDirectory(path);
      } else if (isEditable) {
        loadFileContent(path, name);
      } else {
        showToast(`Selected file: ${name}`);
      }
    });
  });

  // Close editor
  const closeEditorBtn = document.getElementById('btn-close-editor');
  if (closeEditorBtn) {
    closeEditorBtn.addEventListener('click', () => {
      state.editingFile = null;
      render();
    });
  }

  // Save editor
  const saveFileBtn = document.getElementById('btn-save-file');
  const editorArea = document.getElementById('file-editor-textarea') as HTMLTextAreaElement;
  if (saveFileBtn && editorArea && state.editingFile) {
    saveFileBtn.addEventListener('click', () => {
      if (state.editingFile) {
        state.editingFile.content = editorArea.value;
        saveFileContent();
      }
    });
  }

  // Job Approval actions
  attachJobActionHandlers();

  // Host URL update
  const saveHostBtn = document.getElementById('btn-save-host');
  const hostInput = document.getElementById('host-url-input') as HTMLInputElement;
  if (saveHostBtn && hostInput) {
    saveHostBtn.addEventListener('click', () => {
      state.hostUrl = hostInput.value.trim();
      localStorage.setItem('rain_host_url', state.hostUrl);
      showToast(`Host updated to: ${state.hostUrl}`);
      loadConversations();
      loadSessions();
      loadMeshStatus();
      loadHubTopology();
    });
  }

  // Pairing Modal Triggers
  const openPairingTop = document.getElementById('btn-open-pairing');
  const openPairingMesh = document.getElementById('btn-mesh-add-spoke');
  const handleOpenPairing = () => {
    state.showPairingModal = true;
    requestPairingTicket();
  };
  if (openPairingTop) openPairingTop.addEventListener('click', handleOpenPairing);
  if (openPairingMesh) openPairingMesh.addEventListener('click', handleOpenPairing);

  const closePairingBtn = document.getElementById('btn-close-pairing-modal');
  const closePairingFooter = document.getElementById('btn-close-pairing-footer');
  if (closePairingBtn) closePairingBtn.addEventListener('click', () => { state.showPairingModal = false; render(); });
  if (closePairingFooter) closePairingFooter.addEventListener('click', () => { state.showPairingModal = false; render(); });

  // Pairing Tab Switch
  document.querySelectorAll('.pair-tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = btn.getAttribute('data-pair-tab') as 'host' | 'pin' | 'email';
      if (tab) {
        state.pairingTab = tab;
        render();
      }
    });
  });

  // Copy Code & Link
  const copyPinBtn = document.getElementById('btn-copy-pairing-code');
  if (copyPinBtn) {
    copyPinBtn.addEventListener('click', () => {
      const pin = state.pairingTicket?.pairing_code || '849201';
      navigator.clipboard?.writeText(pin);
      showToast(`📋 Copied PIN: ${pin}`);
    });
  }

  const copyLinkBtn = document.getElementById('btn-copy-pairing-link');
  if (copyLinkBtn) {
    copyLinkBtn.addEventListener('click', () => {
      const link = state.pairingTicket?.deep_link || '';
      navigator.clipboard?.writeText(link);
      showToast('🔗 Copied pairing deep link!');
    });
  }

  // PIN Claim inputs
  const pinInput = document.getElementById('input-pairing-pin') as HTMLInputElement;
  if (pinInput) {
    pinInput.addEventListener('input', () => {
      state.pairingPinInput = pinInput.value;
    });
  }

  const submitPinBtn = document.getElementById('btn-submit-pin');
  if (submitPinBtn) {
    submitPinBtn.addEventListener('click', () => {
      submitPinClaim();
    });
  }

  // Email OTP inputs
  const emailInput = document.getElementById('input-pairing-email') as HTMLInputElement;
  if (emailInput) {
    emailInput.addEventListener('input', () => {
      state.pairingEmailInput = emailInput.value;
    });
  }

  const reqOtpBtn = document.getElementById('btn-request-email-otp');
  if (reqOtpBtn) {
    reqOtpBtn.addEventListener('click', () => {
      requestEmailOtp();
    });
  }

  const emailOtpInput = document.getElementById('input-pairing-email-otp') as HTMLInputElement;
  if (emailOtpInput) {
    emailOtpInput.addEventListener('input', () => {
      state.pairingEmailOtpInput = emailOtpInput.value;
    });
  }

  const verifyOtpBtn = document.getElementById('btn-verify-email-otp');
  if (verifyOtpBtn) {
    verifyOtpBtn.addEventListener('click', () => {
      submitEmailOtpVerify();
    });
  }

  // Revoke Spoke Device
  document.querySelectorAll('[data-revoke-device]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const devId = btn.getAttribute('data-revoke-device');
      if (devId) revokeSpokeDevice(devId);
    });
  });

  // Battery Evacuation Button in Machines View
  const evacBtn = document.getElementById('btn-evacuate-now-click');
  if (evacBtn) {
    evacBtn.addEventListener('click', () => {
      triggerBatteryFailover();
    });
  }
}

// Initialization
async function init() {
  await Promise.allSettled([
    loadConversations(),
    loadSessions(),
    loadMachinesOverview(false),
    loadInstalledApps(),
    loadMeshStatus(),
    loadJobs(),
    loadMeshResources(),
    loadHubTopology(),
  ]);

  if (state.conversations.length > 0) {
    state.activeConversationId = state.conversations[0].id;
    await loadMessages(state.conversations[0].id, false);
  } else {
    state.activeConversationId = null;
    state.messages = [];
  }

  render();
  startPolling();
}

window.addEventListener('DOMContentLoaded', init);

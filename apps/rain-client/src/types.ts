// RAIN — Remote Agent Infrastructure Network Domain Types

export interface ChatAttachment {
  id: string;
  name: string;
  mime_type: string;
  size_bytes: number;
  preview_content?: string;
  data_base64?: string;
}

export interface JobDelegation {
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

export interface ChatMessage {
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

export interface Conversation {
  id: string;
  title: string;
  is_group: boolean;
  is_active: boolean;
  participant_ids: string[];
  last_message: string | null;
  last_message_time: string | null;
  unread_count: number;
}

export interface AgentSession {
  id: string;
  name: string;
  provider: string;
  machine_name: string;
  state: 'online' | 'thinking' | 'working' | 'responding' | 'waiting' | 'completed' | 'stopped' | 'disconnected' | 'error';
  cwd: string;
  subagents: string[];
}

export interface BatteryStatus {
  percent: number;
  is_charging: boolean;
  is_ac_connected: boolean;
  power_source: string;
  time_remaining_minutes?: number;
  is_critical: boolean;
}

export interface HubSpokeNode {
  device_id: string;
  device_name: string;
  platform: string;
  is_online: boolean;
  enrolled_at: string;
  last_seen_at: string;
  ip_address?: string;
  role: string;
}

export interface HubTopologyStatus {
  hub_name: string;
  hub_node_id: string;
  hub_listen_addr: string;
  is_hub_online: boolean;
  uptime_seconds: number;
  active_spokes_count: number;
  spokes: HubSpokeNode[];
}

export interface PairingTicket {
  pairing_code: string;
  deep_link: string;
  qr_svg: string;
  host_name: string;
  host_listen_addr: string;
  expires_in_seconds: number;
}

export interface MachineResourceCatalog {
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

export interface MachineOverview {
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

export interface InstalledApp {
  id: string;
  name: string;
  executable: string;
  icon_name: string;
  category: string;
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size_bytes: number;
  modified_time: string | null;
  is_editable: boolean;
}

export interface NetworkMeshStatus {
  current_transport: 'direct_usb' | 'lan_mdns' | 'wireguard_p2p' | 'encrypted_relay';
  node_id: string;
  latency_ms: number;
  bytes_sent: number;
  bytes_received: number;
  is_e2ee: boolean;
  available_transports: string[];
}

export type TabType = 'chats' | 'machines' | 'files' | 'mesh';
export type ThemeMode = 'dark' | 'light';

import React, { useState } from 'react';
import {
  FiServer,
  FiCpu,
  FiHardDrive,
  FiZap,
  FiAlertTriangle,
  FiTerminal,
  FiPlay,
  FiLock,
  FiActivity,
  FiMonitor,
  FiSmartphone
} from 'react-icons/fi';
import { MachineOverview, InstalledApp } from '../types';
import { formatBytes, formatUptime } from '../api';

interface TerminalLog {
  cmd: string;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
  is_sudo: boolean;
}

interface MachinesViewProps {
  machines: MachineOverview[];
  installedApps: InstalledApp[];
  onLaunchApp: (appId: string) => Promise<void>;
  onExecuteTerminal: (cmd: string, isSudo: boolean, password?: string) => Promise<any>;
  onTriggerBatteryEvacuate: () => Promise<void>;
}

export const MachinesView: React.FC<MachinesViewProps> = ({
  machines,
  installedApps,
  onLaunchApp,
  onExecuteTerminal,
  onTriggerBatteryEvacuate,
}) => {
  const [termCmd, setTermCmd] = useState('');
  const [isSudo, setIsSudo] = useState(false);
  const [sudoPassword, setSudoPassword] = useState('');
  const [showPasswordModal, setShowPasswordModal] = useState(false);
  const [logs, setLogs] = useState<TerminalLog[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  const handleRunCommand = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const cmd = termCmd.trim();
    if (!cmd || isRunning) return;

    if (isSudo && !sudoPassword) {
      setShowPasswordModal(true);
      return;
    }

    setIsRunning(true);
    try {
      const res = await onExecuteTerminal(cmd, isSudo, sudoPassword || undefined);
      setLogs(prev => [
        {
          cmd,
          stdout: res.stdout || '',
          stderr: res.stderr || '',
          exit_code: res.exit_code,
          duration_ms: res.duration_ms || 0,
          is_sudo: isSudo,
        },
        ...prev,
      ]);
      setTermCmd('');
      setShowPasswordModal(false);
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="machines-screen">
      {/* 1. Remote Machines Section */}
      <div className="section-header-row">
        <h2 className="section-title">Remote Machines & Workstations</h2>
        <span style={{ fontSize: 12, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
          {machines.length} active node{machines.length === 1 ? '' : 's'}
        </span>
      </div>

      <div className="machines-grid">
        {machines.map(m => {
          const memPercent = Math.round((m.memory_used_bytes / (m.memory_total_bytes || 1)) * 100);
          const diskPercent = Math.round((m.disk_used_bytes / (m.disk_total_bytes || 1)) * 100);
          const isCritical = m.battery?.is_critical;
          const isCharging = m.battery?.is_charging || m.battery?.is_ac_connected;

          return (
            <div key={m.id} className="machine-card">
              <div className="machine-card-header">
                <div className="machine-identity-wrap">
                  <div className="machine-icon-box">
                    {m.platform === 'android' ? (
                      <FiSmartphone size={16} />
                    ) : m.platform === 'linux' ? (
                      <FiServer size={16} />
                    ) : (
                      <FiMonitor size={16} />
                    )}
                  </div>
                  <div>
                    <div className="machine-name-text">{m.name}</div>
                    <div className="machine-ip-text">{m.ip_address}</div>
                  </div>
                </div>

                <div className="telemetry-pill">
                  <span className={`status-dot ${m.is_online ? 'online' : 'offline'}`} />
                  <span style={{ textTransform: 'uppercase', fontSize: 10 }}>
                    {m.is_online ? 'Online' : 'Dormant'}
                  </span>
                </div>
              </div>

              {/* Hardware Power & Battery Telemetry */}
              <div
                className={`power-telemetry-banner ${
                  isCritical ? 'critical' : isCharging ? 'charging' : ''
                }`}
              >
                <div className="power-left">
                  {isCritical ? (
                    <FiAlertTriangle size={15} />
                  ) : (
                    <FiZap size={15} />
                  )}
                  <div>
                    <span>
                      {m.battery
                        ? `${m.battery.percent}% (${m.battery.power_source})`
                        : 'AC Wall Power'}
                    </span>
                    {isCritical && (
                      <span style={{ marginLeft: 8, fontSize: 11, opacity: 0.9 }}>
                        ⏱ ~{m.battery?.time_remaining_minutes || 8}m remaining
                      </span>
                    )}
                  </div>
                </div>

                {isCritical && (
                  <button
                    className="icon-button"
                    style={{
                      backgroundColor: 'var(--danger-btn)',
                      color: '#ffffff',
                      borderColor: 'var(--danger-btn)'
                    }}
                    onClick={onTriggerBatteryEvacuate}
                  >
                    <span>Evacuate</span>
                  </button>
                )}
              </div>

              {/* Metrics Grid */}
              <div className="metrics-block">
                <div className="metric-bar-row">
                  <div className="metric-meta">
                    <span>CPU LOAD</span>
                    <span>{m.cpu_usage.toFixed(1)}%</span>
                  </div>
                  <div className="progress-track">
                    <div
                      className={`progress-fill ${m.cpu_usage > 85 ? 'danger' : ''}`}
                      style={{ width: `${Math.min(100, m.cpu_usage)}%` }}
                    />
                  </div>
                </div>

                <div className="metric-bar-row">
                  <div className="metric-meta">
                    <span>RAM USAGE</span>
                    <span>
                      {formatBytes(m.memory_used_bytes)} / {formatBytes(m.memory_total_bytes)} ({memPercent}%)
                    </span>
                  </div>
                  <div className="progress-track">
                    <div
                      className={`progress-fill ${memPercent > 90 ? 'danger' : ''}`}
                      style={{ width: `${Math.min(100, memPercent)}%` }}
                    />
                  </div>
                </div>

                <div className="metric-bar-row">
                  <div className="metric-meta">
                    <span>DISK STORAGE</span>
                    <span>
                      {formatBytes(m.disk_used_bytes)} / {formatBytes(m.disk_total_bytes)}
                    </span>
                  </div>
                  <div className="progress-track">
                    <div
                      className="progress-fill"
                      style={{ width: `${Math.min(100, diskPercent)}%` }}
                    />
                  </div>
                </div>
              </div>

              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  fontSize: 11,
                  color: 'var(--text-muted)',
                  fontFamily: 'var(--font-mono)',
                  paddingTop: 8,
                  borderTop: '1px solid var(--border-subtle)'
                }}
              >
                <span>Uptime: {formatUptime(m.uptime_seconds)}</span>
                <span>{m.active_agents_count} agents</span>
              </div>
            </div>
          );
        })}
      </div>

      {/* 2. Installed Apps Launcher */}
      <div className="section-header-row" style={{ marginTop: 8 }}>
        <h2 className="section-title">Host Applications</h2>
        <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>
          Launch workstation tools directly
        </span>
      </div>

      <div className="app-shelf-grid">
        {installedApps.map(app => (
          <div
            key={app.id}
            className="app-shelf-item"
            onClick={() => onLaunchApp(app.id)}
            title={`Launch ${app.name} on host machine`}
          >
            <div style={{ color: 'var(--accent-primary)', display: 'flex' }}>
              <FiPlay size={14} />
            </div>
            <div style={{ display: 'flex', flexDirection: 'column' }}>
              <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-primary)' }}>
                {app.name}
              </span>
              <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>{app.category}</span>
            </div>
          </div>
        ))}
      </div>

      {/* 3. Remote Terminal Runner */}
      <div className="section-header-row" style={{ marginTop: 8 }}>
        <h2 className="section-title">Host Terminal Runner</h2>
      </div>

      <div className="terminal-box">
        <form onSubmit={handleRunCommand} style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 12, color: 'var(--text-muted)' }}>
            <FiTerminal size={14} />
            <label style={{ display: 'flex', alignItems: 'center', gap: 4, cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={isSudo}
                onChange={e => setIsSudo(e.target.checked)}
              />
              <span>sudo</span>
            </label>
          </div>

          <input
            type="text"
            className="chat-input-field"
            placeholder="Enter shell command (e.g. uname -a, git status, df -h)..."
            value={termCmd}
            onChange={e => setTermCmd(e.target.value)}
            disabled={isRunning}
            style={{ fontFamily: 'var(--font-mono)', fontSize: 12 }}
          />

          <button
            type="submit"
            className="icon-button primary"
            disabled={!termCmd.trim() || isRunning}
          >
            <FiPlay size={13} />
            <span>{isRunning ? 'Running...' : 'Execute'}</span>
          </button>
        </form>

        {logs.length > 0 && (
          <div className="terminal-output-feed">
            {logs.map((log, idx) => (
              <div key={idx} style={{ borderBottom: '1px solid var(--border-subtle)', paddingBottom: 6 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)' }}>
                  <span>
                    $ {log.is_sudo ? 'sudo ' : ''}
                    {log.cmd}
                  </span>
                  <span style={{ color: log.exit_code === 0 ? 'var(--success-text)' : 'var(--danger-text)' }}>
                    exit {log.exit_code} ({log.duration_ms}ms)
                  </span>
                </div>
                {log.stdout && (
                  <pre style={{ color: 'var(--text-primary)', whiteSpace: 'pre-wrap', marginTop: 4 }}>
                    {log.stdout}
                  </pre>
                )}
                {log.stderr && (
                  <pre style={{ color: 'var(--danger-text)', whiteSpace: 'pre-wrap', marginTop: 4 }}>
                    {log.stderr}
                  </pre>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Sudo Password Modal */}
      {showPasswordModal && (
        <div className="modal-overlay">
          <div className="modal-card">
            <div className="modal-header">
              <span className="modal-title">Elevated Sudo Authorization</span>
              <button className="icon-button" onClick={() => setShowPasswordModal(false)}>
                ✕
              </button>
            </div>
            <div className="modal-body">
              <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                Command <code>{termCmd}</code> requires root privileges on the host workstation.
              </p>
              <input
                type="password"
                className="chat-input-field"
                placeholder="Enter host sudo password..."
                value={sudoPassword}
                onChange={e => setSudoPassword(e.target.value)}
                autoFocus
              />
            </div>
            <div className="modal-footer">
              <button className="icon-button" onClick={() => setShowPasswordModal(false)}>
                Cancel
              </button>
              <button className="icon-button primary" onClick={() => handleRunCommand()}>
                Authorize & Run
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

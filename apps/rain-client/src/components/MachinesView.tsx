import React, { useState } from 'react';
import { FiServer, FiCpu, FiHardDrive, FiZap, FiAlertTriangle, FiTerminal, FiPlay, FiLock, FiActivity, FiMonitor, FiSmartphone } from 'react-icons/fi';
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
export const MachinesView: React.FC<MachinesViewProps> = ({ machines, installedApps, onLaunchApp, onExecuteTerminal, onTriggerBatteryEvacuate, }) => {
    const [termCmd, setTermCmd] = useState('');
    const [isSudo, setIsSudo] = useState(false);
    const [sudoPassword, setSudoPassword] = useState('');
    const [showPasswordModal, setShowPasswordModal] = useState(false);
    const [logs, setLogs] = useState<TerminalLog[]>([]);
    const [isRunning, setIsRunning] = useState(false);
    const handleRunCommand = async (e?: React.FormEvent) => {
        if (e)
            e.preventDefault();
        const cmd = termCmd.trim();
        if (!cmd || isRunning)
            return;
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
        }
        finally {
            setIsRunning(false);
        }
    };
    return (<div className="machines-screen space-y-6 py-4">
      {/* 1. Remote Machines Section */}
      <div className="section-header-row mb-4 flex flex-wrap items-center justify-between gap-4">
        <h2 className="section-title text-lg font-medium tracking-tight">Remote Machines & Workstations</h2>
        <span className="text-xs [color:var(--text-muted)] font-mono">
          {machines.length} active node{machines.length === 1 ? '' : 's'}
        </span>
      </div>

      <div className="machines-grid grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {machines.map(m => {
            const memPercent = Math.round((m.memory_used_bytes / (m.memory_total_bytes || 1)) * 100);
            const diskPercent = Math.round((m.disk_used_bytes / (m.disk_total_bytes || 1)) * 100);
            const isCritical = m.battery?.is_critical;
            const isCharging = m.battery?.is_charging || m.battery?.is_ac_connected;
            return (<div key={m.id} className="machine-card glass-panel min-w-0 rounded-xl p-5">
              <div className="machine-card-header mb-5 flex items-center justify-between gap-3">
                <div className="machine-identity-wrap min-w-0 flex-1">
                  <div className="machine-icon-box flex size-10 shrink-0 items-center justify-center rounded-lg bg-canvas-elevated text-ink-secondary">
                    {m.platform === 'android' ? (<FiSmartphone size={16}/>) : m.platform === 'linux' ? (<FiServer size={16}/>) : (<FiMonitor size={16}/>)}
                  </div>
                  <div>
                    <div className="machine-name-text truncate text-sm font-medium">{m.name}</div>
                    <div className="machine-ip-text mt-1 truncate font-mono text-xs text-ink-muted">{m.ip_address}</div>
                  </div>
                </div>

                <div className="telemetry-pill rounded border border-border-subtle px-2 py-1 font-mono text-[10px]">
                  <span className={`status-dot ${m.is_online ? "online" : "offline"}`}/>
                  <span style={{
                    textTransform: 'uppercase'
                }} className="text-xs">
                    {m.is_online ? 'Online' : 'Dormant'}
                  </span>
                </div>
              </div>

              {/* Hardware Power & Battery Telemetry */}
              <div className={`power-telemetry-banner mt-4 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-status-warning/30 bg-bubble-sudo p-3 text-xs ${isCritical ? "critical" : isCharging ? "charging" : ""}`}>
                <div className="power-left flex items-center gap-2">
                  {isCritical ? (<FiAlertTriangle size={15}/>) : (<FiZap size={15}/>)}
                  <div>
                    <span>
                      {m.battery
                    ? `${m.battery.percent}% (${m.battery.power_source})`
                    : 'AC Wall Power'}
                    </span>
                    {isCritical && (<span style={{
                        marginLeft: 8, opacity: 0.9
                    }} className="text-xs">
                        ⏱ ~{m.battery?.time_remaining_minutes == null ? 'Not reported' : `${m.battery.time_remaining_minutes}m remaining`}
                      </span>)}
                  </div>
                </div>

                {isCritical && (<button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger [background-color:var(--danger-btn)] [color:#ffffff]" style={{
                        borderColor: 'var(--danger-btn)'
                    }} onClick={onTriggerBatteryEvacuate}>
                    <span>Evacuate</span>
                  </button>)}
              </div>

              {/* Metrics Grid */}
              <div className="metrics-block space-y-4">
                <div className="metric-bar-row space-y-2">
                  <div className="metric-meta flex items-center justify-between gap-3 font-mono text-xs">
                    <span>CPU LOAD</span>
                    <span>{m.cpu_usage.toFixed(1)}%</span>
                  </div>
                  <div className="progress-track h-1.5 overflow-hidden rounded-full bg-canvas-hover">
                    <div className={`progress-fill h-full rounded-full transition-[width] duration-300 motion-reduce:transition-none ${m.cpu_usage > 80 ? "bg-status-warning" : m.cpu_usage >= 50 ? "bg-status-working" : "bg-status-online"}`} style={{
                    width: `${Math.min(100, m.cpu_usage)}%`
                }}/>
                  </div>
                </div>

                <div className="metric-bar-row space-y-2">
                  <div className="metric-meta flex items-center justify-between gap-3 font-mono text-xs">
                    <span>RAM USAGE</span>
                    <span>
                      {formatBytes(m.memory_used_bytes)} / {formatBytes(m.memory_total_bytes)} ({memPercent}%)
                    </span>
                  </div>
                  <div className="progress-track h-1.5 overflow-hidden rounded-full bg-canvas-hover">
                    <div className={`progress-fill h-full rounded-full transition-[width] duration-300 motion-reduce:transition-none ${memPercent > 90 ? "bg-status-warning" : "bg-status-online"}`} style={{
                    width: `${Math.min(100, memPercent)}%`
                }}/>
                  </div>
                </div>

                <div className="metric-bar-row space-y-2">
                  <div className="metric-meta flex items-center justify-between gap-3 font-mono text-xs">
                    <span>DISK STORAGE</span>
                    <span>
                      {formatBytes(m.disk_used_bytes)} / {formatBytes(m.disk_total_bytes)}
                    </span>
                  </div>
                  <div className="progress-track h-1.5 overflow-hidden rounded-full bg-canvas-hover">
                    <div className="progress-fill h-full rounded-full bg-status-online transition-[width] duration-300" style={{
                    width: `${Math.min(100, diskPercent)}%`
                }}/>
                  </div>
                </div>
              </div>

              <div className="[display:flex] [justify-content:space-between] text-xs [color:var(--text-muted)] font-mono [padding-top:8px] [border-top:1px_solid_var(--border-subtle)]">
                <span>Uptime: {formatUptime(m.uptime_seconds)}</span>
                <span>{m.active_agents_count} agents</span>
              </div>
            </div>);
        })}
      </div>

      {/* 2. Installed Apps Launcher */}
      <div className="section-header-row mb-4 flex flex-wrap items-center justify-between gap-4 [margin-top:8px]">
        <h2 className="section-title text-lg font-medium tracking-tight">Host Applications</h2>
        <span className="text-xs [color:var(--text-muted)]">
          Launch workstation tools directly
        </span>
      </div>

      <div className="app-shelf-grid grid grid-cols-2 gap-3 md:grid-cols-4">
        {installedApps.map(app => (<div key={app.id} className="app-shelf-item glass-panel flex min-w-0 flex-col items-start gap-3 rounded-xl p-4 text-left" onClick={() => onLaunchApp(app.id)} title={`Launch ${app.name} on host machine`}>
            <div className="[color:var(--accent-primary)] [display:flex]">
              <FiPlay size={14}/>
            </div>
            <div className="[display:flex] [flex-direction:column]">
              <span className="text-xs [font-weight:600] [color:var(--text-primary)]">
                {app.name}
              </span>
              <span className="text-xs [color:var(--text-muted)]">{app.category}</span>
            </div>
          </div>))}
      </div>

      {/* 3. Remote Terminal Runner */}
      <div className="section-header-row mb-4 flex flex-wrap items-center justify-between gap-4 [margin-top:8px]">
        <h2 className="section-title text-lg font-medium tracking-tight">Host Terminal Runner</h2>
      </div>

      <div className="terminal-box overflow-hidden rounded-xl border border-border-subtle bg-canvas-base">
        <form onSubmit={handleRunCommand} className="[display:flex] [gap:8px] [align-items:center]">
          <div className="[display:flex] [align-items:center] [gap:6px] text-xs [color:var(--text-muted)]">
            <FiTerminal size={14}/>
            <label style={{
            cursor: 'pointer'
        }} className="[display:flex] [align-items:center] [gap:4px]">
              <input type="checkbox" checked={isSudo} onChange={e => setIsSudo(e.target.checked)}/>
              <span>sudo</span>
            </label>
          </div>

          <input type="text" className="chat-input-field min-h-11 w-full min-w-0 resize-none border-0! bg-transparent! px-3 py-2 text-base text-ink placeholder:text-ink-muted font-mono text-xs" placeholder="Enter shell command (e.g. uname -a, git status, df -h)..." value={termCmd} onChange={e => setTermCmd(e.target.value)} disabled={isRunning}/>

          <button type="submit" className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" disabled={!termCmd.trim() || isRunning}>
            <FiPlay size={13}/>
            <span>{isRunning ? 'Running...' : 'Execute'}</span>
          </button>
        </form>

        {logs.length > 0 && (<div className="terminal-output-feed max-h-96 overflow-auto p-4 font-mono text-xs">
            {logs.map((log, idx) => (<div key={idx} style={{
                    borderBottom: '1px solid var(--border-subtle)'
                }} className="[padding-bottom:6px]">
                <div className="[display:flex] [justify-content:space-between] [color:var(--text-secondary)]">
                  <span>
                    $ {log.is_sudo ? 'sudo ' : ''}
                    {log.cmd}
                  </span>
                  <span style={{
                    color: log.exit_code === 0 ? 'var(--success-text)' : 'var(--danger-text)'
                }}>
                    exit {log.exit_code} ({log.duration_ms}ms)
                  </span>
                </div>
                {log.stdout && (<pre className="[color:var(--text-primary)] [white-space:pre-wrap] [margin-top:4px]">
                    {log.stdout}
                  </pre>)}
                {log.stderr && (<pre className="[color:var(--danger-text)] [white-space:pre-wrap] [margin-top:4px]">
                    {log.stderr}
                  </pre>)}
              </div>))}
          </div>)}
      </div>

      {/* Sudo Password Modal */}
      {showPasswordModal && (<div className="modal-overlay fixed inset-0 z-50 flex items-center justify-center overflow-y-auto bg-black/60 p-4 backdrop-blur-md">
          <div className="modal-card w-full max-w-lg overflow-hidden rounded-2xl border border-border-subtle bg-canvas-surface/95 shadow-2xl backdrop-blur-xl">
            <div className="modal-header flex min-h-16 items-center justify-between gap-4 border-b border-border-chisel bg-canvas-elevated/50 px-5 py-3">
              <span className="modal-title text-base font-medium">Elevated Sudo Authorization</span>
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => setShowPasswordModal(false)}>
                ✕
              </button>
            </div>
            <div className="modal-body max-h-[70dvh] space-y-4 overflow-y-auto p-5">
              <p className="text-sm [color:var(--text-secondary)]">
                Command <code>{termCmd}</code> requires root privileges on the host workstation.
              </p>
              <input type="password" className="chat-input-field min-h-11 w-full min-w-0 resize-none border-0! bg-transparent! px-3 py-2 text-base text-ink placeholder:text-ink-muted" placeholder="Enter host sudo password..." value={sudoPassword} onChange={e => setSudoPassword(e.target.value)} autoFocus/>
            </div>
            <div className="modal-footer flex flex-wrap justify-end gap-3 border-t border-border-chisel px-5 py-4">
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger" onClick={() => setShowPasswordModal(false)}>
                CANCEL
              </button>
              <button className="icon-button inline-flex min-h-11 min-w-11 shrink-0 items-center justify-center gap-2 rounded-lg border border-border-subtle bg-canvas-surface/70 px-3 py-2 text-xs font-medium text-ink transition-colors hover:border-border-glow hover:bg-canvas-hover disabled:cursor-not-allowed disabled:opacity-45 [&.primary]:border-status-online/40 [&.primary]:bg-status-online/15 [&.primary]:text-ink [&.danger]:border-status-danger/40 [&.danger]:text-status-danger primary border-status-online/40 bg-status-online/15 text-ink hover:bg-status-online/25" onClick={() => handleRunCommand()}>
                Authorize & Run
              </button>
            </div>
          </div>
        </div>)}
    </div>);
};

import React, { useState } from 'react';
import {
  FiServer,
  FiLink,
  FiSmartphone,
  FiMonitor,
  FiTrash2,
  FiActivity,
  FiShield,
  FiSave
} from 'react-icons/fi';
import { HubTopologyStatus, NetworkMeshStatus } from '../types';
import { formatUptime, getHostUrl, setHostUrl } from '../api';

interface MeshViewProps {
  topology: HubTopologyStatus | null;
  meshStatus: NetworkMeshStatus | null;
  latencyMs: number;
  onOpenPairing: () => void;
  onRevokeDevice: (deviceId: string) => Promise<void>;
  onRefreshTopology: () => Promise<void>;
  onHostUrlChanged: () => void;
}

export const MeshView: React.FC<MeshViewProps> = ({
  topology,
  meshStatus,
  latencyMs,
  onOpenPairing,
  onRevokeDevice,
  onRefreshTopology,
  onHostUrlChanged,
}) => {
  const [hostInput, setHostInput] = useState(getHostUrl());
  const [savedNotice, setSavedNotice] = useState(false);

  const handleSaveHost = () => {
    setHostUrl(hostInput);
    setSavedNotice(true);
    onHostUrlChanged();
    setTimeout(() => setSavedNotice(false), 2500);
  };

  return (
    <div className="mesh-screen">
      {/* 1. Network Health & Topology Panel */}
      <div className="section-header-row">
        <h2 className="section-title">Star Topology Coordinator</h2>
        <button className="icon-button primary" onClick={onOpenPairing}>
          <FiLink size={13} />
          <span>+ Pair Worker Node</span>
        </button>
      </div>

      <div className="topology-panel">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <div style={{ fontSize: 15, fontWeight: 700, color: 'var(--text-primary)' }}>
              {topology?.hub_name || 'Central Coordinator Hub'}
            </div>
            <div style={{ fontSize: 12, color: 'var(--text-muted)' }}>
              Central orchestrator coordinating live peer spoke nodes
            </div>
          </div>

          <div className="telemetry-pill">
            <span className="status-dot online" />
            <span>ONLINE</span>
          </div>
        </div>

        <div className="kv-grid">
          <div className="kv-item">
            <div className="kv-label">Listen Address</div>
            <div className="kv-val">{topology?.hub_listen_addr || '0.0.0.0:7890'}</div>
          </div>

          <div className="kv-item">
            <div className="kv-label">Cryptographic Node ID</div>
            <div className="kv-val">{topology?.hub_node_id || 'ed25519:7a4f91b8'}</div>
          </div>

          <div className="kv-item">
            <div className="kv-label">Round-Trip Latency</div>
            <div className="kv-val">{latencyMs} ms</div>
          </div>

          <div className="kv-item">
            <div className="kv-label">Coordinator Uptime</div>
            <div className="kv-val">{formatUptime(topology?.uptime_seconds || 0)}</div>
          </div>
        </div>
      </div>

      {/* 2. Connected Spoke Devices */}
      <div className="section-header-row" style={{ marginTop: 8 }}>
        <h2 className="section-title">
          Connected Spoke Devices ({topology?.spokes?.length || 0})
        </h2>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {topology?.spokes && topology.spokes.length > 0 ? (
          topology.spokes.map(s => (
            <div
              key={s.device_id}
              style={{
                backgroundColor: 'var(--bg-surface)',
                border: '1px solid var(--border-subtle)',
                borderRadius: 'var(--radius-sm)',
                padding: '12px 16px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between'
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <div
                  style={{
                    width: 32,
                    height: 32,
                    borderRadius: 'var(--radius-sm)',
                    backgroundColor: 'var(--bg-subtle)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'var(--text-secondary)'
                  }}
                >
                  {s.platform === 'android' ? (
                    <FiSmartphone size={16} />
                  ) : (
                    <FiMonitor size={16} />
                  )}
                </div>

                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
                      {s.device_name}
                    </span>
                    <span
                      style={{
                        fontSize: 10,
                        fontWeight: 600,
                        textTransform: 'uppercase',
                        padding: '1px 5px',
                        borderRadius: 3,
                        backgroundColor: 'var(--bg-subtle)',
                        border: '1px solid var(--border-subtle)',
                        color: 'var(--text-muted)'
                      }}
                    >
                      {s.role}
                    </span>
                  </div>
                  <span style={{ fontSize: 11, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                    {s.ip_address || 'Connected node'}
                  </span>
                </div>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <span className={`status-dot ${s.is_online ? 'online' : 'offline'}`} />
                {s.role !== 'admin' && (
                  <button
                    className="icon-button"
                    onClick={() => onRevokeDevice(s.device_id)}
                    title="Revoke device access"
                  >
                    <FiTrash2 size={13} />
                    <span>Revoke</span>
                  </button>
                )}
              </div>
            </div>
          ))
        ) : (
          <div
            style={{
              padding: 20,
              textAlign: 'center',
              color: 'var(--text-muted)',
              fontSize: 13,
              backgroundColor: 'var(--bg-surface)',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--border-subtle)'
            }}
          >
            No spoke worker nodes enrolled yet. Tap "+ Pair Worker Node" to connect another machine.
          </div>
        )}
      </div>

      {/* 3. Host Connection Config */}
      <div className="section-header-row" style={{ marginTop: 8 }}>
        <h2 className="section-title">Network Endpoint Settings</h2>
      </div>

      <div
        style={{
          backgroundColor: 'var(--bg-surface)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-md)',
          padding: 16,
          display: 'flex',
          flexDirection: 'column',
          gap: 12
        }}
      >
        <p style={{ fontSize: 12.5, color: 'var(--text-secondary)' }}>
          To connect over Wi-Fi LAN, enter your primary PC's network address (e.g.{' '}
          <code>http://192.168.0.6:7890</code>).
        </p>

        <div style={{ display: 'flex', gap: 8 }}>
          <input
            type="text"
            className="chat-input-field"
            value={hostInput}
            onChange={e => setHostInput(e.target.value)}
            placeholder="http://192.168.0.6:7890"
            style={{ fontFamily: 'var(--font-mono)', fontSize: 12 }}
          />

          <button className="icon-button primary" onClick={handleSaveHost}>
            <FiSave size={13} />
            <span>Save Host</span>
          </button>
        </div>

        {savedNotice && (
          <span style={{ fontSize: 12, color: 'var(--success-text)' }}>
            ✓ Host endpoint updated to {hostInput}
          </span>
        )}
      </div>
    </div>
  );
};

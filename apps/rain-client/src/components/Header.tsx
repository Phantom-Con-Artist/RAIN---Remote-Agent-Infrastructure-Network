import React from 'react';
import { FiSun, FiMoon, FiLink, FiCloud } from 'react-icons/fi';
import { ThemeMode, NetworkMeshStatus } from '../types';

interface HeaderProps {
  theme: ThemeMode;
  onToggleTheme: () => void;
  connectionStatus: 'online' | 'offline' | 'connecting';
  latencyMs: number;
  meshStatus: NetworkMeshStatus | null;
  onOpenPairing: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  theme,
  onToggleTheme,
  connectionStatus,
  latencyMs,
  meshStatus,
  onOpenPairing,
}) => {
  const transportName = meshStatus?.current_transport
    ? meshStatus.current_transport.replace('_', ' ')
    : 'Local LAN / USB';

  return (
    <header className="app-header">
      <div className="app-brand">
        <div className="brand-icon">
          <FiCloud size={16} />
        </div>
        <div className="brand-text-col">
          <span className="brand-title">RAIN</span>
          <span className="brand-sub">Remote Agent Infrastructure Network</span>
        </div>
      </div>

      <div className="header-actions">
        <button
          className="icon-button"
          onClick={onOpenPairing}
          title="Pair a new machine or phone"
        >
          <FiLink size={14} />
          <span>Pair Node</span>
        </button>

        <div className="telemetry-pill" title={`Transport: ${transportName}`}>
          <span className={`status-dot ${connectionStatus}`} />
          <span>{connectionStatus === 'online' ? `${latencyMs} ms` : 'Offline'}</span>
          <span className="transport-tag">{transportName}</span>
        </div>

        <button
          className="icon-button"
          onClick={onToggleTheme}
          title={`Switch to ${theme === 'dark' ? 'Light' : 'Dark'} theme`}
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? <FiSun size={15} /> : <FiMoon size={15} />}
        </button>
      </div>
    </header>
  );
};

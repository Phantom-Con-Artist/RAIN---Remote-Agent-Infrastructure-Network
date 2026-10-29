import {useDialogFocus} from './useDialogFocus';
import React, { useState } from 'react';
import { FiCopy, FiCheck, FiX, FiShield, FiLink } from 'react-icons/fi';
import { PairingTicket } from '../types';

interface PairingModalProps {
  isOpen: boolean;
  onClose: () => void;
  ticket: PairingTicket | null;
  onConfirmPin: (pin: string) => Promise<void>;
  onRequestOtp: (email: string) => Promise<void>;
  onVerifyOtp: (email: string, code: string) => Promise<void>;
}

export const PairingModal: React.FC<PairingModalProps> = ({
  isOpen,
  onClose,
  ticket,
  onConfirmPin,
  onRequestOtp,
  onVerifyOtp,
}) => {
  const dialogRef = useDialogFocus(isOpen, onClose);
  const [error, setError] = useState('');
  const [tab, setTab] = useState<'qr' | 'pin' | 'email'>('qr');
  const [pinInput, setPinInput] = useState('');
  const [emailInput, setEmailInput] = useState('');
  const [otpCode, setOtpCode] = useState('');
  const [otpSent, setOtpSent] = useState(false);
  const [copiedLink, setCopiedLink] = useState(false);
  const [copiedPin, setCopiedPin] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  if (!isOpen) return null;

  const handleCopyLink = () => {
    if (ticket?.deep_link) {
      navigator.clipboard.writeText(ticket.deep_link);
      setCopiedLink(true);
      setTimeout(() => setCopiedLink(false), 2000);
    }
  };

  const handleCopyPin = () => {
    if (ticket?.pairing_code) {
      navigator.clipboard.writeText(ticket.pairing_code);
      setCopiedPin(true);
      setTimeout(() => setCopiedPin(false), 2000);
    }
  };

  const handleClaimPin = async () => {
    if (!pinInput.trim()) return;
    setError('');
    setIsLoading(true);
    try {
      await onConfirmPin(pinInput.trim());
      onClose();
    } catch(e: any) {
      setError(e.message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSendOtp = async () => {
    if (!emailInput.trim()) return;
    setError('');
    setIsLoading(true);
    try {
      await onRequestOtp(emailInput.trim());
      setOtpSent(true);
    } catch(e: any) {
      setError(e.message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleVerifyOtp = async () => {
    if (!otpCode.trim()) return;
    setError('');
    setIsLoading(true);
    try {
      await onVerifyOtp(emailInput.trim(), otpCode.trim());
      onClose();
    } catch(e: any) {
      setError(e.message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div ref={dialogRef} className="modal-overlay" role="dialog" aria-modal="true" aria-label="Pair a device">
      <div className="modal-card">
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <FiShield size={16} style={{ color: 'var(--accent-primary)' }} />
            <span className="modal-title">Pair Machine or Spoke Node</span>
          </div>
          <button className="icon-button" aria-label="Close pairing dialog" onClick={onClose}>
            <FiX size={15} />
          </button>
        </div>

        {/* Modal Tabs */}
        <div style={{ display: 'flex', borderBottom: '1px solid var(--border-subtle)', padding: '0 18px' }}>
          <button
            className={`filter-tab ${tab === 'qr' ? 'active' : ''}`}
            onClick={() => setTab('qr')}
            style={{ borderRadius: 0, borderBottom: tab === 'qr' ? '2px solid var(--accent-primary)' : 'none' }}
          >
            Host QR & Code
          </button>
          <button
            className={`filter-tab ${tab === 'pin' ? 'active' : ''}`}
            onClick={() => setTab('pin')}
            style={{ borderRadius: 0, borderBottom: tab === 'pin' ? '2px solid var(--accent-primary)' : 'none' }}
          >
            Enter 6-Digit PIN
          </button>
          <button
            className={`filter-tab ${tab === 'email' ? 'active' : ''}`}
            onClick={() => setTab('email')}
            style={{ borderRadius: 0, borderBottom: tab === 'email' ? '2px solid var(--accent-primary)' : 'none' }}
          >
            Email OTP
          </button>
        </div>

        <div className="modal-body">
          {error && <p role="alert" style={{color:'var(--danger-text)'}}>{error}</p>}
          {tab === 'qr' && (
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14 }}>
              {ticket?.qr_svg ? (
                <div
                  style={{
                    backgroundColor: '#ffffff',
                    padding: 12,
                    borderRadius: 'var(--radius-sm)',
                    border: '1px solid var(--border-default)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center'
                  }}
                  dangerouslySetInnerHTML={{ __html: ticket.qr_svg }}
                />
              ) : (
                <div style={{ padding: 24, color: 'var(--text-muted)' }}>Generating QR ticket...</div>
              )}

              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase' }}>
                  PAIRING PIN CODE
                </div>
                <div
                  style={{
                    fontSize: 24,
                    fontWeight: 700,
                    fontFamily: 'var(--font-mono)',
                    letterSpacing: 4,
                    color: 'var(--text-primary)',
                    marginTop: 4
                  }}
                >
                  {ticket?.pairing_code || '------'}
                </div>
              </div>

              <div style={{ display: 'flex', gap: 8, width: '100%' }}>
                <button
                  className="icon-button"
                  style={{ flex: 1 }}
                  onClick={handleCopyPin}
                >
                  {copiedPin ? <FiCheck size={13} /> : <FiCopy size={13} />}
                  <span>{copiedPin ? 'Copied PIN' : 'Copy PIN'}</span>
                </button>
                <button
                  className="icon-button"
                  style={{ flex: 1 }}
                  onClick={handleCopyLink}
                >
                  {copiedLink ? <FiCheck size={13} /> : <FiLink size={13} />}
                  <span>{copiedLink ? 'Copied Link' : 'Copy Deep Link'}</span>
                </button>
              </div>
            </div>
          )}

          {tab === 'pin' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                Enter the 6-digit numeric PIN displayed on the Central Hub:
              </p>
              <input
                type="text"
                maxLength={6}
                className="chat-input-field"
                placeholder="e.g. 849201"
                value={pinInput}
                onChange={e => setPinInput(e.target.value)}
                style={{ textAlign: 'center', fontSize: 18, letterSpacing: 4, fontFamily: 'var(--font-mono)' }}
              />
              <button
                className="icon-button primary"
                disabled={!/^\d{6}$/.test(pinInput) || isLoading}
                onClick={handleClaimPin}
              >
                <span>{isLoading ? 'Verifying...' : 'Claim & Authenticate'}</span>
              </button>
            </div>
          )}

          {tab === 'email' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                Authenticate node enrollment with one-time verification code:
              </p>
              <input
                type="email"
                className="chat-input-field"
                placeholder="admin@domain.internal"
                value={emailInput}
                onChange={e => setEmailInput(e.target.value)}
                disabled={otpSent}
              />
              {!otpSent ? (
                <button
                  className="icon-button primary"
                  disabled={!emailInput.includes('@') || isLoading}
                  onClick={handleSendOtp}
                >
                  <span>{isLoading ? 'Sending...' : 'Request Verification Code'}</span>
                </button>
              ) : (
                <>
                  <input
                    type="text"
                    maxLength={6}
                    className="chat-input-field"
                    placeholder="Enter 6-digit OTP..."
                    value={otpCode}
                    onChange={e => setOtpCode(e.target.value)}
                    style={{ textAlign: 'center', letterSpacing: 3, fontFamily: 'var(--font-mono)' }}
                  />
                  <button
                    className="icon-button primary"
                    disabled={otpCode.length < 6 || isLoading}
                    onClick={handleVerifyOtp}
                  >
                    <span>{isLoading ? 'Verifying...' : 'Confirm Authentication'}</span>
                  </button>
                </>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};


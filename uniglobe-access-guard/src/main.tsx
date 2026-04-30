import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import {
  BellRing,
  CheckCircle2,
  Clock3,
  History,
  Laptop,
  LockKeyhole,
  Power,
  RotateCw,
  Settings,
  ShieldAlert,
  ShieldCheck,
  TimerReset,
  XCircle
} from 'lucide-react';
import './styles.css';

type Mode = 'always_allow' | 'ask_to_allow';
type AccessState = 'enabled' | 'blocked' | 'countdown' | 'temporary' | 'error';

type StatusDto = {
  mode: Mode;
  state: AccessState;
  serviceRunning: boolean;
  autostartEnabled: boolean;
  approvedUntil?: string | null;
  countdownRemainingSeconds?: number | null;
  approvedRemainingSeconds?: number | null;
  incomingRequestActive: boolean;
  incomingRequestAt?: string | null;
  message?: string | null;
  mockMode: boolean;
};

type AuditEvent = {
  ts: string;
  event: string;
  mode: Mode;
  state: AccessState;
  message: string;
  durationMinutes?: number | null;
  countdownSeconds?: number | null;
};

const fallbackStatus: StatusDto = {
  mode: 'ask_to_allow',
  state: 'blocked',
  serviceRunning: false,
  autostartEnabled: false,
  incomingRequestActive: false,
  message: 'Mock service state is active in the desktop app.',
  mockMode: true
};

const durationOptions = [15, 30, 60];

function statusLabel(status: StatusDto) {
  switch (status.state) {
    case 'enabled':
      return 'Enabled';
    case 'countdown':
      return 'Countdown';
    case 'temporary':
      return 'Temporarily Allowed';
    case 'error':
      return 'Error';
    default:
      return 'Permission Required';
  }
}

function modeLabel(mode: Mode) {
  return mode === 'always_allow' ? 'Allow access all the time' : 'Ask to allow every time';
}

function formatSeconds(total?: number | null) {
  if (total === undefined || total === null) return '--';
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

function formatDateTime(value?: string | null) {
  if (!value) return 'Not set';
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit'
  }).format(new Date(value));
}

function eventLabel(event: string) {
  return event
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function App() {
  const [status, setStatus] = useState<StatusDto>(fallbackStatus);
  const [auditLog, setAuditLog] = useState<AuditEvent[]>([]);
  const [duration, setDuration] = useState(30);
  const [loadingAction, setLoadingAction] = useState<string | null>(null);

  const refreshAuditLog = useCallback(async () => {
    try {
      const events = await invoke<AuditEvent[]>('get_audit_log');
      setAuditLog(events.slice().reverse());
    } catch {
      setAuditLog([]);
    }
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      const result = await invoke<StatusDto>('get_status');
      setStatus(result);
      return result;
    } catch {
      setStatus(fallbackStatus);
      return fallbackStatus;
    }
  }, []);

  const refreshAll = useCallback(async () => {
    await Promise.all([refreshStatus(), refreshAuditLog()]);
  }, [refreshAuditLog, refreshStatus]);

  useEffect(() => {
    refreshAll();
    const id = window.setInterval(refreshAll, 1000);
    return () => window.clearInterval(id);
  }, [refreshAll]);

  async function runAction(name: string, args?: Record<string, unknown>) {
    setLoadingAction(name);
    try {
      const result = await invoke<StatusDto>(name, args || {});
      setStatus(result);
      await refreshAuditLog();
    } catch (error) {
      setStatus((current) => ({
        ...current,
        state: 'error',
        message: error instanceof Error ? error.message : String(error)
      }));
    } finally {
      setLoadingAction(null);
    }
  }

  const metrics = useMemo(
    () => [
      {
        label: 'Mock Service',
        value: status.serviceRunning ? 'Running' : 'Stopped',
        icon: status.serviceRunning ? CheckCircle2 : XCircle
      },
      {
        label: 'Autostart',
        value: status.autostartEnabled ? 'Enabled' : 'Disabled',
        icon: RotateCw
      },
      {
        label: 'Remaining',
        value:
          status.state === 'countdown'
            ? formatSeconds(status.countdownRemainingSeconds)
            : formatSeconds(status.approvedRemainingSeconds),
        icon: Clock3
      },
      {
        label: 'Approved Until',
        value: formatDateTime(status.approvedUntil),
        icon: TimerReset
      }
    ],
    [status]
  );

  const busy = loadingAction !== null;
  const countdownOpen = status.state === 'countdown';
  const incomingOpen = status.incomingRequestActive && !countdownOpen;

  return (
    <main className="shell">
      <header className="app-header">
        <div>
          <p className="eyebrow">Office Remote Support</p>
          <h1>UniGlobe Access Guard</h1>
        </div>
        <div className={`status-badge ${status.state}`}>
          {status.serviceRunning ? <ShieldCheck size={20} /> : <ShieldAlert size={20} />}
          <span>{statusLabel(status)}</span>
        </div>
      </header>

      <section className="summary-band">
        <div>
          <span className="muted">Current Mode</span>
          <strong>{modeLabel(status.mode)}</strong>
          <p>{status.message}</p>
        </div>
        <button
          className="danger-action"
          disabled={busy}
          onClick={() => runAction('terminate_now')}
          type="button"
        >
          <Power size={18} />
          Deny / Terminate Now
        </button>
      </section>

      <section className="mode-grid" aria-label="Access mode">
        <button
          className={`mode-card ${status.mode === 'always_allow' ? 'active' : ''}`}
          disabled={busy}
          onClick={() => runAction('set_mode', { mode: 'always_allow' })}
          type="button"
        >
          <Laptop size={24} />
          <span>Allow access all the time</span>
          <small>Mock service running with autostart enabled.</small>
        </button>
        <button
          className={`mode-card ${status.mode === 'ask_to_allow' ? 'active' : ''}`}
          disabled={busy}
          onClick={() => runAction('set_mode', { mode: 'ask_to_allow' })}
          type="button"
        >
          <LockKeyhole size={24} />
          <span>Ask to allow every time</span>
          <small>Mock service stopped until local approval.</small>
        </button>
      </section>

      <section className="action-panel">
        <div className="action-copy">
          <p className="eyebrow">Approval</p>
          <h2>Allow Once</h2>
          <p>Starts a visible 60-second countdown before the mock service is marked available.</p>
        </div>
        <div className="action-controls">
          <div className="segmented" aria-label="Approval duration">
            {durationOptions.map((option) => (
              <button
                className={duration === option ? 'selected' : ''}
                key={option}
                onClick={() => setDuration(option)}
                type="button"
              >
                {option}m
              </button>
            ))}
          </div>
          <button
            className="primary-action"
            disabled={busy}
            onClick={() => runAction('approve_once', { minutes: duration })}
            type="button"
          >
            <BellRing size={18} />
            Allow Once
          </button>
          <button
            className="secondary-action"
            disabled={busy}
            onClick={() => runAction('approve_until_revoked')}
            type="button"
          >
            Until Revoked
          </button>
        </div>
      </section>

      <section className="metrics-grid" aria-label="Mock service state">
        {metrics.map((metric) => {
          const Icon = metric.icon;
          return (
            <div className="metric" key={metric.label}>
              <Icon size={18} />
              <span>{metric.label}</span>
              <strong>{metric.value}</strong>
            </div>
          );
        })}
      </section>

      <section className="workspace-grid">
        <section className="settings-panel">
          <div className="section-title">
            <Settings size={20} />
            <h2>Settings</h2>
          </div>
          <div className="setting-row">
            <span>Default approval</span>
            <strong>{duration} minutes</strong>
          </div>
          <div className="setting-row">
            <span>Countdown</span>
            <strong>60 seconds</strong>
          </div>
          <div className="setting-row">
            <span>Backend</span>
            <strong>{status.mockMode ? 'Mock' : 'System'}</strong>
          </div>
          <div className="tray-menu">
            <span>System Tray</span>
            <button disabled={busy} onClick={() => runAction('mock_incoming_request')} type="button">
              Mock Request
            </button>
            <button disabled={busy} onClick={() => runAction('approve_once', { minutes: 15 })} type="button">
              Allow 15m
            </button>
            <button disabled={busy} onClick={() => runAction('terminate_now')} type="button">
              Terminate
            </button>
          </div>
        </section>

        <section className="audit-panel">
          <div className="section-title">
            <History size={20} />
            <h2>Audit Log</h2>
          </div>
          <div className="audit-list">
            {auditLog.length === 0 ? (
              <div className="empty-state">No audit events loaded.</div>
            ) : (
              auditLog.slice(0, 8).map((event) => (
                <article className="audit-event" key={`${event.ts}-${event.event}-${event.message}`}>
                  <div>
                    <strong>{eventLabel(event.event)}</strong>
                    <span>{event.message}</span>
                  </div>
                  <time dateTime={event.ts}>{formatDateTime(event.ts)}</time>
                </article>
              ))
            )}
          </div>
        </section>
      </section>

      {countdownOpen && (
        <div className="modal-backdrop" role="presentation">
          <section className="modal" role="dialog" aria-modal="true" aria-labelledby="countdown-title">
            <TimerReset size={34} />
            <h2 id="countdown-title">Access starts in {status.countdownRemainingSeconds ?? 0}s</h2>
            <p>Save work and close private content before the countdown ends.</p>
            <button
              className="danger-action"
              disabled={busy}
              onClick={() => runAction('terminate_now')}
              type="button"
            >
              <Power size={18} />
              Cancel / Deny
            </button>
          </section>
        </div>
      )}

      {incomingOpen && (
        <div className="modal-backdrop" role="presentation">
          <section className="modal" role="dialog" aria-modal="true" aria-labelledby="incoming-title">
            <BellRing size={34} />
            <h2 id="incoming-title">Incoming remote access request</h2>
            <p>
              Requested {formatDateTime(status.incomingRequestAt)}. Save work and close private
              content before allowing.
            </p>
            <div className="modal-actions">
              <button
                className="primary-action"
                disabled={busy}
                onClick={() => runAction('approve_once', { minutes: duration })}
                type="button"
              >
                Allow {duration}m
              </button>
              <button
                className="secondary-action"
                disabled={busy}
                onClick={() => runAction('approve_until_revoked')}
                type="button"
              >
                Until Revoked
              </button>
              <button
                className="danger-action"
                disabled={busy}
                onClick={() => runAction('terminate_now')}
                type="button"
              >
                Deny
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}

createRoot(document.getElementById('root')!).render(<App />);

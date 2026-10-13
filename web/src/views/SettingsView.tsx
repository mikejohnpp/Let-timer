import React, { useState, useEffect } from 'react';
import {
  Globe,
  Bell,
  Key,
  LogOut,
  Plus,
  Trash2,
  Copy,
  Check,
} from 'lucide-react';
import { getStoredToken, setStoredToken } from '../api/client';

export interface TokenInfo {
  id: number;
  name: string;
  created_at: string;
  last_used_at?: string | null;
  revoked_at?: string | null;
}

export interface CreatedTokenRes {
  token: string;
  token_info: TokenInfo;
}

export const SettingsView: React.FC = () => {
  const [timezone, setTimezone] = useState<string>('UTC');
  const [newTimezone, setNewTimezone] = useState<string>('');
  const [tokens, setTokens] = useState<TokenInfo[]>([]);
  const [newTokenName, setNewTokenName] = useState('');
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [notifPermission, setNotifPermission] = useState<string>(
    typeof Notification !== 'undefined' ? Notification.permission : 'denied'
  );

  const loadSettingsAndTokens = async () => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const [resTz, resTok] = await Promise.all([
        fetch('/api/v1/settings/timezone', { headers }),
        fetch('/api/v1/auth/tokens', { headers }),
      ]);

      if (resTz.ok) {
        const data = await resTz.json();
        setTimezone(data.timezone);
        setNewTimezone(data.timezone);
      }
      if (resTok.ok) {
        setTokens(await resTok.json());
      }
    } catch (e) {
      console.error('Failed to load settings', e);
    }
  };

  useEffect(() => {
    loadSettingsAndTokens();
  }, []);

  const handleSaveTimezone = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/settings/timezone', {
        method: 'PUT',
        headers,
        body: JSON.stringify({ timezone: newTimezone.trim() }),
      });

      if (res.ok) {
        setTimezone(newTimezone.trim());
        alert('Timezone updated successfully');
      } else {
        const err = await res.json().catch(() => ({}));
        alert(err.message || 'Failed to update timezone');
      }
    } catch (e) {
      console.error('Timezone update failed', e);
    }
  };

  const handleRequestNotifications = async () => {
    if ('Notification' in window) {
      const perm = await Notification.requestPermission();
      setNotifPermission(perm);
    }
  };

  const handleCreateToken = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTokenName.trim()) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/auth/tokens', {
        method: 'POST',
        headers,
        body: JSON.stringify({ name: newTokenName.trim() }),
      });

      if (res.ok) {
        const data: CreatedTokenRes = await res.json();
        setCreatedSecret(data.token);
        setNewTokenName('');
        loadSettingsAndTokens();
      } else {
        const err = await res.json().catch(() => ({}));
        alert(err.message || 'Failed to create token');
      }
    } catch (e) {
      console.error('Create token failed', e);
    }
  };

  const handleRevokeToken = async (name: string) => {
    if (!confirm(`Revoke token "${name}"? This action cannot be undone.`)) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/auth/tokens/revoke', {
        method: 'POST',
        headers,
        body: JSON.stringify({ name }),
      });

      if (res.ok) loadSettingsAndTokens();
    } catch (e) {
      console.error('Revoke token failed', e);
    }
  };

  const handleRotateToken = async (name: string) => {
    if (!confirm(`Rotate token "${name}"? The previous secret will be revoked immediately and a new secret will be generated.`)) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/auth/tokens/rotate', {
        method: 'POST',
        headers,
        body: JSON.stringify({ name }),
      });

      if (res.ok) {
        const data: CreatedTokenRes = await res.json();
        setCreatedSecret(data.token);
        loadSettingsAndTokens();
      } else {
        const err = await res.json().catch(() => ({}));
        alert(err.message || 'Failed to rotate token');
      }
    } catch (e) {
      console.error('Rotate token failed', e);
    }
  };

  const handleLogout = () => {
    setStoredToken(null);
    window.location.reload();
  };

  const handleCopySecret = () => {
    if (createdSecret) {
      navigator.clipboard.writeText(createdSecret);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 py-8 flex flex-col gap-6">
      <div className="flex items-center justify-between border-b border-[var(--border)] pb-4">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text)]">Settings</h1>
          <p className="text-xs text-[var(--muted)] mt-1">Configure time zone, notifications, and API credentials.</p>
        </div>

        <button
          onClick={handleLogout}
          className="flex items-center gap-1.5 text-xs font-semibold px-3 py-2 rounded-xl border border-[var(--border)] hover:bg-[var(--surface-hover)] text-red-500 transition"
        >
          <LogOut className="w-3.5 h-3.5" />
          <span>Log out</span>
        </button>
      </div>

      {/* Timezone Setting */}
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
        <div className="flex items-center gap-2 mb-2 font-bold text-sm">
          <Globe className="w-4 h-4 text-[var(--accent)]" />
          <span>Server Timezone</span>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          All calendar dates ("today", "tomorrow", filters) are evaluated using this timezone. Current: <strong>{timezone}</strong>.
        </p>

        <form onSubmit={handleSaveTimezone} className="flex gap-2 max-w-sm">
          <input
            type="text"
            required
            value={newTimezone}
            onChange={(e) => setNewTimezone(e.target.value)}
            placeholder="e.g. Asia/Ho_Chi_Minh or UTC"
            className="flex-1 px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-xs outline-none focus:border-[var(--accent)]"
          />
          <button
            type="submit"
            className="px-4 py-2 bg-[var(--accent)] text-white text-xs font-semibold rounded-xl shadow-sm transition"
          >
            Save
          </button>
        </form>
      </div>

      {/* Notifications Setting */}
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
        <div className="flex items-center gap-2 mb-2 font-bold text-sm">
          <Bell className="w-4 h-4 text-[var(--accent)]" />
          <span>Browser Notifications</span>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          Receive background alerts when task estimates expire. Status: <strong className="capitalize">{notifPermission}</strong>.
        </p>

        {notifPermission !== 'granted' && (
          <button
            onClick={handleRequestNotifications}
            className="px-4 py-2 bg-[var(--surface-hover)] border border-[var(--border)] hover:bg-[var(--border)] text-xs font-semibold rounded-xl transition"
          >
            Enable Notifications
          </button>
        )}
      </div>

      {/* API Tokens Manager */}
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2 font-bold text-sm">
            <Key className="w-4 h-4 text-[var(--accent)]" />
            <span>API Tokens</span>
          </div>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          Authenticate CLI profiles and third-party integrations. Tokens are displayed only once upon creation.
        </p>

        {/* Created secret banner */}
        {createdSecret && (
          <div className="p-4 mb-4 bg-emerald-500/10 border border-emerald-500/30 rounded-xl text-xs">
            <div className="font-bold text-emerald-500 mb-1">New Token Created! Copy it now:</div>
            <div className="flex items-center gap-2 mt-2">
              <code className="flex-1 p-2 bg-[var(--surface)] border border-[var(--border)] rounded font-mono text-xs select-all break-all">
                {createdSecret}
              </code>
              <button
                onClick={handleCopySecret}
                className="px-3 py-2 bg-emerald-500 text-white rounded font-semibold flex items-center gap-1 text-xs"
              >
                {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copied ? 'Copied' : 'Copy'}</span>
              </button>
            </div>
          </div>
        )}

        {/* Create new token form */}
        <form onSubmit={handleCreateToken} className="flex gap-2 max-w-sm mb-4">
          <input
            type="text"
            required
            placeholder="Token name (e.g. laptop-cli)"
            value={newTokenName}
            onChange={(e) => setNewTokenName(e.target.value)}
            className="flex-1 px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-xs outline-none focus:border-[var(--accent)]"
          />
          <button
            type="submit"
            className="flex items-center gap-1 px-4 py-2 bg-[var(--accent)] text-white text-xs font-semibold rounded-xl shadow-sm transition"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Create</span>
          </button>
        </form>

        {/* Existing tokens list */}
        <div className="divide-y divide-[var(--border)] text-xs">
          {tokens.map((t) => (
            <div key={t.id} className="py-2.5 flex items-center justify-between gap-3">
              <div>
                <div className="font-semibold text-[var(--text)]">{t.name}</div>
                <div className="text-[10px] text-[var(--muted)]">
                  Created: {new Date(t.created_at).toLocaleDateString()}
                  {t.last_used_at && ` • Last used: ${new Date(t.last_used_at).toLocaleDateString()}`}
                </div>
              </div>

              {!t.revoked_at ? (
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => handleRotateToken(t.name)}
                    className="px-2 py-1 text-[10px] text-[var(--accent)] hover:bg-[var(--surface-hover)] border border-[var(--border)] rounded-md font-medium transition"
                    title="Rotate token"
                  >
                    Rotate
                  </button>
                  <button
                    onClick={() => handleRevokeToken(t.name)}
                    className="p-1.5 text-[var(--muted)] hover:text-red-500 rounded-lg hover:bg-red-500/10 transition"
                    title="Revoke token"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              ) : (
                <span className="text-[10px] text-red-500 font-medium">Revoked</span>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

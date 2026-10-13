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
  Palette,
  Timer,
} from 'lucide-react';
import { getStoredToken, setStoredToken } from '../api/client';
import {
  Button,
  IconButton,
  SegmentedControl,
  Checkbox,
  Card,
  Input,
  Badge,
  useToast,
  useConfirm,
} from '../components/ui';

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
  const [theme, setTheme] = useState<'light' | 'dark' | 'system'>('system');
  const [palette, setPalette] = useState<'default' | 'forest'>('default');
  const [defaultEstimate, setDefaultEstimate] = useState<string>('25');
  const [noDefaultEstimate, setNoDefaultEstimate] = useState<boolean>(false);

  const [timezone, setTimezone] = useState<string>('UTC');
  const [newTimezone, setNewTimezone] = useState<string>('');
  const [tokens, setTokens] = useState<TokenInfo[]>([]);
  const [newTokenName, setNewTokenName] = useState('');
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [notifPermission, setNotifPermission] = useState<string>(
    typeof Notification !== 'undefined' ? Notification.permission : 'denied'
  );

  const { showToast } = useToast();
  const confirm = useConfirm();

  // Load preferences from localStorage
  useEffect(() => {
    const savedTheme = (localStorage.getItem('let_timer_theme') as 'light' | 'dark' | 'system') || 'system';
    setTheme(savedTheme);

    const savedPalette = (localStorage.getItem('let_timer_palette') as 'default' | 'forest') || 'default';
    setPalette(savedPalette);

    const savedDefEst = localStorage.getItem('let_timer_default_estimate');
    if (savedDefEst === 'none') {
      setNoDefaultEstimate(true);
      setDefaultEstimate('');
    } else if (savedDefEst && !isNaN(Number(savedDefEst))) {
      setDefaultEstimate(savedDefEst);
      setNoDefaultEstimate(false);
    }
  }, []);

  const handleThemeChange = (t: string) => {
    const next = t as 'light' | 'dark' | 'system';
    setTheme(next);
    localStorage.setItem('let_timer_theme', next);
    if (next === 'system') {
      document.documentElement.removeAttribute('data-theme');
    } else {
      document.documentElement.setAttribute('data-theme', next);
    }
  };

  const handlePaletteChange = (p: string) => {
    const next = p as 'default' | 'forest';
    setPalette(next);
    localStorage.setItem('let_timer_palette', next);
    if (next === 'default') {
      document.documentElement.removeAttribute('data-palette');
    } else {
      document.documentElement.setAttribute('data-palette', next);
    }
    showToast(`Switched palette to ${next === 'forest' ? 'Forest & Moss' : 'Default (Indigo)'}`, 'info');
  };

  const handleEstimateChange = (val: string) => {
    setDefaultEstimate(val);
    if (val && !isNaN(Number(val))) {
      localStorage.setItem('let_timer_default_estimate', val);
      setNoDefaultEstimate(false);
    }
  };

  const handleNoEstimateToggle = (checked: boolean) => {
    setNoDefaultEstimate(checked);
    if (checked) {
      localStorage.setItem('let_timer_default_estimate', 'none');
      setDefaultEstimate('');
    } else {
      localStorage.setItem('let_timer_default_estimate', '25');
      setDefaultEstimate('25');
    }
  };

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
        showToast('Timezone updated successfully', 'success');
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to update timezone', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Timezone update failed', 'danger');
    }
  };

  const handleRequestNotifications = async () => {
    if ('Notification' in window) {
      const perm = await Notification.requestPermission();
      setNotifPermission(perm);
      if (perm === 'granted') showToast('Notifications enabled', 'success');
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
        showToast('API token created', 'success');
        loadSettingsAndTokens();
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to create token', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Create token failed', 'danger');
    }
  };

  const handleRotateToken = async (name: string) => {
    const ok = await confirm({
      title: `Rotate token "${name}"?`,
      body: 'The previous secret will be revoked immediately and a new secret will be generated.',
      tone: 'danger',
      confirmLabel: 'Rotate token',
    });
    if (!ok) return;

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
        showToast(`Token "${name}" rotated successfully`, 'success');
        loadSettingsAndTokens();
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to rotate token', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Rotate token failed', 'danger');
    }
  };

  const handleRevokeToken = async (name: string) => {
    const ok = await confirm({
      title: `Revoke token "${name}"?`,
      body: 'Clients using this token will be disconnected immediately. This cannot be undone.',
      tone: 'danger',
      confirmLabel: 'Revoke token',
    });
    if (!ok) return;

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

      if (res.ok) {
        showToast(`Token "${name}" revoked`, 'info');
        loadSettingsAndTokens();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Revoke token failed', 'danger');
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
      showToast('Token copied to clipboard', 'info');
    }
  };

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 py-6 pb-24 flex flex-col gap-6">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-[var(--border)] pb-4">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text)] tracking-tight">Settings</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Configure appearance, time zone, notifications, and API credentials.
          </p>
        </div>

        <Button variant="danger" size="sm" onClick={handleLogout}>
          <LogOut className="w-3.5 h-3.5" />
          <span>Log out</span>
        </Button>
      </div>

      {/* Appearance Setting */}
      <Card>
        <div className="flex items-center gap-2 mb-1.5 font-bold text-sm text-[var(--text)]">
          <Palette className="w-4 h-4 text-[var(--accent-text)]" />
          <span>Appearance</span>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          Customize your theme and color palette.
        </p>

        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 py-2 border-b border-[var(--border)] pb-3">
          <div>
            <div className="text-xs font-semibold text-[var(--text)]">Theme Mode</div>
            <div className="text-[11px] text-[var(--muted)]">Light, Dark or match system</div>
          </div>
          <SegmentedControl
            value={theme}
            onValueChange={handleThemeChange}
            items={[
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
              { value: 'system', label: 'System' },
            ]}
          />
        </div>

        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pt-3">
          <div>
            <div className="text-xs font-semibold text-[var(--text)]">Color Palette</div>
            <div className="text-[11px] text-[var(--muted)]">Accent and focus tones</div>
          </div>
          <SegmentedControl
            value={palette}
            onValueChange={handlePaletteChange}
            items={[
              { value: 'default', label: 'Indigo' },
              { value: 'forest', label: '🌲 Forest & Moss' },
            ]}
          />
        </div>
      </Card>

      {/* New Tasks Default Estimate Setting */}
      <Card>
        <div className="flex items-center gap-2 mb-1.5 font-bold text-sm text-[var(--text)]">
          <Timer className="w-4 h-4 text-[var(--accent-text)]" />
          <span>New Tasks</span>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          Used by quick add and the task drawer. Saved on this device only.
        </p>

        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-2">
            <span className="text-xs text-[var(--muted)]">Default estimate:</span>
            <Input
              type="number"
              min="1"
              disabled={noDefaultEstimate}
              value={defaultEstimate}
              onChange={(e) => handleEstimateChange(e.target.value)}
              placeholder="25"
              className="w-20 h-8 text-center"
            />
            <span className="text-xs text-[var(--muted)]">min</span>
          </div>

          <Checkbox
            checked={noDefaultEstimate}
            onCheckedChange={(c) => handleNoEstimateToggle(!!c)}
            label="No estimate by default"
          />
        </div>
      </Card>

      {/* Timezone Setting */}
      <Card>
        <div className="flex items-center gap-2 mb-1.5 font-bold text-sm text-[var(--text)]">
          <Globe className="w-4 h-4 text-[var(--accent-text)]" />
          <span>Server Timezone</span>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          All calendar dates ("today", "tomorrow", filters) are evaluated using this timezone. Current: <strong>{timezone}</strong>.
        </p>

        <form onSubmit={handleSaveTimezone} className="flex gap-2 max-w-sm">
          <Input
            required
            value={newTimezone}
            onChange={(e) => setNewTimezone(e.target.value)}
            placeholder="e.g. Asia/Ho_Chi_Minh or UTC"
            className="flex-1"
          />
          <Button type="submit" variant="primary" size="sm">
            Save
          </Button>
        </form>
      </Card>

      {/* Notifications Setting */}
      <Card>
        <div className="flex items-center gap-2 mb-1.5 font-bold text-sm text-[var(--text)]">
          <Bell className="w-4 h-4 text-[var(--accent-text)]" />
          <span>Browser Notifications</span>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          Receive background alerts when task estimates expire. Status: <strong className="capitalize">{notifPermission}</strong>.
        </p>

        {notifPermission !== 'granted' ? (
          <Button variant="secondary" size="sm" onClick={handleRequestNotifications}>
            Enable notifications
          </Button>
        ) : (
          <Badge variant="success">Enabled</Badge>
        )}
      </Card>

      {/* API Tokens Manager */}
      <Card>
        <div className="flex items-center justify-between mb-1.5">
          <div className="flex items-center gap-2 font-bold text-sm text-[var(--text)]">
            <Key className="w-4 h-4 text-[var(--accent-text)]" />
            <span>API Tokens</span>
          </div>
        </div>
        <p className="text-xs text-[var(--muted)] mb-4">
          Authenticate CLI profiles and third-party integrations. Tokens are displayed only once upon creation.
        </p>

        {/* Created secret banner */}
        {createdSecret && (
          <div className="p-3.5 mb-4 bg-[var(--warning-bg)] border border-[var(--warning)]/30 rounded-[var(--radius-control)] text-xs">
            <div className="font-bold text-[var(--text)] mb-1">
              Copy this token now. It won't be shown again.
            </div>
            <div className="flex items-center gap-2 mt-2">
              <code className="flex-1 p-2 bg-[var(--surface)] border border-[var(--border)] rounded font-mono text-xs select-all break-all text-[var(--text)]">
                {createdSecret}
              </code>
              <Button variant="primary" size="sm" onClick={handleCopySecret}>
                {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copied ? 'Copied' : 'Copy'}</span>
              </Button>
            </div>
          </div>
        )}

        {/* Create new token form */}
        <form onSubmit={handleCreateToken} className="flex gap-2 max-w-sm mb-4">
          <Input
            required
            placeholder="Token name (e.g. laptop-cli)"
            value={newTokenName}
            onChange={(e) => setNewTokenName(e.target.value)}
            className="flex-1"
          />
          <Button type="submit" variant="primary" size="sm">
            <Plus className="w-3.5 h-3.5 stroke-[3]" />
            <span>Create</span>
          </Button>
        </form>

        {/* Existing tokens list */}
        <div className="divide-y divide-[var(--border)] text-xs">
          {tokens.map((t) => (
            <div key={t.id} className="py-2.5 flex items-center justify-between gap-3">
              <div>
                <div className="font-semibold text-[var(--text)]">{t.name}</div>
                <div className="text-[11px] text-[var(--muted)]">
                  Created: {new Date(t.created_at).toLocaleDateString()}
                  {t.last_used_at && ` • Last used: ${new Date(t.last_used_at).toLocaleDateString()}`}
                </div>
              </div>

              {!t.revoked_at ? (
                <div className="flex items-center gap-1.5">
                  <Button variant="secondary" size="sm" onClick={() => handleRotateToken(t.name)} className="h-7 text-[11px] px-2.5">
                    Rotate
                  </Button>
                  <IconButton variant="danger" size="sm" onClick={() => handleRevokeToken(t.name)} aria-label={`Revoke token ${t.name}`}>
                    <Trash2 className="w-3.5 h-3.5" />
                  </IconButton>
                </div>
              ) : (
                <Badge variant="muted">Revoked</Badge>
              )}
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
};

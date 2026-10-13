import React, { useState, useEffect } from 'react';
import {
  Key,
  LogOut,
  Plus,
  Trash2,
  Copy,
  Check,
  Palette,
  Timer,
  Image as ImageIcon,
  Upload,
  Sparkles,
  X,
} from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredToken, setStoredToken } from '../api/client';
import { useWallpaper } from '../hooks/useWallpaper';
import {
  Button,
  IconButton,
  SegmentedControl,
  Checkbox,
  Input,
  Badge,
  useToast,
  useConfirm,
} from '../components/ui';

type Media = components['schemas']['MediaDto'];

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

  const {
    wallpaperUrl,
    dynamicColor,
    extractedColor,
    setWallpaper,
    setDynamicColor,
    clearWallpaper,
  } = useWallpaper();

  const [mediaImages, setMediaImages] = useState<Media[]>([]);
  const fileInputRef = React.useRef<HTMLInputElement | null>(null);

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

  const handleUploadWallpaper = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async () => {
      const dataUrl = reader.result as string;
      if (dataUrl) {
        try {
          const token = getStoredToken();
          const headers: Record<string, string> = {
            'Content-Type': 'application/json',
            'X-Let-Timer-Protocol': '1',
          };
          if (token) headers['Authorization'] = `Bearer ${token}`;

          fetch('/api/v1/medias', {
            method: 'POST',
            headers,
            body: JSON.stringify({
              name: file.name.replace(/\.[^/.]+$/, ''),
              media_type: 'image',
              url: dataUrl,
              description: 'Uploaded wallpaper',
            }),
          })
            .then(async (res) => {
              if (res.ok) {
                const savedMedia = await res.json();
                setMediaImages((prev) => [savedMedia, ...prev]);
              }
            })
            .catch(() => {});

          await setWallpaper(dataUrl);
          showToast('Wallpaper applied successfully', 'success');
        } catch {
          showToast('Failed to apply wallpaper', 'danger');
        }
      }
    };
    reader.readAsDataURL(file);
    if (fileInputRef.current) fileInputRef.current.value = '';
  };

  const loadSettingsAndTokens = async () => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const [resTz, resTok, resMedias] = await Promise.all([
        fetch('/api/v1/settings/timezone', { headers }),
        fetch('/api/v1/auth/tokens', { headers }),
        fetch('/api/v1/medias', { headers }),
      ]);

      if (resTz.ok) {
        const data = await resTz.json();
        setTimezone(data.timezone);
        setNewTimezone(data.timezone);
      }
      if (resTok.ok) {
        setTokens(await resTok.json());
      }
      if (resMedias.ok) {
        const allMedias: Media[] = await resMedias.json();
        setMediaImages(allMedias.filter((m) => m.media_type === 'image' && m.url));
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
    <div className="max-w-2xl mx-auto px-4 sm:px-6 py-8 pb-28 flex flex-col gap-8">
      {/* Header */}
      <div className="flex items-baseline justify-between border-b border-[var(--border)] pb-4">
        <div>
          <h1 className="text-2xl sm:text-3xl font-serif font-bold text-[var(--text)] tracking-tight">Settings</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Configure appearance, preferences, and API credentials.
          </p>
        </div>

        <button
          onClick={handleLogout}
          className="flex items-center gap-1.5 text-xs text-[var(--danger-text)] hover:underline transition font-medium cursor-pointer"
        >
          <LogOut className="w-3.5 h-3.5" />
          <span>Log out</span>
        </button>
      </div>

      {/* Section: Appearance */}
      <section className="flex flex-col gap-4">
        <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--muted)] flex items-center gap-1.5">
          <Palette className="w-3.5 h-3.5 text-[var(--accent-text)]" />
          <span>Appearance</span>
        </div>

        <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--border)]/60">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3">
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

          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3">
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
        </div>
      </section>

      {/* Section: App Wallpaper & Dynamic Colors */}
      <section className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--muted)] flex items-center gap-1.5">
            <ImageIcon className="w-3.5 h-3.5 text-[var(--accent-text)]" />
            <span>Wallpaper & Dynamic Colors</span>
          </div>

          {wallpaperUrl && (
            <button
              onClick={clearWallpaper}
              className="text-xs text-[var(--danger-text)] hover:underline transition cursor-pointer"
            >
              Remove wallpaper
            </button>
          )}
        </div>

        {/* Dynamic color toggle */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3 rounded-[var(--radius-control)] bg-[var(--surface-hover)]/60 border border-[var(--border)]">
          <div className="flex items-center gap-2.5">
            <Sparkles className="w-4 h-4 text-[var(--accent-text)] shrink-0" />
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Harmonize Theme with Wallpaper</div>
              <div className="text-[11px] text-[var(--muted)]">
                Extracts dominant color and calms background/surfaces
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3">
            {dynamicColor && extractedColor && (
              <div className="flex items-center gap-1.5 text-xs text-[var(--text)]">
                <span
                  className="w-3.5 h-3.5 rounded-full border border-[var(--border)] shadow-xs"
                  style={{ backgroundColor: extractedColor }}
                />
                <span className="font-mono text-[11px] text-[var(--muted)]">{extractedColor}</span>
              </div>
            )}
            <Checkbox
              checked={dynamicColor}
              onCheckedChange={(c) => setDynamicColor(!!c)}
              label="Enable"
            />
          </div>
        </div>

        {/* Wallpaper Picker */}
        <div className="flex flex-col gap-2.5 pt-1">
          <div className="flex items-center justify-between text-xs">
            <span className="text-[11px] text-[var(--muted)] font-medium">Choose an image</span>
            <input
              type="file"
              ref={fileInputRef}
              accept="image/*"
              className="hidden"
              onChange={handleUploadWallpaper}
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => fileInputRef.current?.click()}
              className="h-6 text-xs px-2"
            >
              <Upload className="w-3 h-3" />
              <span>Upload Image...</span>
            </Button>
          </div>

          <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
            <button
              type="button"
              onClick={clearWallpaper}
              className={`h-16 rounded-[var(--radius-control)] border flex flex-col items-center justify-center gap-1 text-xs transition cursor-pointer ${
                !wallpaperUrl
                  ? 'border-[var(--accent-text)] bg-[var(--accent-tint)] font-semibold text-[var(--accent-text)]'
                  : 'border-[var(--border)] bg-[var(--surface-hover)]/40 text-[var(--muted)] hover:text-[var(--text)]'
              }`}
            >
              <X className="w-3.5 h-3.5" />
              <span className="text-[11px]">None</span>
            </button>

            {mediaImages.map((m) => {
              const isSelected = wallpaperUrl === m.url;
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => m.url && setWallpaper(m.url)}
                  className={`h-16 rounded-[var(--radius-control)] border relative overflow-hidden group cursor-pointer transition ${
                    isSelected
                      ? 'border-[var(--accent-text)] ring-2 ring-[var(--accent-text)]/40 shadow-xs'
                      : 'border-[var(--border)] hover:opacity-90'
                  }`}
                  title={m.name}
                >
                  <img
                    src={m.url}
                    alt={m.name}
                    className="w-full h-full object-cover"
                    loading="lazy"
                  />
                  {isSelected && (
                    <div className="absolute inset-0 bg-black/40 flex items-center justify-center">
                      <div className="p-0.5 rounded-full bg-[var(--accent)] text-white">
                        <Check className="w-3 h-3 stroke-[3]" />
                      </div>
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      </section>

      {/* Section: Timer & Defaults */}
      <section className="flex flex-col gap-4">
        <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--muted)] flex items-center gap-1.5">
          <Timer className="w-3.5 h-3.5 text-[var(--accent-text)]" />
          <span>Timer & Preferences</span>
        </div>

        <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--border)]/60">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Default Task Estimate</div>
              <div className="text-[11px] text-[var(--muted)]">Pre-filled when creating new tasks</div>
            </div>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5">
                <Input
                  type="number"
                  min="1"
                  disabled={noDefaultEstimate}
                  value={defaultEstimate}
                  onChange={(e) => handleEstimateChange(e.target.value)}
                  placeholder="25"
                  className="w-16 h-7 text-center text-xs"
                />
                <span className="text-xs text-[var(--muted)]">min</span>
              </div>
              <Checkbox
                checked={noDefaultEstimate}
                onCheckedChange={(c) => handleNoEstimateToggle(!!c)}
                label="No estimate"
              />
            </div>
          </div>

          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Server Timezone</div>
              <div className="text-[11px] text-[var(--muted)]">Current: <strong>{timezone}</strong></div>
            </div>
            <form onSubmit={handleSaveTimezone} className="flex gap-1.5">
              <Input
                required
                value={newTimezone}
                onChange={(e) => setNewTimezone(e.target.value)}
                placeholder="e.g. UTC"
                className="w-36 h-7 text-xs"
              />
              <Button type="submit" variant="secondary" size="sm" className="h-7 text-xs px-2">
                Save
              </Button>
            </form>
          </div>

          <div className="flex items-center justify-between gap-3 py-3">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Browser Notifications</div>
              <div className="text-[11px] text-[var(--muted)]">Status: <strong className="capitalize">{notifPermission}</strong></div>
            </div>
            {notifPermission !== 'granted' ? (
              <Button variant="secondary" size="sm" onClick={handleRequestNotifications} className="h-7 text-xs px-2.5">
                Enable
              </Button>
            ) : (
              <Badge variant="success">Enabled</Badge>
            )}
          </div>
        </div>
      </section>

      {/* Section: API Tokens */}
      <section className="flex flex-col gap-4">
        <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--muted)] flex items-center gap-1.5">
          <Key className="w-3.5 h-3.5 text-[var(--accent-text)]" />
          <span>API Tokens</span>
        </div>

        {/* Created secret banner */}
        {createdSecret && (
          <div className="p-3 bg-[var(--warning-bg)] border border-[var(--warning)]/30 rounded-[var(--radius-control)] text-xs">
            <div className="font-bold text-[var(--text)] mb-1">
              Copy this token now. It won't be shown again.
            </div>
            <div className="flex items-center gap-2 mt-1.5">
              <code className="flex-1 p-1.5 bg-[var(--surface)] border border-[var(--border)] rounded font-mono text-xs select-all break-all text-[var(--text)]">
                {createdSecret}
              </code>
              <Button variant="primary" size="sm" onClick={handleCopySecret} className="h-7 text-xs">
                {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copied ? 'Copied' : 'Copy'}</span>
              </Button>
            </div>
          </div>
        )}

        {/* Create new token form */}
        <form onSubmit={handleCreateToken} className="flex gap-2 max-w-sm">
          <Input
            required
            placeholder="Token name (e.g. laptop-cli)"
            value={newTokenName}
            onChange={(e) => setNewTokenName(e.target.value)}
            className="flex-1 h-7 text-xs"
          />
          <Button type="submit" variant="primary" size="sm" className="h-7 text-xs px-2.5">
            <Plus className="w-3 h-3 stroke-[3]" />
            <span>Create</span>
          </Button>
        </form>

        {/* Existing tokens list */}
        <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--border)]/60 text-xs">
          {tokens.map((t) => (
            <div key={t.id} className="py-2.5 px-1 flex items-center justify-between gap-3">
              <div>
                <div className="font-semibold text-[var(--text)]">{t.name}</div>
                <div className="text-[11px] text-[var(--muted)]">
                  Created: {new Date(t.created_at).toLocaleDateString()}
                  {t.last_used_at && ` • Last used: ${new Date(t.last_used_at).toLocaleDateString()}`}
                </div>
              </div>

              {!t.revoked_at ? (
                <div className="flex items-center gap-1.5">
                  <Button variant="ghost" size="sm" onClick={() => handleRotateToken(t.name)} className="h-6 text-[11px] px-2 text-[var(--muted)] hover:text-[var(--text)]">
                    Rotate
                  </Button>
                  <IconButton variant="ghost" size="sm" className="h-6 w-6 p-0 text-[var(--muted)] hover:text-[var(--danger-text)]" onClick={() => handleRevokeToken(t.name)} aria-label={`Revoke token ${t.name}`}>
                    <Trash2 className="w-3.5 h-3.5" />
                  </IconButton>
                </div>
              ) : (
                <Badge variant="muted">Revoked</Badge>
              )}
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};

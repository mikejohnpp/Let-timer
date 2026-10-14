import React, { useState, useEffect } from 'react';
import {
  LogOut,
  Plus,
  Trash2,
  Copy,
  Check,
  X,
} from 'lucide-react';
import type { components } from '../api/schema';
import { setStoredToken, apiFetch } from '../api/client';
import { useWallpaper } from '../hooks/useWallpaper';
import { useMotionMode, type MotionMode } from '../motion/useMotionMode';
import { withThemeTransition } from '../utils/themeTransition';
import {
  Button,
  IconButton,
  SegmentedControl,
  Checkbox,
  Input,
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
  const { motionMode, setMotionMode } = useMotionMode();
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

  const [autoRotateBg, setAutoRotateBg] = useState<boolean>(() => {
    const val = localStorage.getItem('let_timer_auto_rotate_bg');
    return val === null ? true : val === 'true';
  });

  const handleAutoRotateBgChange = (checked: boolean) => {
    setAutoRotateBg(checked);
    localStorage.setItem('let_timer_auto_rotate_bg', String(checked));
    window.dispatchEvent(new CustomEvent('let-timer-settings-changed'));
  };

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
    withThemeTransition(() => {
      if (next === 'system') {
        document.documentElement.removeAttribute('data-theme');
      } else {
        document.documentElement.setAttribute('data-theme', next);
      }
    });
  };

  const handlePaletteChange = (p: string) => {
    const next = p as 'default' | 'forest';
    setPalette(next);
    localStorage.setItem('let_timer_palette', next);
    withThemeTransition(() => {
      if (next === 'default') {
        document.documentElement.removeAttribute('data-palette');
      } else {
        document.documentElement.setAttribute('data-palette', next);
      }
    });
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

  const handleUploadWallpaper = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      const formData = new FormData();
      formData.append('file', file);
      formData.append('name', file.name.replace(/\.[^/.]+$/, ''));
      formData.append('media_type', 'image');
      formData.append('description', 'Uploaded wallpaper');

      const res = await apiFetch('/api/v1/media/upload', {
        method: 'POST',
        body: formData,
      });

      if (!res.ok) {
        throw new Error('Failed to upload wallpaper file');
      }

      const savedMedia = await res.json();
      setMediaImages((prev) => [savedMedia, ...prev]);
      await setWallpaper(savedMedia.url);
      showToast('Wallpaper uploaded & applied successfully', 'success');
    } catch {
      showToast('Failed to apply wallpaper', 'danger');
    } finally {
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const loadSettingsAndTokens = async () => {
    try {
      const [resTz, resTok, resMedias] = await Promise.all([
        apiFetch('/api/v1/settings/timezone'),
        apiFetch('/api/v1/auth/tokens'),
        apiFetch('/api/v1/medias'),
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
      const res = await apiFetch('/api/v1/settings/timezone', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
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
      const res = await apiFetch('/api/v1/auth/tokens', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
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
      const res = await apiFetch('/api/v1/auth/tokens/rotate', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
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
      const res = await apiFetch('/api/v1/auth/tokens/revoke', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
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
      <div className="flex items-baseline justify-between border-b border-[var(--border)]/30 pb-4">
        <div>
          <h1 className="text-2xl sm:text-3xl font-serif font-bold text-[var(--text)] tracking-tight">Settings</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Configure appearance, preferences, and API credentials
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
      <section className="flex flex-col gap-3">
        <div className="text-[10px] font-semibold uppercase tracking-widest text-[var(--muted)] px-1">
          Appearance
        </div>

        <div className="flex flex-col divide-y divide-[var(--border)]/20 border-y border-[var(--border)]/20">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
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

          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Color Palette</div>
              <div className="text-[11px] text-[var(--muted)]">Accent and focus tones</div>
            </div>
            <SegmentedControl
              value={palette}
              onValueChange={handlePaletteChange}
              items={[
                { value: 'default', label: 'Indigo' },
                { value: 'forest', label: 'Forest' },
              ]}
            />
          </div>

          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Motion</div>
              <div className="text-[11px] text-[var(--muted)]">Calm animations and feedback transitions</div>
            </div>
            <SegmentedControl
              value={motionMode}
              onValueChange={(val) => setMotionMode(val as MotionMode)}
              items={[
                { value: 'system', label: 'System' },
                { value: 'reduced', label: 'Reduced' },
                { value: 'full', label: 'Full' },
              ]}
            />
          </div>
        </div>
      </section>

      {/* Section: Wallpaper & Tone */}
      <section className="flex flex-col gap-3">
        <div className="text-[10px] font-semibold uppercase tracking-widest text-[var(--muted)] px-1">
          Wallpaper & Tone
        </div>

        <div className="flex flex-col divide-y divide-[var(--border)]/20 border-y border-[var(--border)]/20">
          {/* Harmonize toggle */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Harmonize with Wallpaper</div>
              <div className="text-[11px] text-[var(--muted)]">
                Extract dominant tones to calm backgrounds and surfaces
              </div>
            </div>

            <div className="flex items-center gap-3">
              {dynamicColor && extractedColor && (
                <div className="flex items-center gap-1.5 text-xs text-[var(--text)]">
                  <span
                    className="w-3 h-3 rounded-full border border-[var(--border)]"
                    style={{ backgroundColor: extractedColor }}
                  />
                  <span className="font-mono text-[10px] text-[var(--muted)]">{extractedColor}</span>
                </div>
              )}
              <Checkbox
                checked={dynamicColor}
                onCheckedChange={(c) => setDynamicColor(!!c)}
                label="Enable"
              />
            </div>
          </div>

          {/* Auto-rotate Focus Backgrounds toggle */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Auto-rotate Focus Backgrounds</div>
              <div className="text-[11px] text-[var(--muted)]">
                Automatically rotate focus backgrounds every 90 seconds
              </div>
            </div>

            <div className="flex items-center gap-3">
              <Checkbox
                checked={autoRotateBg}
                onCheckedChange={(c) => handleAutoRotateBgChange(!!c)}
                label="Enable"
              />
            </div>
          </div>

          {/* Wallpaper Picker */}
          <div className="py-3 px-1 flex flex-col gap-2.5">
            <div className="flex items-center justify-between text-xs">
              <span className="text-xs font-semibold text-[var(--text)]">Wallpaper</span>
              <div className="flex items-center gap-3">
                {wallpaperUrl && (
                  <button
                    onClick={clearWallpaper}
                    className="text-xs text-[var(--danger-text)] hover:underline cursor-pointer"
                  >
                    Remove
                  </button>
                )}
                <button
                  onClick={() => fileInputRef.current?.click()}
                  className="text-xs text-[var(--accent-text)] hover:underline font-medium cursor-pointer"
                >
                  + Upload
                </button>
                <input
                  type="file"
                  ref={fileInputRef}
                  accept="image/*"
                  className="hidden"
                  onChange={handleUploadWallpaper}
                />
              </div>
            </div>

            <div className="grid grid-cols-4 sm:grid-cols-7 gap-2 pt-1">
              <button
                type="button"
                onClick={clearWallpaper}
                className={`h-14 rounded-lg border flex flex-col items-center justify-center gap-1 text-xs transition cursor-pointer ${
                  !wallpaperUrl
                    ? 'border-[var(--accent-text)] bg-[var(--accent)]/10 font-medium text-[var(--accent-text)]'
                    : 'border-[var(--border)]/40 hover:bg-[var(--surface-hover)] text-[var(--muted)] hover:text-[var(--text)]'
                }`}
              >
                <X className="w-3.5 h-3.5" />
                <span className="text-[10px]">None</span>
              </button>

              {mediaImages.map((m) => {
                const isSelected = wallpaperUrl === m.url;
                return (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => m.url && setWallpaper(m.url)}
                    className={`h-14 rounded-lg border relative overflow-hidden group cursor-pointer transition ${
                      isSelected
                        ? 'border-[var(--accent-text)] ring-2 ring-[var(--accent-text)]/40'
                        : 'border-[var(--border)]/40 hover:opacity-90'
                    }`}
                    title={m.name}
                  >
                    <img src={m.url} alt={m.name} className="w-full h-full object-cover" loading="lazy" />
                    {isSelected && (
                      <div className="absolute inset-0 bg-black/40 flex items-center justify-center">
                        <Check className="w-3.5 h-3.5 text-white stroke-[3]" />
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </section>

      {/* Section: Preferences */}
      <section className="flex flex-col gap-3">
        <div className="text-[10px] font-semibold uppercase tracking-widest text-[var(--muted)] px-1">
          Preferences
        </div>

        <div className="flex flex-col divide-y divide-[var(--border)]/20 border-y border-[var(--border)]/20">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
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
                  className="w-16 h-7 text-center text-xs bg-[var(--surface-hover)]/40 border-[var(--border)]/40"
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

          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 py-3 px-1">
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
                className="w-32 h-7 text-xs bg-[var(--surface-hover)]/40 border-[var(--border)]/40"
              />
              <Button type="submit" variant="ghost" size="sm" className="h-7 text-xs px-2 font-medium text-[var(--accent-text)]">
                Save
              </Button>
            </form>
          </div>

          <div className="flex items-center justify-between gap-3 py-3 px-1">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Browser Notifications</div>
              <div className="text-[11px] text-[var(--muted)]">Status: <strong className="capitalize">{notifPermission}</strong></div>
            </div>
            {notifPermission !== 'granted' ? (
              <Button variant="ghost" size="sm" onClick={handleRequestNotifications} className="h-7 text-xs px-2.5 font-medium text-[var(--accent-text)]">
                Enable
              </Button>
            ) : (
              <span className="text-[11px] text-[var(--success-text)] font-medium">Enabled</span>
            )}
          </div>
        </div>
      </section>

      {/* Section: API Tokens */}
      <section className="flex flex-col gap-3">
        <div className="text-[10px] font-semibold uppercase tracking-widest text-[var(--muted)] px-1">
          API Tokens
        </div>

        {/* Created secret banner */}
        {createdSecret && (
          <div className="p-3 bg-[var(--warning-bg)]/80 border border-[var(--warning)]/30 rounded-lg text-xs backdrop-blur-sm">
            <div className="font-semibold text-[var(--text)] mb-1">
              Copy this token now. It won't be shown again.
            </div>
            <div className="flex items-center gap-2 mt-1.5">
              <code className="flex-1 p-1.5 bg-[var(--surface)] border border-[var(--border)] rounded font-mono text-xs select-all break-all text-[var(--text)]">
                {createdSecret}
              </code>
              <Button variant="ghost" size="sm" onClick={handleCopySecret} className="h-7 text-xs font-semibold text-[var(--accent-text)]">
                {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copied ? 'Copied' : 'Copy'}</span>
              </Button>
            </div>
          </div>
        )}

        {/* Existing tokens list */}
        <div className="flex flex-col divide-y divide-[var(--border)]/20 border-y border-[var(--border)]/20 text-xs">
          {tokens.map((t) => (
            <div key={t.id} className="py-2.5 px-1 flex items-center justify-between gap-3 group">
              <div>
                <div className="font-medium text-[var(--text)]">{t.name}</div>
                <div className="text-[11px] text-[var(--muted)]">
                  Created {new Date(t.created_at).toLocaleDateString()}
                  {t.last_used_at && ` • Last used ${new Date(t.last_used_at).toLocaleDateString()}`}
                </div>
              </div>

              {!t.revoked_at ? (
                <div className="flex items-center gap-1 opacity-80 group-hover:opacity-100 transition-opacity">
                  <button
                    onClick={() => handleRotateToken(t.name)}
                    className="text-[11px] px-2 py-0.5 rounded text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition cursor-pointer"
                  >
                    Rotate
                  </button>
                  <IconButton
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0 text-[var(--muted)] hover:text-[var(--danger-text)]"
                    onClick={() => handleRevokeToken(t.name)}
                    aria-label={`Revoke token ${t.name}`}
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </IconButton>
                </div>
              ) : (
                <span className="text-[10px] text-[var(--muted)] bg-[var(--surface-hover)] px-2 py-0.5 rounded">
                  Revoked
                </span>
              )}
            </div>
          ))}
        </div>

        {/* Create new token form */}
        <form onSubmit={handleCreateToken} className="flex gap-2 max-w-sm pt-1 px-1">
          <Input
            required
            placeholder="Token name (e.g. laptop-cli)"
            value={newTokenName}
            onChange={(e) => setNewTokenName(e.target.value)}
            className="flex-1 h-7 text-xs bg-[var(--surface-hover)]/40 border-[var(--border)]/40"
          />
          <Button type="submit" variant="ghost" size="sm" className="h-7 text-xs px-2.5 font-semibold text-[var(--accent-text)]">
            <Plus className="w-3 h-3 stroke-[3]" />
            <span>Create</span>
          </Button>
        </form>
      </section>
    </div>
  );
};

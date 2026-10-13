import React, { useState, useEffect } from 'react';
import { Clock, Moon, Sun, Monitor, Folder } from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import type { components } from '../api/schema';
import type { AppView } from '../hooks/useHashRoute';
import { getStoredWorkspace, setStoredWorkspace } from '../api/client';
import { ConnectionBadge, IconButton } from './ui';
import { TimerBar } from './TimerBar';

type Workspace = components['schemas']['WorkspaceDto'];

interface HeaderProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  snapshot: TimerSnapshot | null;
  receivedAt?: number;
  onOpenFocus?: () => void;
  onActionComplete?: () => void;
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  onOpenStartPicker: () => void;
  className?: string;
}

export const Header: React.FC<HeaderProps> = ({
  currentView,
  onNavigate,
  snapshot,
  receivedAt,
  onOpenFocus,
  onActionComplete,
  connectionStatus,
  onOpenStartPicker,
  className,
}) => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string>(getStoredWorkspace() || '');
  const [theme, setTheme] = useState<'light' | 'dark' | 'system'>('system');

  useEffect(() => {
    const savedTheme = (localStorage.getItem('let_timer_theme') as 'light' | 'dark' | 'system') || 'system';
    setTheme(savedTheme);
    applyTheme(savedTheme);

    const savedPalette = localStorage.getItem('let_timer_palette') || 'default';
    if (savedPalette !== 'default') {
      document.documentElement.setAttribute('data-palette', savedPalette);
    }

    async function loadWorkspaces() {
      try {
        const token = localStorage.getItem('let_timer_token');
        const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
        if (token) headers['Authorization'] = `Bearer ${token}`;
        const res = await fetch('/api/v1/workspaces', { headers });
        if (res.ok) {
          const data: Workspace[] = await res.json();
          setWorkspaces(data);
          if (!activeWorkspaceId && data.length > 0) {
            setActiveWorkspaceId(data[0].id.toString());
          }
        }
      } catch (e) {
        console.error('Failed to load workspaces', e);
      }
    }

    loadWorkspaces();
    const handleWsChange = () => loadWorkspaces();
    window.addEventListener('let-timer-workspace-changed', handleWsChange);
    return () => window.removeEventListener('let-timer-workspace-changed', handleWsChange);
  }, [activeWorkspaceId]);

  const applyTheme = (t: 'light' | 'dark' | 'system') => {
    if (t === 'system') {
      document.documentElement.removeAttribute('data-theme');
    } else {
      document.documentElement.setAttribute('data-theme', t);
    }
  };

  const cycleTheme = () => {
    const next = theme === 'system' ? 'light' : theme === 'light' ? 'dark' : 'system';
    setTheme(next);
    localStorage.setItem('let_timer_theme', next);
    applyTheme(next);
  };

  const handleWorkspaceSelect = (id: string) => {
    setActiveWorkspaceId(id);
    setStoredWorkspace(id);
    window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
  };

  const isRunning = snapshot && snapshot.state === 'running';

  const navTabs: { view: AppView; label: string }[] = [
    { view: 'today', label: 'Today' },
    { view: 'focus', label: 'Focus' },
    { view: 'workspaces', label: 'Workspaces' },
    { view: 'media', label: 'Media' },
    { view: 'settings', label: 'Settings' },
  ];

  return (
    <header
      className={`border-b border-[var(--border)] bg-[var(--surface)] lg:sticky top-0 z-40 transition-colors h-[var(--header-h)] ${
        className || ''
      }`}
    >
      <div className="max-w-5xl mx-auto px-4 sm:px-6 h-full flex items-center justify-between gap-2 sm:gap-4">
        {/* Left: Brand & Workspace */}
        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
          <button
            onClick={() => onNavigate('today')}
            className="flex items-center gap-1.5 font-bold text-sm sm:text-base tracking-tight hover:opacity-80 transition cursor-pointer select-none"
          >
            <Clock
              className={`w-4 h-4 sm:w-5 sm:h-5 ${
                isRunning ? 'text-[var(--accent-text)] animate-pulse' : 'text-[var(--accent-text)]'
              }`}
            />
            <span className="text-[var(--text)] hidden sm:inline">Let-timer</span>
          </button>

          <span className="text-[var(--border-strong)]/40 hidden sm:inline text-xs">/</span>

          {/* Minimal Workspace dropdown */}
          <div className="flex items-center gap-1 text-xs">
            <Folder className="w-3.5 h-3.5 text-[var(--muted)]" />
            <select
              value={activeWorkspaceId}
              onChange={(e) => handleWorkspaceSelect(e.target.value)}
              className="bg-transparent text-[var(--muted)] hover:text-[var(--text)] font-medium outline-none cursor-pointer max-w-[90px] sm:max-w-[140px] truncate transition"
              aria-label="Active workspace"
            >
              {workspaces.map((w) => (
                <option key={w.id} value={w.id.toString()} className="bg-[var(--surface)] text-[var(--text)]">
                  {w.name}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Center: Sleek Timer Dock */}
        <div className="flex items-center justify-center min-w-0">
          <TimerBar
            snapshot={snapshot}
            receivedAt={receivedAt || Date.now()}
            onOpenFocus={onOpenFocus || (() => onNavigate('focus'))}
            onOpenStartPicker={onOpenStartPicker}
            onActionComplete={onActionComplete || (() => {})}
          />
        </div>

        {/* Right: Nav Tabs & Controls */}
        <div className="flex items-center gap-1 sm:gap-2 shrink-0">
          {/* Navigation Tabs (Only desktop >= 1024px lg) */}
          <nav className="hidden lg:flex items-center gap-0.5 text-xs font-medium mr-1">
            {navTabs.map((t) => {
              const isActive = currentView === t.view;
              return (
                <button
                  key={t.view}
                  onClick={() => onNavigate(t.view)}
                  aria-current={isActive ? 'page' : undefined}
                  className={`px-2.5 py-1 rounded-[var(--radius-control)] transition relative cursor-pointer ${
                    isActive
                      ? 'text-[var(--accent-text)] font-semibold'
                      : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
                  }`}
                >
                  <span>{t.label}</span>
                </button>
              );
            })}
          </nav>

          <ConnectionBadge
            status={
              connectionStatus === 'connected'
                ? 'connected'
                : connectionStatus === 'connecting'
                ? 'reconnecting'
                : 'disconnected'
            }
          />

          <IconButton
            variant="ghost"
            size="sm"
            onClick={cycleTheme}
            aria-label={`Cycle theme, currently ${theme}`}
            title={`Theme: ${theme}`}
          >
            {theme === 'dark' ? (
              <Moon className="w-4 h-4" />
            ) : theme === 'light' ? (
              <Sun className="w-4 h-4" />
            ) : (
              <Monitor className="w-4 h-4" />
            )}
          </IconButton>
        </div>
      </div>
    </header>
  );
};

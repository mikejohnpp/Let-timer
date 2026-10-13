import React, { useState, useEffect } from 'react';
import { Clock, Play, Moon, Sun, CheckCircle2, AlertCircle, RefreshCw, Folder } from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import type { components } from '../api/schema';
import { getStoredWorkspace, setStoredWorkspace } from '../api/client';

type Workspace = components['schemas']['WorkspaceDto'];

interface HeaderProps {
  currentView: 'today' | 'focus' | 'workspaces' | 'media' | 'settings';
  onNavigate: (view: 'today' | 'focus' | 'workspaces' | 'media' | 'settings') => void;
  snapshot: TimerSnapshot | null;
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  onOpenStartPicker: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  currentView,
  onNavigate,
  snapshot,
  connectionStatus,
  onOpenStartPicker,
}) => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string>(getStoredWorkspace() || '');
  const [theme, setTheme] = useState<'light' | 'dark'>('dark');

  useEffect(() => {
    // Initial theme
    const savedTheme = localStorage.getItem('let_timer_theme') as 'light' | 'dark' | null;
    if (savedTheme) {
      setTheme(savedTheme);
      document.documentElement.setAttribute('data-theme', savedTheme);
    } else {
      document.documentElement.setAttribute('data-theme', 'dark');
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

  const toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    localStorage.setItem('let_timer_theme', next);
    document.documentElement.setAttribute('data-theme', next);
  };

  const handleWorkspaceSelect = (id: string) => {
    setActiveWorkspaceId(id);
    setStoredWorkspace(id);
    window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
  };

  const isRunning = snapshot && snapshot.state === 'running';

  return (
    <header className="border-b border-[var(--border)] bg-[var(--surface)] sticky top-0 z-40 transition-colors">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between gap-4">
        {/* Brand & Workspace */}
        <div className="flex items-center gap-4">
          <button
            onClick={() => onNavigate('today')}
            className="flex items-center gap-2 font-bold text-lg tracking-tight hover:opacity-80 transition"
          >
            <Clock className={`w-6 h-6 ${isRunning ? 'text-[var(--timer-run)] animate-pulse' : 'text-[var(--accent)]'}`} />
            <span>Let-timer</span>
          </button>

          {/* Workspace dropdown */}
          <div className="flex items-center gap-1.5 bg-[var(--surface-hover)] border border-[var(--border)] px-2.5 py-1 rounded-lg text-sm">
            <Folder className="w-3.5 h-3.5 text-[var(--muted)]" />
            <select
              value={activeWorkspaceId}
              onChange={(e) => handleWorkspaceSelect(e.target.value)}
              className="bg-transparent text-[var(--text)] font-medium outline-none cursor-pointer text-xs sm:text-sm"
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

        {/* Navigation Tabs */}
        <nav className="hidden md:flex items-center gap-1 text-sm font-medium">
          <button
            onClick={() => onNavigate('today')}
            className={`px-3 py-1.5 rounded-lg transition ${
              currentView === 'today'
                ? 'bg-[var(--accent)] text-white'
                : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
            }`}
          >
            Today
          </button>
          <button
            onClick={() => onNavigate('focus')}
            className={`px-3 py-1.5 rounded-lg transition ${
              currentView === 'focus'
                ? 'bg-[var(--accent)] text-white'
                : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
            }`}
          >
            Focus
          </button>
          <button
            onClick={() => onNavigate('workspaces')}
            className={`px-3 py-1.5 rounded-lg transition ${
              currentView === 'workspaces'
                ? 'bg-[var(--accent)] text-white'
                : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
            }`}
          >
            Workspaces
          </button>
          <button
            onClick={() => onNavigate('media')}
            className={`px-3 py-1.5 rounded-lg transition ${
              currentView === 'media'
                ? 'bg-[var(--accent)] text-white'
                : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
            }`}
          >
            Media
          </button>
          <button
            onClick={() => onNavigate('settings')}
            className={`px-3 py-1.5 rounded-lg transition ${
              currentView === 'settings'
                ? 'bg-[var(--accent)] text-white'
                : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
            }`}
          >
            Settings
          </button>
        </nav>

        {/* Right Actions */}
        <div className="flex items-center gap-2">
          {(!snapshot || snapshot.state === 'idle') && (
            <button
              onClick={onOpenStartPicker}
              className="flex items-center gap-1.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-3 py-1.5 rounded-lg text-sm font-medium shadow-sm transition"
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span className="hidden sm:inline">Start...</span>
            </button>
          )}

          {/* Connection status indicator */}
          <div
            className="flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium border border-[var(--border)]"
            title={`Connection: ${connectionStatus}`}
          >
            {connectionStatus === 'connected' && (
              <>
                <CheckCircle2 className="w-3.5 h-3.5 text-[var(--success)]" />
                <span className="hidden lg:inline text-[var(--muted)]">Live</span>
              </>
            )}
            {connectionStatus === 'connecting' && (
              <>
                <RefreshCw className="w-3.5 h-3.5 text-[var(--warning)] animate-spin" />
                <span className="hidden lg:inline text-[var(--muted)]">Syncing</span>
              </>
            )}
            {connectionStatus === 'disconnected' && (
              <>
                <AlertCircle className="w-3.5 h-3.5 text-[var(--danger)]" />
                <span className="hidden lg:inline text-[var(--danger)]">Offline</span>
              </>
            )}
          </div>

          {/* Theme switch */}
          <button
            onClick={toggleTheme}
            className="p-2 text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-lg transition"
            aria-label="Toggle theme"
          >
            {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
          </button>
        </div>
      </div>
    </header>
  );
};

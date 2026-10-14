import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Clock, Moon, Sun, Monitor, ChevronDown, Plus, Trash2 } from 'lucide-react';
import type { components } from '../api/schema';
import type { AppView } from '../hooks/useHashRoute';
import { getStoredWorkspace, setStoredWorkspace, apiFetch } from '../api/client';
import { withThemeTransition } from '../utils/themeTransition';
import { motion, AnimatePresence } from 'motion/react';
import { useMotionTransition } from '../motion';
import {
  ConnectionBadge,
  IconButton,
  Button,
  Popover,
  PopoverTrigger,
  PopoverContent,
  useToast,
  useConfirm,
} from './ui';

type Workspace = components['schemas']['WorkspaceDto'];

interface HeaderProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  className?: string;
}

export const Header: React.FC<HeaderProps> = ({
  currentView,
  onNavigate,
  connectionStatus,
  className,
}) => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string>(getStoredWorkspace() || '');
  const [theme, setTheme] = useState<'light' | 'dark' | 'system'>('system');
  const [wsPopoverOpen, setWsPopoverOpen] = useState(false);
  const [newWsName, setNewWsName] = useState('');
  const [isCreatingWs, setIsCreatingWs] = useState(false);
  const [focusVisible, setFocusVisible] = useState(true);
  const hideTimerRef = useRef<number | null>(null);

  const { showToast } = useToast();
  const confirm = useConfirm();
  const wsItemTransition = useMotionTransition('base');

  const isFocusView = currentView === 'focus';

  const resetHideTimer = useCallback(() => {
    setFocusVisible(true);
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(() => {
      if (!wsPopoverOpen) {
        setFocusVisible(false);
      }
    }, 3000);
  }, [wsPopoverOpen]);

  useEffect(() => {
    if (!isFocusView) {
      setFocusVisible(true);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
      return;
    }

    const handleActivity = () => resetHideTimer();
    window.addEventListener('mousemove', handleActivity);
    window.addEventListener('keydown', handleActivity);
    window.addEventListener('touchstart', handleActivity);
    resetHideTimer();

    return () => {
      window.removeEventListener('mousemove', handleActivity);
      window.removeEventListener('keydown', handleActivity);
      window.removeEventListener('touchstart', handleActivity);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
  }, [isFocusView, resetHideTimer]);

  useEffect(() => {
    if (wsPopoverOpen) {
      setFocusVisible(true);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    } else if (isFocusView) {
      resetHideTimer();
    }
  }, [wsPopoverOpen, isFocusView, resetHideTimer]);

  const loadWorkspaces = async () => {
    try {
      const res = await apiFetch('/api/v1/workspaces');
      if (res.ok) {
        const data: Workspace[] = await res.json();
        setWorkspaces(data);
        const stored = getStoredWorkspace();
        const validStored = stored && data.some((w) => w.id.toString() === stored);
        if (validStored) {
          if (activeWorkspaceId !== stored) {
            setActiveWorkspaceId(stored);
          }
        } else {
          let targetId = '';
          const curRes = await apiFetch('/api/v1/workspaces/current').catch(() => null);
          if (curRes && curRes.ok) {
            const curWs: Workspace = await curRes.json();
            targetId = curWs.id.toString();
          } else if (data.length > 0) {
            targetId = data[0].id.toString();
          }
          if (targetId) {
            setActiveWorkspaceId(targetId);
            setStoredWorkspace(targetId);
          }
        }
      }
    } catch (e) {
      console.error('Failed to load workspaces', e);
    }
  };

  useEffect(() => {
    const savedTheme = (localStorage.getItem('let_timer_theme') as 'light' | 'dark' | 'system') || 'system';
    setTheme(savedTheme);
    applyTheme(savedTheme);

    const savedPalette = localStorage.getItem('let_timer_palette') || 'default';
    if (savedPalette !== 'default') {
      document.documentElement.setAttribute('data-palette', savedPalette);
    }

    loadWorkspaces();
    const handleWsChange = () => loadWorkspaces();
    window.addEventListener('let-timer-workspace-changed', handleWsChange);
    return () => window.removeEventListener('let-timer-workspace-changed', handleWsChange);
  }, []);

  const applyTheme = (t: 'light' | 'dark' | 'system') => {
    withThemeTransition(() => {
      if (t === 'system') {
        document.documentElement.removeAttribute('data-theme');
      } else {
        document.documentElement.setAttribute('data-theme', t);
      }
    });
  };

  const cycleTheme = () => {
    const next = theme === 'system' ? 'light' : theme === 'light' ? 'dark' : 'system';
    setTheme(next);
    localStorage.setItem('let_timer_theme', next);
    applyTheme(next);
  };

  const handleWorkspaceSelect = async (id: string) => {
    setActiveWorkspaceId(id);
    setStoredWorkspace(id);
    setWsPopoverOpen(false);
    try {
      await apiFetch('/api/v1/workspaces/switch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: Number(id) }),
      });
    } catch (e) {
      console.error('Failed to sync switch with daemon', e);
    }
    window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
    showToast('Workspace switched', 'info');
  };

  const handleCreateWorkspace = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newWsName.trim() || isCreatingWs) return;

    setIsCreatingWs(true);
    try {
      const res = await apiFetch('/api/v1/workspaces', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ name: newWsName.trim() }),
      });

      if (res.ok) {
        const created: Workspace = await res.json();
        setNewWsName('');
        showToast('Workspace created', 'success');
        await loadWorkspaces();
        await handleWorkspaceSelect(created.id.toString());
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to create workspace', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Error creating workspace', 'danger');
    } finally {
      setIsCreatingWs(false);
    }
  };

  const handleDeleteWorkspace = async (id: number, name: string) => {
    const ok = await confirm({
      title: `Delete workspace "${name}"?`,
      body: 'Tasks inside this workspace will be deleted as well. This cannot be undone.',
      tone: 'danger',
      confirmLabel: 'Delete workspace',
    });
    if (!ok) return;

    try {
      const res = await apiFetch(`/api/v1/workspaces/${id}`, { method: 'DELETE' });
      if (res.ok) {
        showToast(`Workspace "${name}" deleted`, 'info');
        const remaining = workspaces.filter((w) => w.id !== id);
        setWorkspaces(remaining);
        if (activeWorkspaceId === id.toString() && remaining.length > 0) {
          await handleWorkspaceSelect(remaining[0].id.toString());
        }
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to delete workspace', 'danger');
    }
  };

  const activeWs = workspaces.find((w) => w.id.toString() === activeWorkspaceId);
  const activeWsName = activeWs ? activeWs.name : 'Workspace';

  const navTabs: { view: AppView; label: string }[] = [
    { view: 'today', label: 'Today' },
    { view: 'focus', label: 'Focus' },
    { view: 'media', label: 'Media' },
    { view: 'settings', label: 'Settings' },
  ];

  return (
    <header
      className={`sticky top-0 z-40 w-full border-none transition-opacity duration-[var(--dur-fast)] ease-[var(--ease-out)] h-[var(--header-h)] ${
        isFocusView
          ? `bg-transparent focus-within:opacity-100 focus-within:pointer-events-auto ${
              focusVisible
                ? 'opacity-100'
                : 'opacity-0 pointer-events-none'
            }`
          : 'bg-[var(--bg)]/80 backdrop-blur-md opacity-100'
      } ${className || ''}`}
    >
      <div className="relative w-full px-4 sm:px-8 h-full flex items-center justify-between gap-4">
        {/* Left: Brand & Custom Workspace Popover */}
        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
          <button
            onClick={() => onNavigate('today')}
            className="flex items-center gap-1.5 font-bold text-sm sm:text-base tracking-tight hover:opacity-80 transition cursor-pointer select-none"
          >
            <Clock className="w-4 h-4 sm:w-5 sm:h-5 text-[var(--accent-text)]" />
            <span className="text-[var(--text)] hidden sm:inline">Let-timer</span>
          </button>

          <span className="text-[var(--border-strong)]/40 hidden sm:inline text-xs">/</span>

          {/* Workspace Popover Trigger */}
          <Popover open={wsPopoverOpen} onOpenChange={setWsPopoverOpen}>
            <PopoverTrigger asChild>
              <button
                className="flex items-center gap-1 text-xs font-medium text-[var(--muted)] hover:text-[var(--text)] transition cursor-pointer px-1.5 py-0.5 rounded-[var(--radius-control)] hover:bg-[var(--surface-hover)] select-none"
                aria-label="Active workspace selector"
              >
                <span className="max-w-[120px] sm:max-w-[160px] truncate">{activeWsName}</span>
                <ChevronDown className="w-3 h-3 text-[var(--muted)]" />
              </button>
            </PopoverTrigger>
            <PopoverContent align="start" className="w-64 p-3 flex flex-col gap-2">
              <div className="text-[10px] font-bold uppercase tracking-wider text-[var(--muted)] px-1">
                Workspaces
              </div>

              {/* Workspaces list */}
              <div className="flex flex-col divide-y divide-[var(--border)]/40 max-h-48 overflow-y-auto">
                <AnimatePresence initial={false}>
                  {workspaces.map((w) => {
                    const isActive = w.id.toString() === activeWorkspaceId;
                    return (
                      <motion.div
                        key={w.id}
                        layout
                        initial={{ opacity: 0, y: 4, scale: 0.98 }}
                        animate={{ opacity: 1, y: 0, scale: 1 }}
                        exit={{ opacity: 0, height: 0, scale: 0.95, overflow: 'hidden' }}
                        transition={wsItemTransition}
                        onClick={() => handleWorkspaceSelect(w.id.toString())}
                        className={`py-1.5 px-1.5 flex items-center justify-between gap-2 rounded cursor-pointer transition group ${
                          isActive
                            ? 'bg-[var(--accent)]/10 text-[var(--accent-text)] font-semibold'
                            : 'text-[var(--text)] hover:bg-[var(--surface-hover)]'
                        }`}
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <span
                            className={`w-2 h-2 rounded-full shrink-0 ${
                              isActive ? 'bg-[var(--accent-text)]' : 'border border-[var(--muted)]'
                            }`}
                          />
                          <span className="text-xs truncate">{w.name}</span>
                        </div>

                        {!isActive && (
                          <IconButton
                            variant="ghost"
                            size="sm"
                            className="h-5 w-5 p-0 opacity-0 group-hover:opacity-100 text-[var(--muted)] hover:text-[var(--danger-text)]"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDeleteWorkspace(w.id, w.name);
                            }}
                            aria-label={`Delete ${w.name}`}
                            title={`Delete ${w.name}`}
                          >
                            <Trash2 className="w-3 h-3" />
                          </IconButton>
                        )}
                      </motion.div>
                    );
                  })}
                </AnimatePresence>
              </div>

              {/* Inline Create Form */}
              <form onSubmit={handleCreateWorkspace} className="flex gap-1.5 pt-2 border-t border-[var(--border)]">
                <input
                  type="text"
                  required
                  placeholder="New workspace..."
                  value={newWsName}
                  onChange={(e) => setNewWsName(e.target.value)}
                  className="flex-1 min-w-0 bg-[var(--surface-hover)] text-xs px-2 py-1 rounded outline-none border border-transparent focus:border-[var(--accent-text)] text-[var(--text)] placeholder-[var(--muted)]"
                />
                <Button
                  type="submit"
                  variant="ghost"
                  size="sm"
                  disabled={!newWsName.trim() || isCreatingWs}
                  className="h-6 text-xs px-2 text-[var(--accent-text)] font-semibold"
                >
                  <Plus className="w-3 h-3" />
                  <span>Add</span>
                </Button>
              </form>
            </PopoverContent>
          </Popover>
        </div>

        {/* Center: Navigation Tabs (Desktop only; on mobile bottom tab bar is used) */}
        <div className="hidden lg:flex items-center justify-center absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 pointer-events-auto">
          <nav className="flex items-center gap-1 text-xs font-medium">
            {navTabs.map((t) => {
              const isActive = currentView === t.view;
              return (
                <button
                  key={t.view}
                  onClick={() => onNavigate(t.view)}
                  aria-current={isActive ? 'page' : undefined}
                  className={`px-3 py-1 rounded-full transition relative cursor-pointer ${
                    isActive
                      ? 'text-[var(--accent-text)] font-semibold bg-[var(--accent)]/10'
                      : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
                  }`}
                >
                  <span>{t.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        {/* Right: Borderless Connection & Theme Toggle */}
        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
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
            className="h-7 w-7 p-0"
          >
            {theme === 'dark' ? (
              <Moon className="w-3.5 h-3.5" />
            ) : theme === 'light' ? (
              <Sun className="w-3.5 h-3.5" />
            ) : (
              <Monitor className="w-3.5 h-3.5" />
            )}
          </IconButton>
        </div>
      </div>
    </header>
  );
};

export default Header;

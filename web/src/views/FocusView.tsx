import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  Play,
  Pause,
  Check,
  Plus,
  Music,
  ChevronDown,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { apiFetch } from '../api/client';
import { useToast, TimerDigits, Popover, PopoverTrigger, PopoverContent } from '../components/ui';
import { useAudioPlayer } from '../context/AudioPlayerContext';
import { FloatingPlayerIsland } from '../components/FloatingPlayerIsland';

interface FocusViewProps {
  snapshot: TimerSnapshot | null;
  receivedAt: number;
  onRefreshTimer: () => void;
  onOpenStartPicker: () => void;
}

export const FocusView: React.FC<FocusViewProps> = ({
  snapshot,
  receivedAt,
  onRefreshTimer,
  onOpenStartPicker,
}) => {
  const [now, setNow] = useState<number>(Date.now());
  const [controlsVisible, setControlsVisible] = useState(true);
  const [mediaLists, setMediaLists] = useState<{ id: number; name: string }[]>([]);
  const [selectedMediaListId, setSelectedMediaListId] = useState<number | null>(null);
  const [playlistPopoverOpen, setPlaylistPopoverOpen] = useState(false);

  const { showToast } = useToast();
  const hideTimerRef = useRef<number | null>(null);
  const {
    mediaListDetail,
    loadMediaListForTask,
    clearPlayer,
    currentImageIndex,
    setCurrentImageIndex,
  } = useAudioPlayer();

  // Fetch available media lists for idle ambient playback
  useEffect(() => {
    apiFetch('/api/v1/media-lists')
      .then((res) => (res.ok ? res.json() : []))
      .then((data: { id: number; name: string }[]) => {
        setMediaLists(data);
      })
      .catch(() => {});
  }, []);

  const [activeWorkspaceName, setActiveWorkspaceName] = useState<string>('');

  const loadActiveWorkspace = useCallback(async () => {
    try {
      const res = await apiFetch('/api/v1/workspaces/current');
      if (res.ok) {
        const data = await res.json();
        if (data?.name) {
          setActiveWorkspaceName(data.name);
          return;
        }
      }
      const listRes = await apiFetch('/api/v1/workspaces');
      if (listRes.ok) {
        const list = await listRes.json();
        const stored = typeof localStorage !== 'undefined' ? localStorage.getItem('let_timer_active_workspace') : null;
        const found = stored ? list.find((w: { id: number; name: string }) => w.id.toString() === stored) : list[0];
        if (found?.name) {
          setActiveWorkspaceName(found.name);
        }
      }
    } catch {
      // Ignore network errors
    }
  }, []);

  useEffect(() => {
    loadActiveWorkspace();
    window.addEventListener('let-timer-workspace-changed', loadActiveWorkspace);
    return () => {
      window.removeEventListener('let-timer-workspace-changed', loadActiveWorkspace);
    };
  }, [loadActiveWorkspace]);

  const currentTaskId = snapshot?.task?.id;
  const currentMediaListId = snapshot?.task?.media_list_id || null;
  const prevTaskIdRef = useRef<number | null | undefined>(undefined);

  // When task changes or starts: sync task's media list or clear player if none
  useEffect(() => {
    const prevTaskId = prevTaskIdRef.current;
    prevTaskIdRef.current = currentTaskId;

    if (!currentTaskId || snapshot?.state === 'idle') {
      return;
    }

    if (prevTaskId !== currentTaskId) {
      if (currentMediaListId) {
        setSelectedMediaListId(currentMediaListId);
        loadMediaListForTask(currentMediaListId);
      } else {
        setSelectedMediaListId(null);
        clearPlayer();
      }
    }
  }, [currentTaskId, currentMediaListId, snapshot?.state, loadMediaListForTask, clearPlayer]);

  // Live ticking
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const timerInfo: FormattedTimer = computeTimerDisplay(snapshot, receivedAt, now);

  // Activity detection for auto-hiding controls after 3s (A25)
  const resetHideTimer = () => {
    setControlsVisible(true);
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(() => {
      // Never hide while a menu, drawer, sheet or popover is open (A25)
      if (document.querySelector('[role="dialog"], [role="menu"], [data-radix-popper-content-wrapper], [data-state="open"]')) {
        return;
      }
      setControlsVisible(false);
    }, 3000);
  };

  useEffect(() => {
    const handleActivity = () => resetHideTimer();
    window.addEventListener('mousemove', handleActivity);
    window.addEventListener('keydown', handleActivity);
    window.addEventListener('touchstart', handleActivity);
    window.addEventListener('focusin', handleActivity);
    resetHideTimer();

    return () => {
      window.removeEventListener('mousemove', handleActivity);
      window.removeEventListener('keydown', handleActivity);
      window.removeEventListener('touchstart', handleActivity);
      window.removeEventListener('focusin', handleActivity);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
  }, []);

  // Filter items
  const images = mediaListDetail?.items.filter((i) => i.media_type === 'image' && i.url) || [];

  // Check auto-rotate setting
  const [autoRotateBg, setAutoRotateBg] = useState<boolean>(() => {
    const val = typeof localStorage !== 'undefined' ? localStorage.getItem('let_timer_auto_rotate_bg') : null;
    return val === null ? true : val === 'true';
  });

  useEffect(() => {
    const handleSync = () => {
      const val = typeof localStorage !== 'undefined' ? localStorage.getItem('let_timer_auto_rotate_bg') : null;
      setAutoRotateBg(val === null ? true : val === 'true');
    };
    window.addEventListener('let-timer-settings-changed', handleSync);
    return () => window.removeEventListener('let-timer-settings-changed', handleSync);
  }, []);

  // Crossfade between images every 90s if > 1 and auto-rotate is enabled
  useEffect(() => {
    if (images.length <= 1 || !autoRotateBg) return;
    const interval = setInterval(() => {
      setCurrentImageIndex((prev) => (prev + 1) % images.length);
    }, 90000);
    return () => clearInterval(interval);
  }, [images.length, autoRotateBg]);

  const appWallpaper = typeof localStorage !== 'undefined' ? localStorage.getItem('let_timer_wallpaper') : null;
  const hasImageBg = images.length > 0;
  const effectiveBg = hasImageBg ? (images[currentImageIndex]?.url || null) : appWallpaper;
  const hasAnyBg = Boolean(effectiveBg);

  const isIdle = !snapshot || snapshot.state === 'idle';
  const isRunning = snapshot?.state === 'running';
  const isPaused = snapshot?.state === 'paused';
  const isOvertime = timerInfo.mode === 'overtime';

  const currentTimeStr = new Date(now).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });

  const displayDigits = isIdle ? currentTimeStr : timerInfo.display;

  const digitColorClass = isIdle
    ? hasAnyBg
      ? 'text-[var(--imm-text)]'
      : 'text-[var(--accent-text)]'
    : isOvertime
    ? 'text-[var(--over-text)]'
    : isPaused
    ? hasAnyBg
      ? 'text-[var(--imm-muted)]'
      : 'text-[var(--muted)]'
    : hasAnyBg
    ? 'text-[var(--imm-text)]'
    : 'text-[var(--accent-text)]';

  const handleLifecycle = async (action: 'pause' | 'resume' | 'done') => {
    if (!snapshot) return;
    try {
      const res = await apiFetch(`/api/v1/tasks/${snapshot.task.id}/${action}`, {
        method: 'POST',
      });
      if (res.ok) onRefreshTimer();
      else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Action failed', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Action failed', 'danger');
    }
  };

  const handleAddFiveMinutes = async () => {
    if (!snapshot) return;
    const currentEst = snapshot.task.estimated_mins || Math.ceil(snapshot.elapsed_secs / 60);
    const newEst = currentEst + 5;

    try {
      const res = await apiFetch(`/api/v1/tasks/${snapshot.task.id}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ estimated_mins: newEst }),
      });

      if (res.ok) {
        showToast(`Extended estimate to ${newEst}m`, 'success');
        onRefreshTimer();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to extend estimate', 'danger');
    }
  };

  return (
    <div className="relative h-[calc(100vh-var(--header-h))] min-h-[500px] w-full overflow-hidden flex flex-col justify-between p-6 sm:p-10 pb-8 sm:pb-10 pt-2 sm:pt-4 select-none">
      {/* Layer 2: Main Timer Center Stage */}
      <div className="relative z-2 my-auto text-center flex flex-col items-center">
        {/* Workspace label right above timer digits */}
        <span
          className={`text-xs uppercase tracking-widest font-semibold mb-2 sm:mb-3 ${
            hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
          }`}
        >
          {(snapshot && snapshot.state !== 'idle' ? snapshot.workspace_name : null) || activeWorkspaceName || 'Workspace'}
        </span>

        {/* Giant Timer Digits (Always visible with consistent giant size) */}
        <TimerDigits
          digits={displayDigits}
          size="display"
          mode={isOvertime ? 'overtime' : isPaused ? 'paused' : isIdle ? 'idle' : 'countdown'}
          style={hasAnyBg ? { textShadow: '0 2px 10px rgba(0,0,0,0.5)' } : undefined}
          className={`transition-colors ${digitColorClass}`}
        />

        {/* Task Name or Idle Title */}
        <h1
          className={`font-serif text-2xl sm:text-3xl md:text-4xl font-semibold leading-tight mt-4 max-w-2xl px-4 ${
            hasAnyBg ? 'text-[var(--imm-text)]' : 'text-[var(--text)]'
          }`}
        >
          {isIdle ? 'No task running right now' : snapshot.task.name}
        </h1>

        {/* Task description or MediaList selector */}
        {!isIdle && snapshot.task.description && (
          <p
            className={`text-xs sm:text-sm mt-2 max-w-lg truncate leading-relaxed ${
              hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
            }`}
          >
            {snapshot.task.description}
          </p>
        )}

        {isIdle && (
          <div
            className={`flex items-center justify-center gap-2 mt-3 transition-opacity duration-[var(--dur-fast)] ease-[var(--ease-out)] focus-within:opacity-100 ${
              controlsVisible ? 'opacity-100' : 'opacity-0'
            }`}
          >
            <Popover open={playlistPopoverOpen} onOpenChange={setPlaylistPopoverOpen}>
              <PopoverTrigger asChild>
                <button
                  type="button"
                  disabled={mediaLists.length === 0}
                  className={`flex items-center gap-2 px-3.5 py-1.5 rounded-full border text-xs backdrop-blur-md shadow-xs transition select-none ${
                    mediaLists.length === 0 ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
                  } ${
                    hasAnyBg
                      ? 'bg-black/35 border-white/20 text-[var(--imm-text)] hover:bg-black/45'
                      : 'bg-[var(--surface)]/90 border-[var(--border)] text-[var(--text)] hover:bg-[var(--surface-hover)]'
                  }`}
                  aria-label="Select focus playlist"
                  aria-haspopup="listbox"
                >
                  <Music className="w-3.5 h-3.5 text-[var(--accent-text)] shrink-0" />
                  <span className="font-medium max-w-[160px] sm:max-w-[220px] truncate">
                    {mediaLists.length === 0
                      ? 'No playlists available'
                      : mediaLists.find((l) => l.id === selectedMediaListId)?.name || 'Choose focus playlist…'}
                  </span>
                  <ChevronDown className="w-3 h-3 text-[var(--muted)] opacity-70 shrink-0" />
                </button>
              </PopoverTrigger>

              <PopoverContent
                side="top"
                align="center"
                sideOffset={8}
                className="w-64 p-2 flex flex-col gap-1.5 max-h-60 overflow-y-auto"
              >
                <div className="text-xs font-semibold uppercase tracking-wider leading-tight text-[var(--muted)] px-2 py-1">
                  Focus Playlists
                </div>

                <div role="listbox" aria-label="Focus playlists" className="flex flex-col gap-0.5">
                  {selectedMediaListId && (
                    <button
                      type="button"
                      role="option"
                      aria-selected={false}
                      onClick={() => {
                        setSelectedMediaListId(null);
                        clearPlayer();
                        setPlaylistPopoverOpen(false);
                      }}
                      className="py-1.5 px-2 text-xs text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] rounded flex items-center justify-between transition cursor-pointer"
                    >
                      <span>None</span>
                    </button>
                  )}

                  {mediaLists.map((l) => {
                    const isSelected = l.id === selectedMediaListId;
                    return (
                      <button
                        key={l.id}
                        type="button"
                        role="option"
                        aria-selected={isSelected}
                        onClick={() => {
                          setSelectedMediaListId(l.id);
                          loadMediaListForTask(l.id);
                          setPlaylistPopoverOpen(false);
                        }}
                        className={`py-1.5 px-2 text-xs rounded flex items-center justify-between gap-2 transition cursor-pointer text-left ${
                          isSelected
                            ? 'bg-[var(--accent)]/10 text-[var(--accent-text)] font-semibold'
                            : 'text-[var(--text)] hover:bg-[var(--surface-hover)]'
                        }`}
                      >
                        <span className="truncate">{l.name}</span>
                        {isSelected && <Check className="w-3.5 h-3.5 text-[var(--accent-text)] shrink-0" />}
                      </button>
                    );
                  })}
                </div>
              </PopoverContent>
            </Popover>
          </div>
        )}

        {/* Action Controls (Fade out when idle) */}
        <div
          className={`flex items-center justify-center gap-3 mt-8 transition-opacity duration-[var(--dur-fast)] ease-[var(--ease-out)] focus-within:opacity-100 ${
            controlsVisible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          {isIdle ? (
            <button
              onClick={onOpenStartPicker}
              className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-[0.98] ${
                hasAnyBg
                  ? 'bg-white/10 hover:bg-white/20 text-[var(--imm-text)] border border-white/15'
                  : 'bg-[var(--surface-hover)] hover:bg-[var(--surface)] text-[var(--text)] border border-[var(--border)]'
              }`}
              aria-label="Start a task"
            >
              <Play className="w-3.5 h-3.5 fill-current text-[var(--accent-text)]" />
              <span>Start a task</span>
            </button>
          ) : (
            <>
              {isOvertime && (
                <button
                  onClick={handleAddFiveMinutes}
                  className={`flex items-center gap-1.5 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-[0.98] ${
                    hasAnyBg
                      ? 'bg-[var(--timer-over)]/25 hover:bg-[var(--timer-over)]/35 text-[var(--on-over)] border border-[var(--timer-over)]/40'
                      : 'bg-[var(--timer-over)]/15 hover:bg-[var(--timer-over)]/25 text-[var(--over-text)] border border-[var(--timer-over)]/30'
                  }`}
                  aria-label="Add 5 minutes"
                >
                  <Plus className="w-3.5 h-3.5 stroke-[2.5]" />
                  <span>+5 min</span>
                </button>
              )}

              {isRunning ? (
                <button
                  onClick={() => handleLifecycle('pause')}
                  className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-[0.98] ${
                    hasAnyBg
                      ? 'bg-white/10 hover:bg-white/20 text-[var(--imm-text)] border border-white/15'
                      : 'bg-[var(--surface-hover)] hover:bg-[var(--surface)] text-[var(--text)] border border-[var(--border)]'
                  }`}
                  aria-label="Pause timer"
                >
                  <Pause className="w-3.5 h-3.5 fill-current text-[var(--accent-text)]" />
                  <span>Pause</span>
                </button>
              ) : (
                <button
                  onClick={() => handleLifecycle('resume')}
                  className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-[0.98] ${
                    hasAnyBg
                      ? 'bg-white/10 hover:bg-white/20 text-[var(--imm-text)] border border-white/15'
                      : 'bg-[var(--surface-hover)] hover:bg-[var(--surface)] text-[var(--text)] border border-[var(--border)]'
                  }`}
                  aria-label="Resume timer"
                >
                  <Play className="w-3.5 h-3.5 fill-current text-[var(--accent-text)]" />
                  <span>Resume</span>
                </button>
              )}

              <button
                onClick={() => handleLifecycle('done')}
                className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-[0.98] ${
                  hasAnyBg
                    ? 'bg-white/10 hover:bg-white/20 text-[var(--imm-text)] border border-white/15'
                    : 'bg-[var(--surface-hover)] hover:bg-[var(--surface)] text-[var(--text)] border border-[var(--border)]'
                }`}
                aria-label="Complete task"
              >
                <Check className="w-3.5 h-3.5 stroke-[2.5] text-[var(--success-text)]" />
                <span>Done</span>
              </button>
            </>
          )}
        </div>
      </div>

      {/* Layer 3: Bottom Floating Island & hints */}
      <div className="relative z-3 h-10 flex items-center justify-between gap-4">
        <div className="w-24 hidden sm:block" />

        {/* Center Stage: Floating Player Island (always visible in collapsed or expanded form) */}
        <div className="relative flex items-center justify-center h-10">
          <FloatingPlayerIsland
            hasBackgroundPhoto={hasAnyBg}
            className="absolute bottom-0 left-1/2 -translate-x-1/2"
          />
        </div>

        <div
          className={`text-xs w-28 text-right hidden sm:block transition-opacity duration-[var(--dur-fast)] ease-[var(--ease-out)] focus-within:opacity-100 ${
            controlsVisible ? 'opacity-100' : 'opacity-0'
          } ${hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'}`}
        >
          {isIdle ? 'S: start' : 'Space: pause'}
        </div>
      </div>
    </div>
  );
};

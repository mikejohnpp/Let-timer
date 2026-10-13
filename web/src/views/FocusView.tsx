import React, { useState, useEffect, useRef } from 'react';
import {
  Play,
  Pause,
  Check,
  Plus,
  Music,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { apiFetch } from '../api/client';
import { useToast } from '../components/ui';
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
  const [currentImageIndex, setCurrentImageIndex] = useState(0);
  const [mediaLists, setMediaLists] = useState<{ id: number; name: string }[]>([]);
  const [selectedMediaListId, setSelectedMediaListId] = useState<number | null>(null);

  const { showToast } = useToast();
  const hideTimerRef = useRef<number | null>(null);
  const { mediaListDetail, loadMediaListForTask, pause, clearPlayer } = useAudioPlayer();

  const prevSnapshotStateRef = useRef<string>(snapshot?.state || 'idle');

  // Fetch available media lists for idle ambient playback
  useEffect(() => {
    apiFetch('/api/v1/media-lists')
      .then((res) => (res.ok ? res.json() : []))
      .then((data: { id: number; name: string }[]) => {
        setMediaLists(data);
      })
      .catch(() => {});
  }, []);

  // When a task starts: stop idle music and switch to task's media list if present
  useEffect(() => {
    const prevState = prevSnapshotStateRef.current;
    const currentState = snapshot?.state || 'idle';

    if ((prevState === 'idle' || !prevState) && currentState === 'running') {
      pause();
      if (snapshot?.task?.media_list_id) {
        setSelectedMediaListId(snapshot.task.media_list_id);
        loadMediaListForTask(snapshot.task.media_list_id);
      } else {
        setSelectedMediaListId(null);
        clearPlayer();
      }
    } else if (currentState !== 'idle' && snapshot?.task?.media_list_id && snapshot.task.media_list_id !== selectedMediaListId) {
      setSelectedMediaListId(snapshot.task.media_list_id);
      loadMediaListForTask(snapshot.task.media_list_id);
    }

    prevSnapshotStateRef.current = currentState;
  }, [snapshot?.state, snapshot?.task?.media_list_id, pause, clearPlayer, loadMediaListForTask, selectedMediaListId]);

  // Live ticking
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const timerInfo: FormattedTimer = computeTimerDisplay(snapshot, receivedAt, now);

  // Activity detection for auto-hiding controls after 3s
  const resetHideTimer = () => {
    setControlsVisible(true);
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(() => {
      setControlsVisible(false);
    }, 3000);
  };

  useEffect(() => {
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
  }, []);

  // Filter items
  const images = mediaListDetail?.items.filter((i) => i.media_type === 'image' && i.url) || [];

  // Crossfade between images every 90s if > 1
  useEffect(() => {
    if (images.length <= 1) return;
    const interval = setInterval(() => {
      setCurrentImageIndex((prev) => (prev + 1) % images.length);
    }, 90000);
    return () => clearInterval(interval);
  }, [images.length]);

  const appWallpaper = typeof window !== 'undefined' ? localStorage.getItem('let_timer_wallpaper') : null;
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
    <div className="relative min-h-screen w-full overflow-hidden flex flex-col justify-between p-6 sm:p-12 pb-24 sm:pb-12 pt-14 sm:pt-16 select-none">
      {/* Layer 0: Background Photo */}
      {hasAnyBg && effectiveBg && (
        <div
          className="absolute inset-0 bg-cover bg-center transition-all duration-1000 z-0"
          style={{ backgroundImage: `url(${effectiveBg})` }}
        />
      )}

      {/* Layer 1: 65% Black Overlay (when image is present) */}
      {hasAnyBg && (
        <div className="absolute inset-0 bg-[var(--imm-overlay)] z-1 pointer-events-none" />
      )}

      {/* Layer 2: Top Space & Workspace label (always visible) */}
      <div className="relative z-2 text-center">
        <span
          className={`text-xs uppercase tracking-widest font-semibold ${
            hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
          }`}
        >
          {snapshot?.workspace_name || 'Focus'}
        </span>
      </div>

      {/* Layer 2: Main Timer Center Stage */}
      <div className="relative z-2 my-auto text-center flex flex-col items-center">
        {/* Giant Timer Digits (Always visible with consistent giant size) */}
        <div
          className={`font-mono font-light tabular-nums tracking-tighter text-6xl sm:text-8xl md:text-9xl transition-colors ${digitColorClass}`}
          style={hasAnyBg ? { textShadow: '0 2px 10px rgba(0,0,0,0.5)' } : undefined}
        >
          {displayDigits}
        </div>

        {/* Task Name or Idle Title */}
        <h1
          className={`font-serif text-2xl sm:text-3xl md:text-4xl font-bold mt-4 max-w-2xl px-4 ${
            hasAnyBg ? 'text-[var(--imm-text)]' : 'text-[var(--text)]'
          }`}
        >
          {isIdle ? 'No task running right now' : snapshot.task.name}
        </h1>

        {/* Task description or MediaList selector */}
        {!isIdle && snapshot.task.description && (
          <p
            className={`text-xs sm:text-sm mt-2 max-w-lg truncate ${
              hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
            }`}
          >
            {snapshot.task.description}
          </p>
        )}

        {isIdle && (
          <div
            className={`flex items-center justify-center gap-2 mt-3 transition-opacity duration-200 ${
              controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
            }`}
          >
            <div
              className={`flex items-center gap-2 px-3.5 py-1.5 rounded-full border text-xs backdrop-blur-md shadow-xs ${
                hasAnyBg
                  ? 'bg-black/35 border-white/20 text-[var(--imm-text)]'
                  : 'bg-[var(--surface)]/90 border-[var(--border)] text-[var(--text)]'
              }`}
            >
              <Music className="w-3.5 h-3.5 text-[var(--accent-text)] shrink-0" />
              <select
                value={selectedMediaListId || ''}
                onChange={(e) => {
                  const val = e.target.value ? Number(e.target.value) : null;
                  setSelectedMediaListId(val);
                  loadMediaListForTask(val);
                }}
                className="bg-transparent outline-none cursor-pointer text-xs font-medium max-w-[200px] truncate"
                aria-label="Select focus playlist"
              >
                <option value="" className="text-black bg-white dark:text-white dark:bg-neutral-900">
                  {mediaLists.length === 0 ? 'No playlists available' : 'Choose focus playlist…'}
                </option>
                {mediaLists.map((l) => (
                  <option key={l.id} value={l.id} className="text-black bg-white dark:text-white dark:bg-neutral-900">
                    {l.name}
                  </option>
                ))}
              </select>
            </div>
          </div>
        )}

        {/* Action Controls (Fade out when idle) */}
        <div
          className={`flex items-center justify-center gap-3 mt-8 transition-opacity duration-200 ${
            controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
          }`}
        >
          {isIdle ? (
            <button
              onClick={onOpenStartPicker}
              className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-95 ${
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
                  className={`flex items-center gap-1.5 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-95 ${
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
                  className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-95 ${
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
                  className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-95 ${
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
                className={`flex items-center gap-2 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-95 ${
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
      <div className="relative z-3 flex items-center justify-between gap-4">
        <div className="w-24 hidden sm:block" />

        {/* Center Stage: Floating Player Island (always visible in collapsed or expanded form) */}
        <FloatingPlayerIsland
          hasBackgroundPhoto={hasAnyBg}
          className="mx-auto"
        />

        <div
          className={`text-[11px] w-24 text-right hidden sm:block transition-opacity duration-200 ${
            controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
          } ${hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'}`}
        >
          {isIdle ? 'S: start' : 'Space: pause'}
        </div>
      </div>
    </div>
  );
};

import React, { useState, useEffect, useRef } from 'react';
import {
  Play,
  Pause,
  Check,
  RotateCcw,
  Square,
  XCircle,
  Plus,
  MoreHorizontal,
  Compass,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { getStoredToken } from '../api/client';

interface TimerBarProps {
  snapshot: TimerSnapshot | null;
  receivedAt: number;
  onOpenFocus: () => void;
  onOpenStartPicker: () => void;
  onActionComplete: () => void;
}

export const TimerBar: React.FC<TimerBarProps> = ({
  snapshot,
  receivedAt,
  onOpenFocus,
  onOpenStartPicker,
  onActionComplete,
}) => {
  const [now, setNow] = useState<number>(Date.now());
  const [showMenu, setShowMenu] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Live 1-second ticking
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Close more menu when clicking outside
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowMenu(false);
      }
    }
    if (showMenu) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showMenu]);

  const timerInfo: FormattedTimer = computeTimerDisplay(snapshot, receivedAt, now);

  const performTransition = async (action: 'pause' | 'resume' | 'stop' | 'done' | 'cancel' | 'restart') => {
    if (!snapshot || snapshot.state === 'idle') return;
    setIsSubmitting(true);
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks/${snapshot.task.id}/${action}`, {
        method: 'POST',
        headers,
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        alert(err.message || `Action failed with status ${res.status}`);
      } else {
        onActionComplete();
      }
    } catch (e) {
      console.error(`Failed to perform ${action}`, e);
    } finally {
      setIsSubmitting(false);
      setShowMenu(false);
    }
  };

  const handleAddFiveMinutes = async () => {
    if (!snapshot || snapshot.state === 'idle') return;
    const currentEst = snapshot.task.estimated_mins || Math.ceil(snapshot.elapsed_secs / 60);
    const newEst = currentEst + 5;

    setIsSubmitting(true);
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks/${snapshot.task.id}`, {
        method: 'PATCH',
        headers,
        body: JSON.stringify({ estimated_mins: newEst }),
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        alert(err.message || `Failed to extend estimate`);
      } else {
        onActionComplete();
      }
    } catch (e) {
      console.error('Failed to extend estimate', e);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!snapshot || snapshot.state === 'idle') {
    return (
      <div className="bg-[var(--surface)] border-b border-[var(--border)] px-4 py-3 shadow-sm">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-3 text-[var(--muted)]">
            <span className="w-2.5 h-2.5 rounded-full bg-[var(--muted)]/50" />
            <span className="text-sm font-medium">Nothing running</span>
          </div>
          <button
            onClick={onOpenStartPicker}
            className="flex items-center gap-1.5 bg-[var(--surface-hover)] hover:bg-[var(--border)] text-[var(--text)] px-3 py-1.5 rounded-lg text-sm font-medium transition"
          >
            <Play className="w-3.5 h-3.5 fill-current" />
            <span>Start a task…</span>
          </button>
        </div>
      </div>
    );
  }

  const isRunning = snapshot.state === 'running';
  const isPaused = snapshot.state === 'paused';
  const isOvertime = timerInfo.mode === 'overtime';

  // Color tokens per state
  let stateColor = 'var(--timer-run)';
  let stateBadgeBg = 'rgba(2, 132, 199, 0.1)';
  let stateBorder = 'rgba(2, 132, 199, 0.2)';
  if (isOvertime) {
    stateColor = 'var(--timer-over)';
    stateBadgeBg = 'rgba(249, 115, 22, 0.1)';
    stateBorder = 'rgba(249, 115, 22, 0.3)';
  } else if (isPaused) {
    stateColor = 'var(--timer-pause)';
    stateBadgeBg = 'rgba(234, 179, 8, 0.1)';
    stateBorder = 'rgba(234, 179, 8, 0.2)';
  }

  return (
    <div
      className="bg-[var(--surface)] border-b border-[var(--border)] px-4 py-2.5 shadow-sm sticky top-16 z-30 transition-colors"
      style={{ borderBottomColor: stateBorder }}
    >
      {/* Hidden live region for accessibility announcements */}
      <div aria-live="polite" className="sr-only">
        {timerInfo.display} - {snapshot.task.name}
      </div>

      <div className="max-w-7xl mx-auto flex flex-wrap items-center justify-between gap-3">
        {/* Left: Timer digits + Task info */}
        <div className="flex items-center gap-3 min-w-0">
          <div
            className="flex items-center gap-2 px-3 py-1 rounded-lg font-bold text-xl tabular-nums tracking-tight border transition-colors shadow-inner"
            style={{
              color: stateColor,
              backgroundColor: stateBadgeBg,
              borderColor: stateBorder,
            }}
          >
            {isPaused ? '⏸' : isOvertime ? '⚠' : '⏳'}
            <span>{timerInfo.display}</span>
          </div>

          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-semibold text-sm truncate text-[var(--text)]">
                {snapshot.task.name}
              </span>
              <span className="text-xs px-1.5 py-0.5 rounded bg-[var(--surface-hover)] text-[var(--muted)] border border-[var(--border)]">
                {snapshot.workspace_name}
              </span>
            </div>
            <div className="text-xs text-[var(--muted)] flex items-center gap-1.5">
              <span>{isOvertime ? "Time's up" : isPaused ? 'Paused' : 'In progress'}</span>
              {snapshot.task.estimated_mins && (
                <>
                  <span>•</span>
                  <span>{snapshot.task.estimated_mins}m estimate</span>
                </>
              )}
            </div>
          </div>
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-2 ml-auto">
          {/* Focus mode link */}
          <button
            onClick={onOpenFocus}
            className="p-1.5 text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-lg transition"
            title="Open Focus mode"
            aria-label="Open Focus mode"
          >
            <Compass className="w-4 h-4" />
          </button>

          {/* Overtime "+5 min" button */}
          {isOvertime && (
            <button
              onClick={handleAddFiveMinutes}
              disabled={isSubmitting}
              className="flex items-center gap-1 bg-[var(--timer-over)] hover:opacity-90 text-white px-2.5 py-1.5 rounded-lg text-xs font-semibold shadow-sm transition disabled:opacity-50"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>+5 min</span>
            </button>
          )}

          {/* Primary Lifecycle Controls */}
          {isRunning ? (
            <button
              onClick={() => performTransition('pause')}
              disabled={isSubmitting}
              className="flex items-center gap-1.5 bg-[var(--surface-hover)] hover:bg-[var(--border)] text-[var(--text)] px-3 py-1.5 rounded-lg text-sm font-medium border border-[var(--border)] transition disabled:opacity-50"
            >
              <Pause className="w-3.5 h-3.5 fill-current" />
              <span>Pause</span>
            </button>
          ) : (
            <button
              onClick={() => performTransition('resume')}
              disabled={isSubmitting}
              className="flex items-center gap-1.5 bg-[var(--timer-run)] hover:opacity-90 text-white px-3 py-1.5 rounded-lg text-sm font-medium shadow-sm transition disabled:opacity-50"
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span>Resume</span>
            </button>
          )}

          <button
            onClick={() => performTransition('done')}
            disabled={isSubmitting}
            className="flex items-center gap-1.5 bg-[var(--success)] hover:opacity-90 text-white px-3 py-1.5 rounded-lg text-sm font-semibold shadow-sm transition disabled:opacity-50"
          >
            <Check className="w-4 h-4 stroke-[3]" />
            <span>Done</span>
          </button>

          {/* More menu (Stop, Restart, Cancel) */}
          <div className="relative" ref={menuRef}>
            <button
              onClick={() => setShowMenu(!showMenu)}
              className="p-1.5 text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-lg border border-[var(--border)] transition"
              aria-label="More actions"
            >
              <MoreHorizontal className="w-4 h-4" />
            </button>

            {showMenu && (
              <div className="absolute right-0 mt-2 w-48 bg-[var(--surface)] border border-[var(--border)] rounded-xl shadow-xl py-1 z-50 text-sm">
                <button
                  onClick={() => performTransition('stop')}
                  className="w-full px-4 py-2 text-left flex items-center gap-2.5 text-[var(--text)] hover:bg-[var(--surface-hover)] transition"
                >
                  <Square className="w-4 h-4 text-[var(--muted)]" />
                  <span>Stop (keep elapsed)</span>
                </button>
                <button
                  onClick={() => performTransition('restart')}
                  className="w-full px-4 py-2 text-left flex items-center gap-2.5 text-[var(--text)] hover:bg-[var(--surface-hover)] transition"
                >
                  <RotateCcw className="w-4 h-4 text-[var(--muted)]" />
                  <span>Restart (new run)</span>
                </button>
                <div className="my-1 border-t border-[var(--border)]" />
                <button
                  onClick={() => performTransition('cancel')}
                  className="w-full px-4 py-2 text-left flex items-center gap-2.5 text-[var(--danger)] hover:bg-[var(--surface-hover)] transition"
                >
                  <XCircle className="w-4 h-4" />
                  <span>Cancel task</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

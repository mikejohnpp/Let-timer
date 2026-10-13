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
  Target,
  Timer as TimerIcon,
  AlertTriangle,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { getStoredToken } from '../api/client';
import {
  Button,
  IconButton,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  LiveRegion,
  useToast,
  useConfirm,
} from './ui';

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
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [lastAnnouncedMinute, setLastAnnouncedMinute] = useState<number | null>(null);
  const [liveAnnouncement, setLiveAnnouncement] = useState<string>('');
  const [liveRole, setLiveRole] = useState<'status' | 'alert'>('status');

  const { showToast } = useToast();
  const confirm = useConfirm();
  const prevModeRef = useRef<string>('idle');

  // Live 1-second ticking
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const timerInfo: FormattedTimer = computeTimerDisplay(snapshot, receivedAt, now);

  // Announce minute change or state change in LiveRegion
  useEffect(() => {
    if (!snapshot || snapshot.state === 'idle') return;

    const currentMinute = Math.floor(timerInfo.totalElapsedSecs / 60);
    const modeChanged = prevModeRef.current !== timerInfo.mode;
    prevModeRef.current = timerInfo.mode;

    if (timerInfo.mode === 'overtime' && modeChanged) {
      setLiveRole('alert');
      setLiveAnnouncement(`Time's up: ${snapshot.task.name}`);
      setLastAnnouncedMinute(currentMinute);
    } else if (modeChanged || (currentMinute !== lastAnnouncedMinute && timerInfo.totalElapsedSecs > 0)) {
      setLiveRole('status');
      setLiveAnnouncement(`${timerInfo.display} - ${snapshot.task.name}`);
      setLastAnnouncedMinute(currentMinute);
    }
  }, [snapshot, timerInfo.display, timerInfo.mode, timerInfo.totalElapsedSecs, lastAnnouncedMinute]);

  const performTransition = async (action: 'pause' | 'resume' | 'stop' | 'done' | 'cancel' | 'restart') => {
    if (!snapshot || snapshot.state === 'idle') return;

    if (action === 'cancel') {
      const ok = await confirm({
        title: `Cancel task "${snapshot.task.name}"?`,
        body: 'The task status will be set to cancelled and the timer will stop.',
        confirmLabel: 'Cancel task',
        tone: 'danger',
      });
      if (!ok) return;
    } else if (action === 'restart') {
      const ok = await confirm({
        title: `Restart timer for "${snapshot.task.name}"?`,
        body: 'Restart starts a new run and resets elapsed time.',
        confirmLabel: 'Restart',
      });
      if (!ok) return;
    }

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
        showToast(err.message || `Action failed with status ${res.status}`, 'danger');
      } else {
        onActionComplete();
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Action failed';
      showToast(msg, 'danger');
    } finally {
      setIsSubmitting(false);
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
        showToast(err.message || 'Failed to extend estimate', 'danger');
      } else {
        showToast(`Extended estimate to ${newEst}m`, 'success');
        onActionComplete();
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Failed to extend estimate';
      showToast(msg, 'danger');
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!snapshot || snapshot.state === 'idle') {
    return (
      <div className="bg-[var(--surface)] border-b border-[var(--border)] px-4 py-2.5 shadow-xs sticky top-0 lg:top-[var(--header-h)] z-30 transition-colors pt-[env(safe-area-inset-top)]">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-2.5 text-[var(--muted)]">
            <span className="w-2 h-2 rounded-full bg-[var(--muted)]/50" />
            <span className="text-xs font-medium">Nothing running</span>
          </div>
          <Button variant="secondary" size="sm" onClick={onOpenStartPicker}>
            <Play className="w-3.5 h-3.5 fill-current" />
            <span>Start a task…</span>
          </Button>
        </div>
      </div>
    );
  }

  const isRunning = snapshot.state === 'running';
  const isPaused = snapshot.state === 'paused';
  const isOvertime = timerInfo.mode === 'overtime';

  return (
    <div
      className={`border-b px-4 py-2 shadow-xs sticky top-0 lg:top-[var(--header-h)] z-30 transition-colors pt-[env(safe-area-inset-top)] ${
        isOvertime
          ? 'bg-[var(--over-bg)] border-[var(--timer-over)]'
          : isPaused
          ? 'bg-[var(--surface)] border-[var(--border)]'
          : 'bg-[var(--surface)] border-[var(--accent)]/30'
      }`}
    >
      <LiveRegion message={liveAnnouncement} role={liveRole} />

      <div className="max-w-7xl mx-auto flex items-center justify-between gap-3">
        {/* Left: Timer digits + Task info */}
        <div className="flex items-center gap-2.5 sm:gap-3 min-w-0">
          {/* Status icon & digits */}
          <div
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-control)] font-mono font-bold text-sm sm:text-base tabular-nums border select-none ${
              isOvertime
                ? 'text-[var(--over-text)] bg-[var(--timer-over)]/10 border-[var(--timer-over)]/30'
                : isPaused
                ? 'text-[var(--muted)] bg-[var(--surface-hover)] border-[var(--border)]'
                : 'text-[var(--accent-text)] bg-[var(--accent)]/10 border-[var(--accent)]/20'
            }`}
          >
            {isOvertime ? (
              <AlertTriangle className="w-3.5 h-3.5 text-[var(--timer-over)]" />
            ) : isPaused ? (
              <Pause className="w-3.5 h-3.5 text-[var(--muted)]" />
            ) : (
              <TimerIcon className="w-3.5 h-3.5 text-[var(--accent-text)] animate-pulse" />
            )}
            <span>{timerInfo.display}</span>
          </div>

          <div className="min-w-0 flex flex-col justify-center">
            <div className="flex items-center gap-1.5">
              <span className="font-semibold text-xs sm:text-sm truncate text-[var(--text)]">
                {snapshot.task.name}
              </span>
              <span className="hidden sm:inline-block text-[10px] px-1.5 py-0.2 rounded bg-[var(--surface-hover)] text-[var(--muted)] border border-[var(--border)] font-medium">
                {snapshot.workspace_name}
              </span>
            </div>
            <div className="text-[11px] text-[var(--muted)] flex items-center gap-1.5">
              <span className="capitalize font-medium">
                {isOvertime ? "Time's up" : isPaused ? 'Paused' : 'In progress'}
              </span>
              {snapshot.task.estimated_mins && (
                <>
                  <span>•</span>
                  <span>{snapshot.task.estimated_mins}m est</span>
                </>
              )}
            </div>
          </div>
        </div>

        {/* Right: Controls */}
        <div className="flex items-center gap-1.5 sm:gap-2 shrink-0">
          {/* Focus button */}
          <IconButton
            variant="ghost"
            size="sm"
            onClick={onOpenFocus}
            aria-label="Open Focus mode"
            title="Open Focus mode"
          >
            <Target className="w-4 h-4" />
          </IconButton>

          {/* Overtime +5 min button */}
          {isOvertime && (
            <button
              onClick={handleAddFiveMinutes}
              disabled={isSubmitting}
              className="flex items-center gap-1 bg-[var(--timer-over)] text-[var(--on-over)] hover:opacity-90 px-2.5 py-1 rounded-[var(--radius-control)] text-xs font-bold transition disabled:opacity-50 cursor-pointer shadow-xs"
            >
              <Plus className="w-3.5 h-3.5 stroke-[3]" />
              <span className="hidden sm:inline">+5 min</span>
            </button>
          )}

          {/* Pause / Resume */}
          {isRunning ? (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => performTransition('pause')}
              disabled={isSubmitting}
            >
              <Pause className="w-3.5 h-3.5 fill-current" />
              <span className="hidden sm:inline">Pause</span>
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              onClick={() => performTransition('resume')}
              disabled={isSubmitting}
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span className="hidden sm:inline">Resume</span>
            </Button>
          )}

          {/* Done */}
          <Button
            variant="secondary"
            size="sm"
            onClick={() => performTransition('done')}
            disabled={isSubmitting}
            className="border-[var(--success)]/40 hover:bg-[var(--success-bg)] text-[var(--success-text)]"
          >
            <Check className="w-3.5 h-3.5 stroke-[3]" />
            <span className="hidden sm:inline">Done</span>
          </Button>

          {/* Radix Dropdown Menu for Stop / Restart / Cancel */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <IconButton variant="ghost" size="sm" aria-label="More timer actions">
                <MoreHorizontal className="w-4 h-4" />
              </IconButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => performTransition('stop')}>
                <Square className="w-3.5 h-3.5 text-[var(--muted)]" />
                <span>Stop (keep elapsed)</span>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => performTransition('restart')}>
                <RotateCcw className="w-3.5 h-3.5 text-[var(--muted)]" />
                <span>Restart (new run)</span>
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem destructive onClick={() => performTransition('cancel')}>
                <XCircle className="w-3.5 h-3.5" />
                <span>Cancel task</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    </div>
  );
};

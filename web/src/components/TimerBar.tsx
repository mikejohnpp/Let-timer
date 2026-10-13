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
import { apiFetch } from '../api/client';
import {
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
      const res = await apiFetch(`/api/v1/tasks/${snapshot.task.id}/${action}`, {
        method: 'POST',
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
      const res = await apiFetch(`/api/v1/tasks/${snapshot.task.id}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
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
      <div className="sr-only" aria-hidden="true">
        <span>Nothing running</span>
        <button onClick={onOpenStartPicker}>Start a task…</button>
      </div>
    );
  }

  const isRunning = snapshot.state === 'running';
  const isPaused = snapshot.state === 'paused';
  const isOvertime = timerInfo.mode === 'overtime';

  return (
    <div
      className={`flex items-center gap-2.5 sm:gap-3 px-3.5 sm:px-4 py-1.5 rounded-full border transition-all duration-300 select-none shadow-xl backdrop-blur-md ${
        isOvertime
          ? 'bg-[var(--over-bg)]/95 border-[var(--timer-over)]'
          : isPaused
          ? 'bg-[var(--surface-hover)]/95 border-[var(--border)]'
          : 'bg-[var(--surface)]/90 border-[var(--border-strong)]/30'
      }`}
    >
      <LiveRegion message={liveAnnouncement} role={liveRole} />

      {/* Timer digits + status icon */}
      <div className="flex items-center gap-1.5 font-mono font-bold text-xs sm:text-sm tabular-nums">
        {isOvertime ? (
          <AlertTriangle className="w-3.5 h-3.5 text-[var(--timer-over)]" />
        ) : isPaused ? (
          <Pause className="w-3.5 h-3.5 text-[var(--muted)]" />
        ) : (
          <TimerIcon className="w-3.5 h-3.5 text-[var(--accent-text)] animate-pulse" />
        )}
        <span
          className={
            isOvertime
              ? 'text-[var(--over-text)]'
              : isPaused
              ? 'text-[var(--muted)]'
              : 'text-[var(--accent-text)]'
          }
        >
          {timerInfo.display}
        </span>
      </div>

      {/* Task name & status metadata */}
      <div className="flex items-center gap-1.5 min-w-0 max-w-[110px] sm:max-w-[200px]">
        <span className="text-xs font-semibold truncate text-[var(--text)]">
          {snapshot.task.name}
        </span>
        <span className="sr-only">{snapshot.workspace_name}</span>
        <span className="sr-only">
          {isOvertime ? "Time's up" : isPaused ? 'Paused' : 'In progress'}
        </span>
      </div>

      {/* Control buttons */}
      <div className="flex items-center gap-1">
        {isOvertime && (
          <button
            onClick={handleAddFiveMinutes}
            disabled={isSubmitting}
            className="flex items-center gap-1 bg-[var(--timer-over)] text-[var(--on-over)] px-2 py-0.5 rounded-full text-[11px] font-bold hover:opacity-90 transition cursor-pointer"
          >
            <Plus className="w-3 h-3 stroke-[3]" />
            <span>+5 min</span>
          </button>
        )}

        {/* Focus Mode button */}
        <IconButton
          variant="ghost"
          size="sm"
          className="h-6 w-6 p-0"
          onClick={onOpenFocus}
          aria-label="Open Focus mode"
          title="Open Focus mode"
        >
          <Target className="w-3.5 h-3.5 text-[var(--muted)] hover:text-[var(--text)]" />
        </IconButton>

        {/* Pause / Resume */}
        {isRunning ? (
          <IconButton
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            onClick={() => performTransition('pause')}
            disabled={isSubmitting}
            aria-label="Pause"
            title="Pause"
          >
            <Pause className="w-3.5 h-3.5 fill-current text-[var(--text)]" />
            <span className="sr-only">Pause</span>
          </IconButton>
        ) : (
          <IconButton
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            onClick={() => performTransition('resume')}
            disabled={isSubmitting}
            aria-label="Resume"
            title="Resume"
          >
            <Play className="w-3.5 h-3.5 fill-current text-[var(--accent-text)]" />
            <span className="sr-only">Resume</span>
          </IconButton>
        )}

        {/* Done */}
        <IconButton
          variant="ghost"
          size="sm"
          className="h-6 w-6 p-0 hover:bg-[var(--success-bg)]"
          onClick={() => performTransition('done')}
          disabled={isSubmitting}
          aria-label="Done"
          title="Done"
        >
          <Check className="w-3.5 h-3.5 stroke-[3] text-[var(--success-text)]" />
          <span className="sr-only">Done</span>
        </IconButton>

        {/* Dropdown Menu for Stop / Restart / Cancel */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <IconButton variant="ghost" size="sm" className="h-6 w-6 p-0" aria-label="More timer actions">
              <MoreHorizontal className="w-3.5 h-3.5 text-[var(--muted)]" />
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
  );
};

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
  AlertTriangle,
  Music,
  SkipForward,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { apiFetch } from '../api/client';
import { useAudioPlayer } from '../context/AudioPlayerContext';
import { SoundWave } from './SoundWave';
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
  const { currentTrack, isPlaying: isAudioPlaying, togglePlay: toggleAudio, nextTrack } = useAudioPlayer();

  const lastActiveSnapshotRef = useRef<TimerSnapshot | null>(
    snapshot && snapshot.state !== 'idle' ? snapshot : null
  );
  if (snapshot && snapshot.state !== 'idle') {
    lastActiveSnapshotRef.current = snapshot;
  }
  const activeSnapshot = snapshot && snapshot.state !== 'idle' ? snapshot : lastActiveSnapshotRef.current;

  // Live 1-second ticking
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const timerInfo: FormattedTimer = computeTimerDisplay(activeSnapshot, receivedAt, now);

  // Announce minute change or state change in LiveRegion
  useEffect(() => {
    if (!snapshot || snapshot.state === 'idle') return;

    const currentMinute = Math.floor(timerInfo.totalElapsedSecs / 60);
    const modeChanged = prevModeRef.current !== timerInfo.mode;
    prevModeRef.current = timerInfo.mode;

    if (timerInfo.mode === 'overtime' && modeChanged) {
      setLiveRole('alert');
      setLiveAnnouncement(`Time's up: ${snapshot.task.name}`);
      showToast(`Time's up: ${snapshot.task.name}`, 'danger');
      setLastAnnouncedMinute(currentMinute);
    } else if (modeChanged || (currentMinute !== lastAnnouncedMinute && timerInfo.totalElapsedSecs > 0)) {
      setLiveRole('status');
      setLiveAnnouncement(`${timerInfo.display} - ${snapshot.task.name}`);
      setLastAnnouncedMinute(currentMinute);
    }
  }, [snapshot, timerInfo.display, timerInfo.mode, timerInfo.totalElapsedSecs, lastAnnouncedMinute, showToast]);

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

  if (!activeSnapshot || activeSnapshot.state === 'idle') {
    return (
      <div className="sr-only" aria-hidden="true">
        <span>Nothing running</span>
        <button onClick={onOpenStartPicker}>Start a task…</button>
      </div>
    );
  }

  const isRunning = activeSnapshot.state === 'running';
  const isPaused = activeSnapshot.state === 'paused';
  const isOvertime = timerInfo.mode === 'overtime';

  return (
    <div
      className={`relative flex items-center justify-between gap-2.5 sm:gap-3 px-3.5 sm:px-4 py-1.5 rounded-full border transition-[background-color,border-color,color] duration-[var(--dur-ambient)] ease-[var(--ease-out)] select-none shadow-xl backdrop-blur-md w-[320px] sm:w-[420px] max-w-[92vw] ${
        isOvertime
          ? 'bg-[var(--over-bg)]/95 border-[var(--timer-over)]'
          : isPaused
          ? 'bg-[var(--surface-hover)]/95 border-[var(--border)]'
          : 'bg-[var(--surface)]/90 border-[var(--border-strong)]/30'
      }`}
    >
      {/* Time's up two ring pulses (A14) */}
      {isOvertime && (
        <div
          aria-hidden="true"
          className="absolute -inset-1 rounded-full border-2 border-[var(--timer-over)] ring-pulse pointer-events-none"
        />
      )}

      <LiveRegion message={liveAnnouncement} role={liveRole} />

      {/* Timer digits + status icon */}
      <div className="flex items-center gap-1.5 font-mono font-bold text-xs sm:text-sm tabular-nums shrink-0">
        <span className="transition-opacity duration-[var(--dur-fast)] ease-[var(--ease-out)] flex items-center">
          {isOvertime ? (
            <AlertTriangle className="w-3.5 h-3.5 text-[var(--timer-over)]" />
          ) : isPaused ? (
            <Pause className="w-3.5 h-3.5 text-[var(--muted)]" />
          ) : (
            <span className="relative flex h-3.5 w-3.5 items-center justify-center">
              <span className="w-2 h-2 rounded-full bg-[var(--accent-text)] dot-running" />
            </span>
          )}
        </span>
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

      {/* Task name & status metadata (fixed width flex-1 with crossfade) */}
      <div className="flex items-center gap-1.5 min-w-0 flex-1">
        <span className="text-xs font-semibold truncate text-[var(--text)] transition-opacity duration-[var(--dur-fast)] ease-[var(--ease-out)]">
          {activeSnapshot.task.name}
        </span>
        <span className="sr-only">{activeSnapshot.workspace_name}</span>
        <span className="sr-only">
          {isOvertime ? "Time's up" : isPaused ? 'Paused' : 'In progress'}
        </span>
      </div>

      {/* Music Section in Status Island (if track is active) */}
      {currentTrack && (
        <div className="flex items-center gap-1.5 pl-2 border-l border-[var(--border)]/40 min-w-0">
          <div
            onClick={toggleAudio}
            className="flex items-center gap-1.5 cursor-pointer group min-w-0"
            title={`${isAudioPlaying ? 'Pause' : 'Play'}: ${currentTrack.name}`}
          >
            {isAudioPlaying ? (
              <SoundWave isPlaying={true} className="shrink-0" />
            ) : (
              <Music className="w-3.5 h-3.5 text-[var(--muted)] group-hover:text-[var(--text)] shrink-0 transition" />
            )}
            <span className="text-[11px] font-medium text-[var(--muted)] group-hover:text-[var(--text)] truncate max-w-[75px] sm:max-w-[120px] transition">
              {currentTrack.name}
            </span>
          </div>

          <div className="flex items-center gap-0.5 shrink-0">
            <button
              type="button"
              onClick={toggleAudio}
              className="h-5 w-5 rounded-full flex items-center justify-center text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition cursor-pointer"
              aria-label={isAudioPlaying ? 'Pause music' : 'Play music'}
              title={isAudioPlaying ? 'Pause music' : 'Play music'}
            >
              {isAudioPlaying ? (
                <Pause className="w-2.5 h-2.5 fill-current" />
              ) : (
                <Play className="w-2.5 h-2.5 fill-current ml-0.5" />
              )}
            </button>
            <button
              type="button"
              onClick={nextTrack}
              className="h-5 w-5 rounded-full flex items-center justify-center text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition cursor-pointer"
              aria-label="Next track"
              title="Next track"
            >
              <SkipForward className="w-2.5 h-2.5" />
            </button>
          </div>
        </div>
      )}

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

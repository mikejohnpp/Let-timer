import type { components } from '../api/schema';

export type TimerSnapshot = components['schemas']['TimerSnapshotDto'];

export interface FormattedTimer {
  mode: 'countdown' | 'overtime' | 'elapsed' | 'idle';
  display: string;
  totalElapsedSecs: number;
  remainingSecs: number | null;
  overtimeSecs: number;
}

/**
 * Format a duration in seconds into mm:ss or hh:mm:ss.
 */
export function formatDuration(seconds: number): string {
  const isNegative = seconds < 0;
  const abs = Math.abs(seconds);
  const hrs = Math.floor(abs / 3600);
  const mins = Math.floor((abs % 3600) / 60);
  const secs = abs % 60;

  const pad = (n: number) => n.toString().padStart(2, '0');

  let formatted = '';
  if (hrs > 0) {
    formatted = `${hrs}:${pad(mins)}:${pad(secs)}`;
  } else {
    formatted = `${pad(mins)}:${pad(secs)}`;
  }

  return isNegative ? `-${formatted}` : formatted;
}

/**
 * Pure timer maths: given a snapshot, the timestamp when it was received, and the current instant,
 * compute the active timer display and overtime state.
 */
export function computeTimerDisplay(
  snapshot: TimerSnapshot | null,
  receivedAt: number,
  now: number
): FormattedTimer {
  if (!snapshot || snapshot.state === 'idle') {
    return {
      mode: 'idle',
      display: '00:00',
      totalElapsedSecs: 0,
      remainingSecs: null,
      overtimeSecs: 0,
    };
  }

  const isTicking = snapshot.state === 'running';
  const deltaSecs = isTicking ? Math.max(0, Math.floor((now - receivedAt) / 1000)) : 0;
  const totalElapsed = snapshot.elapsed_secs + deltaSecs;

  const estimatedMins = snapshot.task.estimated_mins;
  if (estimatedMins == null) {
    // No estimate: simply elapsed timer
    return {
      mode: 'elapsed',
      display: formatDuration(totalElapsed),
      totalElapsedSecs: totalElapsed,
      remainingSecs: null,
      overtimeSecs: 0,
    };
  }

  const estimatedSecs = estimatedMins * 60;
  const remaining = estimatedSecs - totalElapsed;

  if (remaining < 0) {
    const overtime = Math.abs(remaining);
    return {
      mode: 'overtime',
      display: `+${formatDuration(overtime)}`,
      totalElapsedSecs: totalElapsed,
      remainingSecs: remaining,
      overtimeSecs: overtime,
    };
  }

  return {
    mode: 'countdown',
    display: formatDuration(remaining),
    totalElapsedSecs: totalElapsed,
    remainingSecs: remaining,
    overtimeSecs: 0,
  };
}

/**
 * Formats a Date object to local YYYY-MM-DD string without UTC conversion.
 */
export function toLocalISODate(date: Date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}


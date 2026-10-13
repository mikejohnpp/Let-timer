import { describe, it, expect } from 'vitest';
import { computeTimerDisplay, formatDuration, type TimerSnapshot } from '../utils/timer';

describe('formatDuration', () => {
  it('formats seconds to mm:ss', () => {
    expect(formatDuration(0)).toBe('00:00');
    expect(formatDuration(59)).toBe('00:59');
    expect(formatDuration(65)).toBe('01:05');
    expect(formatDuration(1500)).toBe('25:00');
  });

  it('formats hours to hh:mm:ss', () => {
    expect(formatDuration(3600)).toBe('1:00:00');
    expect(formatDuration(3665)).toBe('1:01:05');
  });
});

describe('computeTimerDisplay', () => {
  const baseTask = {
    id: 1,
    name: 'Test Task',
    workspace_id: 1,
    priority: 1,
    status: 'in-progress',
    run_no: 0,
    created_at: '2026-10-13T10:00:00Z',
    updated_at: '2026-10-13T10:00:00Z',
  };

  it('returns idle mode when snapshot is null or idle', () => {
    expect(computeTimerDisplay(null, 1000, 2000).mode).toBe('idle');
    const idleSnapshot: TimerSnapshot = {
      task: baseTask,
      state: 'idle',
      elapsed_secs: 0,
      remaining_secs: null,
      workspace_name: 'Default',
    };
    expect(computeTimerDisplay(idleSnapshot, 1000, 2000).mode).toBe('idle');
  });

  it('computes countdown for running task with estimate', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: 25 },
      state: 'running',
      elapsed_secs: 100, // 25 mins = 1500s. 1500 - 100 = 1400s left.
      remaining_secs: 1400,
      workspace_name: 'Default',
    };

    const receivedAt = 1_000_000;
    // 10 seconds later:
    const now = receivedAt + 10_000;
    const res = computeTimerDisplay(snapshot, receivedAt, now);

    expect(res.mode).toBe('countdown');
    expect(res.totalElapsedSecs).toBe(110);
    expect(res.remainingSecs).toBe(1390);
    expect(res.display).toBe('23:10'); // 1390s = 23m 10s
  });

  it('computes overtime when estimate is exceeded', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: 1 },
      state: 'running',
      elapsed_secs: 55, // 60s total
      remaining_secs: 5,
      workspace_name: 'Default',
    };

    const receivedAt = 1_000_000;
    // 15 seconds later: elapsed is 70s -> 10s overtime
    const now = receivedAt + 15_000;
    const res = computeTimerDisplay(snapshot, receivedAt, now);

    expect(res.mode).toBe('overtime');
    expect(res.totalElapsedSecs).toBe(70);
    expect(res.remainingSecs).toBe(-10);
    expect(res.overtimeSecs).toBe(10);
    expect(res.display).toBe('+00:10');
  });

  it('does not tick when task is paused', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: 25 },
      state: 'paused',
      elapsed_secs: 300,
      remaining_secs: 1200,
      workspace_name: 'Default',
    };

    const receivedAt = 1_000_000;
    const now = receivedAt + 50_000; // 50 seconds later
    const res = computeTimerDisplay(snapshot, receivedAt, now);

    expect(res.totalElapsedSecs).toBe(300); // didn't advance
    expect(res.display).toBe('20:00');
  });

  it('computes elapsed mode when task has no estimate', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: null },
      state: 'running',
      elapsed_secs: 60,
      remaining_secs: null,
      workspace_name: 'Default',
    };

    const receivedAt = 1_000_000;
    const now = receivedAt + 30_000;
    const res = computeTimerDisplay(snapshot, receivedAt, now);

    expect(res.mode).toBe('elapsed');
    expect(res.totalElapsedSecs).toBe(90);
    expect(res.display).toBe('01:30');
  });
});

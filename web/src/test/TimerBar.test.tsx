import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TimerBar } from '../components/TimerBar';
import type { TimerSnapshot } from '../hooks/useEventStream';

describe('TimerBar component', () => {
  const dummyProps = {
    onOpenFocus: vi.fn(),
    onOpenStartPicker: vi.fn(),
    onActionComplete: vi.fn(),
  };

  const baseTask = {
    id: 42,
    name: 'Write Research Paper',
    workspace_id: 1,
    priority: 1,
    status: 'in-progress',
    run_no: 0,
    created_at: '2026-10-13T10:00:00Z',
    updated_at: '2026-10-13T10:00:00Z',
  };

  it('renders idle state when nothing is running', () => {
    render(<TimerBar snapshot={null} receivedAt={Date.now()} {...dummyProps} />);
    expect(screen.getByText('Nothing running')).toBeInTheDocument();
    expect(screen.getByText('Start a task…')).toBeInTheDocument();
  });

  it('renders running countdown and task title', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: 25 },
      state: 'running',
      elapsed_secs: 100,
      remaining_secs: 1400,
      workspace_name: 'Work',
    };

    render(<TimerBar snapshot={snapshot} receivedAt={Date.now()} {...dummyProps} />);
    expect(screen.getByText('Write Research Paper')).toBeInTheDocument();
    expect(screen.getByText('Work')).toBeInTheDocument();
    expect(screen.getByText('Pause')).toBeInTheDocument();
    expect(screen.getByText('Done')).toBeInTheDocument();
  });

  it('renders overtime with +5 min button', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: 1 },
      state: 'running',
      elapsed_secs: 70, // 10s overtime
      remaining_secs: -10,
      workspace_name: 'Work',
    };

    render(<TimerBar snapshot={snapshot} receivedAt={Date.now()} {...dummyProps} />);
    expect(screen.getByText('+5 min')).toBeInTheDocument();
    expect(screen.getByText("Time's up")).toBeInTheDocument();
  });

  it('renders resume button when paused', () => {
    const snapshot: TimerSnapshot = {
      task: { ...baseTask, estimated_mins: 25 },
      state: 'paused',
      elapsed_secs: 300,
      remaining_secs: 1200,
      workspace_name: 'Work',
    };

    render(<TimerBar snapshot={snapshot} receivedAt={Date.now()} {...dummyProps} />);
    expect(screen.getByText('Resume')).toBeInTheDocument();
    expect(screen.getByText('Paused')).toBeInTheDocument();
  });
});

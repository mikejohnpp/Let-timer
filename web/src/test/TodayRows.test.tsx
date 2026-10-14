import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { TodayView } from '../views/TodayView';
import { MotionProvider, invalidateMotionTokenCache } from '../motion';
import { ToastProvider, ConfirmProvider } from '../components/ui';

// Mock apiFetch
vi.mock('../api/client', () => ({
  getStoredWorkspace: vi.fn().mockReturnValue(1),
  apiFetch: vi.fn().mockImplementation((url: string) => {
    if (url.includes('overdue=true')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([]),
      });
    }
    if (url.includes('/api/v1/tasks')) {
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve([
            {
              id: 101,
              name: 'First task',
              status: 'pending',
              priority: 1,
              workspace_id: 1,
              estimated_mins: 25,
            },
            {
              id: 102,
              name: 'Second task',
              status: 'completed',
              priority: 0,
              workspace_id: 1,
              estimated_mins: 15,
            },
          ]),
      });
    }
    return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
  }),
}));

describe('T3: Today rows layout continuity', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    invalidateMotionTokenCache();
    document.documentElement.removeAttribute('data-motion');
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }));
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    document.documentElement.removeAttribute('data-motion');
    invalidateMotionTokenCache();
    vi.clearAllMocks();
  });

  it('renders tasks with LayoutGroup and single Motion layout engine (no useFlip or .task-row-add)', async () => {
    render(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <TodayView snapshot={null} onRefreshTimer={vi.fn()} onOpenStartPicker={vi.fn()} />
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );

    // Wait for tasks to load
    await waitFor(() => {
      expect(screen.getByText('First task')).toBeTruthy();
      expect(screen.getByText('Second task')).toBeTruthy();
    });

    const taskRow = screen.getByText('First task').closest('[data-task-id="101"]');
    expect(taskRow).toBeTruthy();

    // Verify .task-row-add class is NOT used
    expect(taskRow?.classList.contains('task-row-add')).toBe(false);
  });

  it('in reduced motion mode, renders without travel offset and with layout disabled', async () => {
    document.documentElement.setAttribute('data-motion', 'reduced');
    invalidateMotionTokenCache();

    render(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <TodayView snapshot={null} onRefreshTimer={vi.fn()} onOpenStartPicker={vi.fn()} />
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );

    await waitFor(() => {
      expect(screen.getByText('First task')).toBeTruthy();
    });

    const taskRow = screen.getByText('First task').closest('[data-task-id="101"]');
    expect(taskRow).toBeTruthy();
    expect(taskRow?.classList.contains('task-row-add')).toBe(false);
  });
});

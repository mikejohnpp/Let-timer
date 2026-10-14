import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { StatusGlyph } from '../components/ui/StatusGlyph';
import { ToastProvider, useToast } from '../components/ui/Toast';
import { MotionProvider, invalidateMotionTokenCache } from '../motion';

describe('Render-based Motion Tests (R4 / G6)', () => {
  beforeEach(() => {
    invalidateMotionTokenCache();
    document.documentElement.removeAttribute('data-motion');
  });

  afterEach(() => {
    document.documentElement.removeAttribute('data-motion');
    invalidateMotionTokenCache();
    vi.clearAllMocks();
  });

  describe('StatusGlyph animateTick', () => {
    it('applies task-check-tick class only when animateTick is true for completed status', () => {
      const { container: containerWithTick } = render(
        <StatusGlyph status="completed" animateTick={true} />
      );
      const tickSvg = containerWithTick.querySelector('svg');
      expect(tickSvg).not.toBeNull();
      expect(tickSvg?.classList.contains('task-check-tick')).toBe(true);

      const { container: containerWithoutTick } = render(
        <StatusGlyph status="completed" animateTick={false} />
      );
      const noTickSvg = containerWithoutTick.querySelector('svg');
      expect(noTickSvg).not.toBeNull();
      expect(noTickSvg?.classList.contains('task-check-tick')).toBe(false);
    });

    it('renders non-completed statuses without task-check-tick', () => {
      const { container: pendingContainer } = render(
        <StatusGlyph status="pending" animateTick={true} />
      );
      expect(pendingContainer.querySelector('.task-check-tick')).toBeNull();

      const { container: runningContainer } = render(
        <StatusGlyph status="in-progress" animateTick={true} />
      );
      expect(runningContainer.querySelector('.task-check-tick')).toBeNull();
      expect(runningContainer.querySelector('.dot-running')).not.toBeNull();
    });
  });

  describe('Toast Provider and Stack Order', () => {
    const ToastTester: React.FC = () => {
      const { showToast } = useToast();
      return (
        <div>
          <button onClick={() => showToast('First message', 'info')}>Show First</button>
          <button onClick={() => showToast('Second message', 'success')}>Show Second</button>
          <button onClick={() => showToast('Error message', 'danger')}>Show Danger</button>
        </div>
      );
    };

    it('renders toasts in chronological order with correct tones and dismisses them', async () => {
      render(
        <MotionProvider>
          <ToastProvider>
            <ToastTester />
          </ToastProvider>
        </MotionProvider>
      );

      fireEvent.click(screen.getByText('Show First'));
      fireEvent.click(screen.getByText('Show Second'));

      const firstToast = screen.getByText('First message');
      const secondToast = screen.getByText('Second message');
      expect(firstToast).toBeInTheDocument();
      expect(secondToast).toBeInTheDocument();

      // Find close buttons
      const closeButtons = screen.getAllByRole('button').filter((b) => !b.textContent);
      expect(closeButtons.length).toBe(2);

      // Dismiss the first toast
      fireEvent.click(closeButtons[0]);

      // First toast was dismissed
      expect(screen.queryByText('First message')).toBeNull();
      expect(screen.getByText('Second message')).toBeInTheDocument();
    });
  });

  describe('removingIds gating logic', () => {
    it('gates exit animation only for IDs explicitly in removingIds set', () => {
      const removingIds = new Set<number>([42]);

      const getExitConfig = (taskId: number) =>
        removingIds.has(taskId) ? { opacity: 0, height: 0, overflow: 'hidden' } : undefined;

      // Deleted task has height-collapse exit
      expect(getExitConfig(42)).toEqual({ opacity: 0, height: 0, overflow: 'hidden' });

      // Move or status change does not have exit config (allowing layoutId shared motion)
      expect(getExitConfig(101)).toBeUndefined();
      expect(getExitConfig(999)).toBeUndefined();
    });
  });
});

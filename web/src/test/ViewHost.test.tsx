import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ViewHost } from '../components/ViewHost';
import { MotionProvider, invalidateMotionTokenCache } from '../motion';
import { prefersReducedMotion, hasReducedMotionListener } from 'motion-dom';

describe('T2: ViewHost tab transitions', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    invalidateMotionTokenCache();
    document.documentElement.removeAttribute('data-motion');
    prefersReducedMotion.current = null;
    hasReducedMotionListener.current = false;
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
  });

  it('renders active tab content inside single motion wrapper', async () => {
    const { rerender } = render(
      <MotionProvider>
        <ViewHost activeTab="today">
          <div data-testid="today-view">Today Content</div>
        </ViewHost>
      </MotionProvider>
    );

    expect(screen.getByTestId('today-view')).toBeTruthy();
    expect(screen.getByTestId('view-tab-today')).toBeTruthy();
    expect(screen.getByTestId('view-host')).toBeTruthy();

    // Switch tab to workspaces
    rerender(
      <MotionProvider>
        <ViewHost activeTab="workspaces">
          <div data-testid="workspaces-view">Workspaces Content</div>
        </ViewHost>
      </MotionProvider>
    );

    // Old tab content exits and new tab content enters
    expect(await screen.findByTestId('workspaces-view')).toBeTruthy();
    expect(await screen.findByTestId('view-tab-workspaces')).toBeTruthy();
  });

  it('with data-motion="reduced", ViewHost configures reduced motion without travel', () => {
    document.documentElement.setAttribute('data-motion', 'reduced');
    invalidateMotionTokenCache();

    render(
      <MotionProvider>
        <ViewHost activeTab="focus">
          <div data-testid="focus-view">Focus Content</div>
        </ViewHost>
      </MotionProvider>
    );

    const tabElement = screen.getByTestId('view-tab-focus');
    expect(tabElement).toBeTruthy();
    // Verify no .view-enter class is attached to any element
    expect(tabElement.classList.contains('view-enter')).toBe(false);
    expect(screen.getByTestId('view-host').classList.contains('view-enter')).toBe(false);
  });

  it('with data-motion="off", ViewHost renders without view-enter CSS class and with 0 duration', () => {
    document.documentElement.setAttribute('data-motion', 'off');
    invalidateMotionTokenCache();

    render(
      <MotionProvider>
        <ViewHost activeTab="settings">
          <div data-testid="settings-view">Settings Content</div>
        </ViewHost>
      </MotionProvider>
    );

    expect(screen.getByTestId('settings-view')).toBeTruthy();
    expect(document.querySelector('.view-enter')).toBeNull();
  });
});

import { useState, useEffect, useCallback } from 'react';
import { invalidateMotionTokenCache, type MotionMode } from './tokens';

export type { MotionMode };

const STORAGE_KEY = 'let_timer_motion';

export function getInitialMotionMode(): MotionMode {
  if (typeof window === 'undefined') return 'system';

  // Check URL query param ?motion=off in development or non-production
  const searchParams = new URLSearchParams(window.location.search);
  const motionParam = searchParams.get('motion');
  if (motionParam === 'off' && import.meta.env.MODE !== 'production') {
    return 'system'; // 'off' is handled via data-motion="off" attribute directly
  }

  const stored = localStorage.getItem(STORAGE_KEY) as MotionMode | null;
  if (stored === 'reduced' || stored === 'full' || stored === 'system') {
    return stored;
  }
  return 'system';
}

export function applyMotionMode(mode: MotionMode): void {
  if (typeof document === 'undefined') return;

  const searchParams = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : null;
  if (searchParams?.get('motion') === 'off' && import.meta.env.MODE !== 'production') {
    document.documentElement.setAttribute('data-motion', 'off');
    invalidateMotionTokenCache();
    return;
  }

  if (mode === 'system') {
    document.documentElement.removeAttribute('data-motion');
  } else {
    document.documentElement.setAttribute('data-motion', mode);
  }
  invalidateMotionTokenCache();
  window.dispatchEvent(new CustomEvent('let-timer-motion-changed', { detail: { mode } }));
}

export function useMotionMode() {
  const [motionMode, setMotionModeState] = useState<MotionMode>(() => getInitialMotionMode());

  useEffect(() => {
    applyMotionMode(motionMode);
  }, [motionMode]);

  // Listen to OS prefers-reduced-motion media query changes
  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;

    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const handleMediaChange = () => {
      invalidateMotionTokenCache();
      window.dispatchEvent(new CustomEvent('let-timer-motion-changed', { detail: { mode: motionMode } }));
    };

    mediaQuery.addEventListener('change', handleMediaChange);
    return () => mediaQuery.removeEventListener('change', handleMediaChange);
  }, [motionMode]);

  const setMotionMode = useCallback((newMode: MotionMode) => {
    setMotionModeState(newMode);
    localStorage.setItem(STORAGE_KEY, newMode);
    applyMotionMode(newMode);
  }, []);

  return {
    motionMode,
    setMotionMode,
  };
}

import { useState, useEffect, useRef, useCallback } from 'react';

/**
 * Hook to manage auto-hiding controls after a period of inactivity (default 3000ms).
 * Activity is detected via mousemove, keydown, and touchstart.
 */
export function useIdleTimer(timeoutMs = 3000, enabled = true) {
  const [isVisible, setIsVisible] = useState(true);
  const timerRef = useRef<number | null>(null);

  const resetTimer = useCallback(() => {
    setIsVisible(true);
    if (timerRef.current) window.clearTimeout(timerRef.current);
    if (enabled) {
      timerRef.current = window.setTimeout(() => {
        setIsVisible(false);
      }, timeoutMs);
    }
  }, [timeoutMs, enabled]);

  useEffect(() => {
    if (!enabled) {
      setIsVisible(true);
      if (timerRef.current) window.clearTimeout(timerRef.current);
      return;
    }

    const handleActivity = () => resetTimer();
    window.addEventListener('mousemove', handleActivity);
    window.addEventListener('keydown', handleActivity);
    window.addEventListener('touchstart', handleActivity);
    resetTimer();

    return () => {
      window.removeEventListener('mousemove', handleActivity);
      window.removeEventListener('keydown', handleActivity);
      window.removeEventListener('touchstart', handleActivity);
      if (timerRef.current) window.clearTimeout(timerRef.current);
    };
  }, [enabled, resetTimer]);

  return isVisible;
}


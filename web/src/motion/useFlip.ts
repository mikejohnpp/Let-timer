import { useLayoutEffect, useRef } from 'react';
import { readMotionTokens } from './tokens';
import { animate } from './animate';

export interface UseFlipOptions {
  enabled?: boolean;
  duration?: number;
  easing?: string;
}

/**
 * useFlip: Container-relative FLIP (First, Last, Invert, Play) hook for animated lists.
 * Respects motion mode (instantly places on reduced/off), guards against scroll delta shifts,
 * and handles exceptions gracefully without breaking DOM rendering.
 * Ref: docs/motion_plan.md Appendix B
 */
export function useFlip<T extends HTMLElement>(
  deps: unknown[],
  containerRef: React.RefObject<HTMLElement | null>,
  options: UseFlipOptions = {}
) {
  const nodes = useRef(new Map<string, T>());
  const prevRects = useRef(new Map<string, { left: number; top: number }>());
  const prevScroll = useRef(0);

  const register = (key: string) => (el: T | null) => {
    if (el) {
      nodes.current.set(key, el);
    } else {
      nodes.current.delete(key);
      prevRects.current.delete(key);
    }
  };

  useLayoutEffect(() => {
    try {
      const isFeatureFlagOn =
        typeof window !== 'undefined'
          ? localStorage.getItem('let_timer_motion_flip') !== 'false'
          : true;

      const enabled = options.enabled !== false && isFeatureFlagOn;
      const tokens = readMotionTokens();

      const container = containerRef.current;
      const origin = container?.getBoundingClientRect();
      const currentScroll = container?.scrollTop ?? 0;
      const scrollUnchanged = currentScroll === prevScroll.current;

      const nextRects = new Map<string, { left: number; top: number }>();

      nodes.current.forEach((el, k) => {
        if (!el || !el.isConnected) return;
        const r = el.getBoundingClientRect();
        if (origin) {
          nextRects.set(k, {
            left: r.left - origin.left,
            top: r.top - origin.top,
          });
        } else {
          nextRects.set(k, { left: r.left, top: r.top });
        }
      });

      if (enabled && !tokens.isReduced && !tokens.isOff && scrollUnchanged && prevRects.current.size > 0) {
        nextRects.forEach((b, k) => {
          const a = prevRects.current.get(k);
          const el = nodes.current.get(k);
          if (!a || !el) return;

          const dx = a.left - b.left;
          const dy = a.top - b.top;

          if (dx !== 0 || dy !== 0) {
            animate(
              el,
              [
                { transform: `translate(${dx}px, ${dy}px)` },
                { transform: 'none' },
              ],
              {
                duration: options.duration ?? tokens.durSlow,
                easing: options.easing ?? tokens.easeEmph,
              }
            );
          }
        });
      }

      prevRects.current = nextRects;
      prevScroll.current = currentScroll;
    } catch {
      // Graceful fallback to instant placement on any exception
    }
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps

  return register;
}


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
  const targetRects = useRef(new Map<string, { left: number; top: number }>());
  const activeAnimations = useRef(new Map<string, Animation>());
  const prevScroll = useRef(0);

  const registerCache = useRef(new Map<string, (el: T | null) => void>());

  const register = (key: string) => {
    let cb = registerCache.current.get(key);
    if (!cb) {
      cb = (el: T | null) => {
        if (el) {
          nodes.current.set(key, el);
        } else {
          nodes.current.delete(key);
          activeAnimations.current.get(key)?.cancel();
          activeAnimations.current.delete(key);
          targetRects.current.delete(key);
          prevRects.current.delete(key);
        }
      };
      registerCache.current.set(key, cb);
    }
    return cb;
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

      // Measure resting/target positions
      nodes.current.forEach((el, k) => {
        if (!el || !el.isConnected) return;

        const activeAnim = activeAnimations.current.get(k);
        if (activeAnim && activeAnim.playState === 'running') {
          const knownTarget = targetRects.current.get(k);
          if (knownTarget) {
            nextRects.set(k, knownTarget);
            return;
          }
        }

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

          // If an animation is already running to destination b, do not interrupt or replay
          const activeAnim = activeAnimations.current.get(k);
          if (activeAnim && activeAnim.playState === 'running') {
            const currentTarget = targetRects.current.get(k);
            if (currentTarget && Math.abs(currentTarget.left - b.left) < 1 && Math.abs(currentTarget.top - b.top) < 1) {
              return;
            }
          }

          const dx = a.left - b.left;
          const dy = a.top - b.top;

          // Only animate if movement is significant (>= 1px)
          if (Math.abs(dx) >= 1 || Math.abs(dy) >= 1) {
            if (activeAnim) {
              activeAnim.cancel();
              activeAnimations.current.delete(k);
            }

            const anim = animate(
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

            if (anim) {
              activeAnimations.current.set(k, anim);
              targetRects.current.set(k, b);
              anim.onfinish = () => {
                if (activeAnimations.current.get(k) === anim) {
                  activeAnimations.current.delete(k);
                  targetRects.current.delete(k);
                }
              };
            }
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


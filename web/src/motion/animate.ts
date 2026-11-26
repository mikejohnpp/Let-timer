/**
 * Web Animations API wrapper that automatically respects motion mode.
 * - Under reduced motion: strips transform/scale movements, leaving opacity-only transitions with 100ms duration.
 * - Under motion=off: runs with 0ms duration for deterministic testing.
 * Ref: docs/motion_plan.md §3, §8
 */

import { readMotionTokens } from './tokens';

export interface AnimateOptions extends KeyframeAnimationOptions {
  allowTravelUnderReduced?: boolean;
}

export function animate(
  element: HTMLElement | null,
  keyframes: Keyframe[] | PropertyIndexedKeyframes,
  options?: number | AnimateOptions
): Animation | null {
  if (!element || typeof element.animate !== 'function') {
    return null;
  }

  const tokens = readMotionTokens();

  const duration = typeof options === 'number' ? options : options?.duration;
  const allowTravel = typeof options === 'object' && options?.allowTravelUnderReduced;
  const opts: KeyframeAnimationOptions = typeof options === 'object' ? { ...options } : {};

  if (tokens.isOff) {
    opts.duration = 0;
    return element.animate(keyframes, opts);
  }

  if (tokens.isReduced) {
    opts.duration = Math.min(typeof duration === 'number' ? duration : 100, 100);
    opts.easing = 'ease-out';

    // Filter out transform and motion-related travel unless explicitly permitted
    let sanitizedKeyframes: Keyframe[] | PropertyIndexedKeyframes = keyframes;

    if (Array.isArray(keyframes)) {
      sanitizedKeyframes = keyframes.map((frame) => {
        const copy = { ...frame };
        if (!allowTravel) {
          delete copy.transform;
          delete copy.translate;
          delete copy.scale;
        }
        return copy;
      });
    }

    return element.animate(sanitizedKeyframes, opts);
  }

  // Normal / Full motion mode
  return element.animate(keyframes, options);
}

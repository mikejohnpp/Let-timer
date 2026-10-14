import { useMotionTokens } from './MotionProvider';
import { parseBezier, type BezierDefinition } from './tokens';

export type Kind = 'instant' | 'fast' | 'base' | 'slow' | 'ambient';

export interface MotionTransitionOptions {
  exit?: boolean;
  ease?: 'out' | 'inout' | 'emph';
}

export interface MotionTransitionResult {
  duration: number;
  ease: BezierDefinition;
}

export function useMotionTransition(
  kind: Kind,
  opts?: MotionTransitionOptions
): MotionTransitionResult {
  const t = useMotionTokens();
  const ms = {
    instant: t.durInstant,
    fast: t.durFast,
    base: t.durBase,
    slow: t.durSlow,
    ambient: t.durAmbient,
  }[kind];
  const eased = {
    out: t.easeOut,
    inout: t.easeInout,
    emph: t.easeEmph,
  }[opts?.ease ?? 'out'];

  return {
    duration: t.mode === 'off' ? 0 : (opts?.exit ? ms * t.exitRatio : ms) / 1000,
    ease: parseBezier(eased),
  };
}

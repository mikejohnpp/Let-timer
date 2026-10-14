/**
 * Single source of truth for reading motion tokens in TypeScript.
 * Reads CSS variables from document element, caches values, and refreshes on mode change.
 * Ref: docs/motion_plan.md §5 and §8
 */

export interface MotionTokens {
  durInstant: number;
  durFast: number;
  durBase: number;
  durSlow: number;
  durAmbient: number;
  durScenic: number;
  easeOut: string;
  easeInout: string;
  easeEmph: string;
  travelS: number;
  travelM: number;
  exitRatio: number;
  isReduced: boolean;
  isOff: boolean;
}

let cachedTokens: MotionTokens | null = null;

function parseMs(val: string, fallback: number): number {
  if (!val) return fallback;
  const trimmed = val.trim();
  if (trimmed.endsWith('ms')) return parseFloat(trimmed);
  if (trimmed.endsWith('s')) return parseFloat(trimmed) * 1000;
  const num = parseFloat(trimmed);
  return isNaN(num) ? fallback : num;
}

function parsePx(val: string, fallback: number): number {
  if (!val) return fallback;
  const trimmed = val.trim();
  if (trimmed.endsWith('px')) return parseFloat(trimmed);
  const num = parseFloat(trimmed);
  return isNaN(num) ? fallback : num;
}

export function readMotionTokens(forceRefresh = false): MotionTokens {
  if (cachedTokens && !forceRefresh) {
    return cachedTokens;
  }

  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return {
      durInstant: 80,
      durFast: 120,
      durBase: 200,
      durSlow: 340,
      durAmbient: 460,
      durScenic: 1200,
      easeOut: 'cubic-bezier(0.2, 0, 0, 1)',
      easeInout: 'cubic-bezier(0.45, 0, 0.25, 1)',
      easeEmph: 'cubic-bezier(0.3, 0, 0, 1)',
      travelS: 8,
      travelM: 28,
      exitRatio: 0.7,
      isReduced: false,
      isOff: false,
    };
  }

  const computed = window.getComputedStyle(document.documentElement);
  const dataMotion = document.documentElement.getAttribute('data-motion');
  const prefersReduced = typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;

  const isOff = dataMotion === 'off';
  const isReduced = isOff || dataMotion === 'reduced' || (dataMotion !== 'full' && prefersReduced);

  cachedTokens = {
    durInstant: parseMs(computed.getPropertyValue('--dur-instant'), 80),
    durFast: parseMs(computed.getPropertyValue('--dur-fast'), 120),
    durBase: parseMs(computed.getPropertyValue('--dur-base'), 200),
    durSlow: parseMs(computed.getPropertyValue('--dur-slow'), 340),
    durAmbient: parseMs(computed.getPropertyValue('--dur-ambient'), 460),
    durScenic: parseMs(computed.getPropertyValue('--dur-scenic'), 1200),
    easeOut: computed.getPropertyValue('--ease-out').trim() || 'cubic-bezier(0.2, 0, 0, 1)',
    easeInout: computed.getPropertyValue('--ease-inout').trim() || 'cubic-bezier(0.45, 0, 0.25, 1)',
    easeEmph: computed.getPropertyValue('--ease-emph').trim() || 'cubic-bezier(0.3, 0, 0, 1)',
    travelS: parsePx(computed.getPropertyValue('--travel-s'), 8),
    travelM: parsePx(computed.getPropertyValue('--travel-m'), 28),
    exitRatio: parseFloat(computed.getPropertyValue('--exit-ratio')) || 0.7,
    isReduced,
    isOff,
  };

  return cachedTokens;
}

export function invalidateMotionTokenCache(): void {
  cachedTokens = null;
}

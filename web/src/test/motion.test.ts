import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'fs';
import path from 'path';
import { readMotionTokens, invalidateMotionTokenCache } from '../motion/tokens';
import { applyMotionMode } from '../motion/useMotionMode';

describe('Motion Foundations (Phase M0)', () => {
  const cssPath = path.resolve(__dirname, '../styles/motion.css');
  const cssContent = fs.readFileSync(cssPath, 'utf8');

  it('declares all required motion tokens in motion.css', () => {
    expect(cssContent).toContain('--dur-instant: 80ms;');
    expect(cssContent).toContain('--dur-fast: 120ms;');
    expect(cssContent).toContain('--dur-base: 200ms;');
    expect(cssContent).toContain('--dur-slow: 340ms;');
    expect(cssContent).toContain('--dur-ambient: 460ms;');
    expect(cssContent).toContain('--dur-scenic: 1200ms;');

    expect(cssContent).toContain('--ease-out:   cubic-bezier(0.2, 0, 0, 1);');
    expect(cssContent).toContain('--ease-inout: cubic-bezier(0.45, 0, 0.25, 1);');
    expect(cssContent).toContain('--ease-emph:  cubic-bezier(0.3, 0, 0, 1);');

    expect(cssContent).toContain('--travel-s: 8px;');
    expect(cssContent).toContain('--travel-m: 28px;');
    expect(cssContent).toContain('--exit-ratio: 0.7;');
  });

  it('has identical reduced-motion token values in explicit attribute and media query blocks', () => {
    // Extract :root[data-motion="reduced"] block
    const reducedAttrMatch = cssContent.match(/:root\[data-motion="reduced"\]\s*\{([^}]+)\}/);
    expect(reducedAttrMatch).toBeTruthy();
    const attrBlock = reducedAttrMatch![1].replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ').trim();

    // Extract @media (prefers-reduced-motion: reduce) inner block
    const mediaMatch = cssContent.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*:root:not\(\[data-motion="full"\]\)\s*\{([^}]+)\}/);
    expect(mediaMatch).toBeTruthy();
    const mediaBlock = mediaMatch![1].replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ').trim();

    expect(attrBlock).toEqual(mediaBlock);
  });

  it('zeroes all durations under data-motion="off"', () => {
    const offMatch = cssContent.match(/:root\[data-motion="off"\]\s*\{([^}]+)\}/);
    expect(offMatch).toBeTruthy();
    const offBlock = offMatch![1];
    expect(offBlock).toContain('--dur-instant: 0ms;');
    expect(offBlock).toContain('--dur-fast: 0ms;');
    expect(offBlock).toContain('--dur-base: 0ms;');
    expect(offBlock).toContain('--dur-slow: 0ms;');
    expect(offBlock).toContain('--dur-ambient: 0ms;');
    expect(offBlock).toContain('--dur-scenic: 0ms;');
    expect(offBlock).toContain('--travel-s: 0px;');
    expect(offBlock).toContain('--travel-m: 0px;');
  });

  describe('readMotionTokens and mode changes', () => {
    beforeEach(() => {
      document.documentElement.removeAttribute('data-motion');
      invalidateMotionTokenCache();
    });

    afterEach(() => {
      document.documentElement.removeAttribute('data-motion');
      invalidateMotionTokenCache();
    });

    it('returns default fallback tokens in test jsdom environment', () => {
      const tokens = readMotionTokens(true);
      expect(tokens.durFast).toBeGreaterThanOrEqual(100);
      expect(tokens.easeOut).toContain('cubic-bezier');
      expect(tokens.travelS).toBeDefined();
    });

    it('updates isReduced when data-motion="reduced" is set', () => {
      applyMotionMode('reduced');
      const tokens = readMotionTokens(true);
      expect(tokens.isReduced).toBe(true);
    });

    it('updates isOff when data-motion="off" is set', () => {
      document.documentElement.setAttribute('data-motion', 'off');
      const tokens = readMotionTokens(true);
      expect(tokens.isOff).toBe(true);
      expect(tokens.isReduced).toBe(true);
    });
  });
});

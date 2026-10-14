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

  describe('Overlays and Micro-interactions (Phase M1)', () => {
    it('disables active scale on buttons in reduced-motion CSS', () => {
      expect(cssContent).toContain(':root[data-motion="reduced"] button:active');
      expect(cssContent).toContain('transform: none !important;');
    });
  });

  describe('Timer Pill and Lifecycle Feedback (Phase M2)', () => {
    it('declares 7-iteration breathe and 2-pulse ring animations in motion.css', () => {
      expect(cssContent).toContain('.dot-running {');
      expect(cssContent).toContain('animation: breathe 4s var(--ease-inout) 7;');
      expect(cssContent).toContain('.ring-pulse {');
      expect(cssContent).toContain('animation: ring-pulse 1.1s var(--ease-out) 2;');
    });

    it('disables breathing and ring pulse in reduced and off motion', () => {
      expect(cssContent).toContain(':root[data-motion="reduced"] .dot-running');
      expect(cssContent).toContain(':root[data-motion="reduced"] .ring-pulse');
      expect(cssContent).toContain('animation: none !important;');
    });

    it('disables progress bar transitions under reduced-motion CSS', () => {
      expect(cssContent).toContain(':root[data-motion="reduced"] .progress-bar-fill');
      expect(cssContent).toContain('transition: none !important;');
    });
  });

  describe('Task Flow and FLIP (Phase M3)', () => {
    it('declares draw-tick and slide-date keyframes in motion.css', () => {
      expect(cssContent).toContain('@keyframes draw-tick');
      expect(cssContent).toContain('@keyframes slide-date-next');
      expect(cssContent).toContain('@keyframes slide-date-prev');
      expect(cssContent).toContain('.task-check-tick');
    });

    it('disables task-check-tick animation in reduced and off motion', () => {
      expect(cssContent).toContain(':root[data-motion="reduced"] .task-check-tick');
      expect(cssContent).toContain(':root[data-motion="off"] .task-check-tick');
    });
  });

  describe('Focus and Ambience (Phase M4)', () => {
    it('attaches theme-transition class during theme/palette change (A28)', () => {
      expect(cssContent).toContain('.theme-transition');
      expect(cssContent).toContain('transition: background-color 250ms var(--ease-out)');
      expect(cssContent).toContain(':root[data-motion="reduced"] .theme-transition');
    });

    it('declares 900ms equalizer bars in motion.css (A29)', () => {
      expect(cssContent).toContain('@keyframes eq-bar-1');
      expect(cssContent).toContain('animation: eq-bar-1 900ms var(--ease-inout) infinite;');
      expect(cssContent).toContain(':root[data-motion="reduced"] .eq-bar-1');
    });

    it('declares 1.6s skeleton-shimmer with 300ms delay in motion.css (A31)', () => {
      expect(cssContent).toContain('@keyframes shimmer');
      expect(cssContent).toContain('animation: shimmer 1.6s var(--ease-inout) infinite;');
      expect(cssContent).toContain(':root[data-motion="reduced"] .skeleton-shimmer');
    });
  });

  describe('View Transitions and Polish (Phase M5)', () => {
    it('has removed .view-enter in favor of ViewHost single-engine transition (A24)', () => {
      expect(cssContent).not.toContain('.view-enter {');
      expect(cssContent).not.toContain('@keyframes view-enter');
    });

    it('isolates wallpaper with view-transition-name: none', () => {
      expect(cssContent).toContain('.wallpaper-layers {\n  view-transition-name: none;\n}');
    });

    it('enforces calm budget: no raw durations or easings outside motion.css and tokens.ts', () => {
      const srcDir = path.resolve(__dirname, '..');
      const files: string[] = [];
      const walk = (dir: string) => {
        fs.readdirSync(dir, { withFileTypes: true }).forEach((entry) => {
          const res = path.resolve(dir, entry.name);
          if (entry.isDirectory()) {
            if (entry.name !== 'node_modules' && entry.name !== 'test') walk(res);
          } else if (entry.name.endsWith('.tsx') || entry.name.endsWith('.ts') || entry.name.endsWith('.css')) {
            if (!res.endsWith('motion.css') && !res.endsWith('tokens.ts')) {
              files.push(res);
            }
          }
        });
      };
      walk(srcDir);

      files.forEach((file) => {
        const content = fs.readFileSync(file, 'utf8');
        expect(content).not.toMatch(/transition-duration:\s*[0-9]/);
        expect(content).not.toMatch(/cubic-bezier\(/);
        expect(content).not.toMatch(/animation:\s*.*[0-9]+ms/);
      });
    });
  });
});

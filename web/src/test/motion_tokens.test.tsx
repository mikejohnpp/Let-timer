import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, renderHook } from '@testing-library/react';
import { useReducedMotionConfig } from 'motion/react';
import {
  parseBezier,
  invalidateMotionTokenCache,
  MotionProvider,
  useMotionTokens,
  useMotionTransition,
} from '../motion';
import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';

import { prefersReducedMotion, hasReducedMotionListener } from 'motion-dom';

describe('T1: Motion Foundation - tokens, provider, hook, and lint', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    invalidateMotionTokenCache();
    document.documentElement.removeAttribute('data-motion');
    prefersReducedMotion.current = null;
    hasReducedMotionListener.current = false;
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    document.documentElement.removeAttribute('data-motion');
    invalidateMotionTokenCache();
  });

  describe('parseBezier', () => {
    it('correctly parses cubic-bezier definitions into tuples', () => {
      expect(parseBezier('cubic-bezier(0.2, 0, 0, 1)')).toEqual([0.2, 0, 0, 1]);
      expect(parseBezier('cubic-bezier(0.45, 0, 0.25, 1)')).toEqual([0.45, 0, 0.25, 1]);
      expect(parseBezier('cubic-bezier(0.3, 0, 0, 1)')).toEqual([0.3, 0, 0, 1]);
    });

    it('falls back gracefully on empty or invalid inputs', () => {
      expect(parseBezier('')).toEqual([0.2, 0, 0, 1]);
      expect(parseBezier('linear')).toEqual([0.2, 0, 0, 1]);
    });
  });

  describe('useMotionTransition', () => {
    it('returns token-derived values in seconds with parsed bezier ease', () => {
      const { result } = renderHook(() => useMotionTransition('base'), {
        wrapper: MotionProvider,
      });
      // Default base is 200ms -> 0.2s
      expect(result.current.duration).toBe(0.2);
      expect(result.current.ease).toEqual([0.2, 0, 0, 1]);
    });

    it('applies exitRatio (0.7) when exit is true', () => {
      const { result } = renderHook(() => useMotionTransition('base', { exit: true }), {
        wrapper: MotionProvider,
      });
      // 200ms * 0.7 = 140ms -> 0.14s
      expect(result.current.duration).toBeCloseTo(0.14, 3);
    });

    it('returns duration 0 when data-motion="off"', () => {
      document.documentElement.setAttribute('data-motion', 'off');
      invalidateMotionTokenCache();

      const { result } = renderHook(() => useMotionTransition('base'), {
        wrapper: MotionProvider,
      });
      expect(result.current.duration).toBe(0);
    });
  });

  describe('MotionProvider mode mapping and useReducedMotionConfig', () => {
    const TestConsumer = () => {
      const reducedConfig = useReducedMotionConfig();
      const tokens = useMotionTokens();
      return (
        <div>
          <div data-testid="is-reduced">{String(reducedConfig)}</div>
          <div data-testid="tokens-reduced">{String(tokens.isReduced)}</div>
          <div data-testid="tokens-mode">{tokens.mode}</div>
        </div>
      );
    };

    it('with data-motion="full" and OS reduce motion true, child useReducedMotionConfig() is false (overrides OS)', () => {
      document.documentElement.setAttribute('data-motion', 'full');
      invalidateMotionTokenCache();

      window.matchMedia = vi.fn().mockImplementation((query) => ({
        matches: query.includes('reduce'),
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));

      render(
        <MotionProvider>
          <TestConsumer />
        </MotionProvider>
      );

      expect(screen.getByTestId('is-reduced').textContent).toBe('false');
      expect(screen.getByTestId('tokens-reduced').textContent).toBe('false');
      expect(screen.getByTestId('tokens-mode').textContent).toBe('full');
    });

    it('with data-motion="reduced", child useReducedMotionConfig() is true', () => {
      document.documentElement.setAttribute('data-motion', 'reduced');
      invalidateMotionTokenCache();

      window.matchMedia = vi.fn().mockImplementation((query) => ({
        matches: false,
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));

      render(
        <MotionProvider>
          <TestConsumer />
        </MotionProvider>
      );

      expect(screen.getByTestId('is-reduced').textContent).toBe('true');
      expect(screen.getByTestId('tokens-reduced').textContent).toBe('true');
      expect(screen.getByTestId('tokens-mode').textContent).toBe('reduced');
    });

    it('with data-motion="system", child follows OS media query', () => {
      document.documentElement.removeAttribute('data-motion');
      invalidateMotionTokenCache();

      // Case A: OS reduced = true
      window.matchMedia = vi.fn().mockImplementation((query) => ({
        matches: query.includes('reduce'),
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));

      const { unmount } = render(
        <MotionProvider>
          <TestConsumer />
        </MotionProvider>
      );
      expect(screen.getByTestId('is-reduced').textContent).toBe('true');
      expect(screen.getByTestId('tokens-reduced').textContent).toBe('true');
      unmount();

      // Case B: OS reduced = false
      invalidateMotionTokenCache();
      prefersReducedMotion.current = null;
      hasReducedMotionListener.current = false;
      window.matchMedia = vi.fn().mockImplementation(() => ({
        matches: false,
        media: '',
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));

      render(
        <MotionProvider>
          <TestConsumer />
        </MotionProvider>
      );
      expect(screen.getByTestId('is-reduced').textContent).toBe('false');
      expect(screen.getByTestId('tokens-reduced').textContent).toBe('false');
    });
  });

  describe('check-motion-tokens script', () => {
    it('exits non-zero on a fixture containing raw duration: 0.2', () => {
      const scriptPath = path.resolve(__dirname, '../../scripts/check-motion-tokens.mjs');
      const tempFixtureDir = path.resolve(__dirname, '../temp_fixture_test');
      fs.mkdirSync(tempFixtureDir, { recursive: true });
      const tempFixtureFile = path.join(tempFixtureDir, 'BadComponent.tsx');
      fs.writeFileSync(tempFixtureFile, 'const x = { duration: 0.2 };');

      try {
        let threw = false;
        try {
          execSync(`node "${scriptPath}"`, {
            cwd: path.resolve(__dirname, '../..'),
            stdio: 'pipe',
          });
        } catch (e: any) {
          threw = true;
          expect(e.status).not.toBe(0);
        }
        expect(threw).toBe(true);
      } finally {
        fs.rmSync(tempFixtureDir, { recursive: true, force: true });
      }
    });
  });
});

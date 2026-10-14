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
    it('applies Radix data-state keyframes to Drawer', () => {
      const drawerCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Drawer.tsx'), 'utf8');
      expect(drawerCode).toContain('data-[state=open]:animate-[slide-in-right_var(--dur-base)_var(--ease-out)]');
      expect(drawerCode).toContain('data-[state=closed]:animate-[slide-out-right_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]');
      expect(drawerCode).toContain('data-[state=open]:animate-[rise-in_var(--dur-base)_var(--ease-out)]');
      expect(drawerCode).toContain('data-[state=closed]:animate-[rise-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]');
      expect(drawerCode).toContain('data-[state=open]:animate-[fade-in_var(--dur-base)_var(--ease-out)]');
      expect(drawerCode).toContain('data-[state=closed]:animate-[fade-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]');
    });

    it('applies Radix data-state keyframes to Modal', () => {
      const modalCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Modal.tsx'), 'utf8');
      expect(modalCode).toContain('data-[state=open]:animate-[rise-in_var(--dur-base)_var(--ease-out)]');
      expect(modalCode).toContain('data-[state=closed]:animate-[rise-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]');
      expect(modalCode).toContain('data-[state=open]:animate-[fade-in_var(--dur-base)_var(--ease-out)]');
    });

    it('applies pop-in and fade-out to Popover and DropdownMenu', () => {
      const popoverCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Popover.tsx'), 'utf8');
      expect(popoverCode).toContain('data-[state=open]:animate-[pop-in_var(--dur-fast)_var(--ease-out)]');
      expect(popoverCode).toContain('data-[state=closed]:animate-[fade-out_var(--dur-instant)_var(--ease-out)_forwards]');

      const menuCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/DropdownMenu.tsx'), 'utf8');
      expect(menuCode).toContain('data-[state=open]:animate-[pop-in_var(--dur-fast)_var(--ease-out)]');
      expect(menuCode).toContain('data-[state=closed]:animate-[fade-out_var(--dur-instant)_var(--ease-out)_forwards]');
    });

    it('configures Toast with 4000ms duration and rise-in animation', () => {
      const toastCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Toast.tsx'), 'utf8');
      expect(toastCode).toContain('duration={4000}');
      expect(toastCode).toContain('data-[state=open]:animate-[rise-in_var(--dur-base)_var(--ease-out)]');
      expect(toastCode).toContain('data-[state=closed]:animate-[fade-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]');
    });

    it('applies active:scale-[0.98] with duration tokens to Button and IconButton', () => {
      const buttonCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Button.tsx'), 'utf8');
      expect(buttonCode).toContain('active:scale-[0.98]');
      expect(buttonCode).toContain('duration-[var(--dur-fast)]');
      expect(buttonCode).toContain('active:duration-[var(--dur-instant)]');
    });

    it('disables active scale on buttons in reduced-motion CSS', () => {
      expect(cssContent).toContain(':root[data-motion="reduced"] button:active');
      expect(cssContent).toContain('transform: none !important;');
    });

    it('applies duration-base and ease-out transitions to SegmentedControl', () => {
      const segCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/SegmentedControl.tsx'), 'utf8');
      expect(segCode).toContain('duration-[var(--dur-base)]');
      expect(segCode).toContain('ease-[var(--ease-out)]');
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

    it('uses Presence helper for floating timer bar in App.tsx', () => {
      const appCode = fs.readFileSync(path.resolve(__dirname, '../App.tsx'), 'utf8');
      expect(appCode).toContain('<Presence');
      expect(appCode).toContain('enterAnimation="animate-[rise-in_var(--dur-base)_var(--ease-out)]"');
      expect(appCode).toContain('exitAnimation="animate-[rise-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]"');
    });

    it('applies fixed width, ring-pulse and ambient state transitions in TimerBar', () => {
      const timerCode = fs.readFileSync(path.resolve(__dirname, '../components/TimerBar.tsx'), 'utf8');
      expect(timerCode).toContain('duration-[var(--dur-ambient)]');
      expect(timerCode).toContain('ring-pulse');
      expect(timerCode).toContain('dot-running');
      expect(timerCode).toContain('w-[320px] sm:w-[420px] max-w-[92vw]');
    });

    it('animates daily progress bar using scaleX and token duration (A15)', () => {
      const todayCode = fs.readFileSync(path.resolve(__dirname, '../views/TodayView.tsx'), 'utf8');
      expect(todayCode).toContain('scaleX(${progressPercent / 100})');
      expect(todayCode).toContain('duration-[var(--dur-slow)]');
      expect(todayCode).toContain('progress-bar-fill');
      expect(todayCode).not.toContain('width: `${progressPercent}%`');
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

    it('uses LayoutGroup and layoutId for single-engine task transitions in TodayView', () => {
      const todayCode = fs.readFileSync(path.resolve(__dirname, '../views/TodayView.tsx'), 'utf8');
      expect(todayCode).toContain('<LayoutGroup id="today-tasks">');
      expect(todayCode).toContain('layoutId={`task-${t.id}`}');
      expect(todayCode).not.toContain('useFlip');
    });

    it('applies date slide animations in TodayView on date change', () => {
      const todayCode = fs.readFileSync(path.resolve(__dirname, '../views/TodayView.tsx'), 'utf8');
      expect(todayCode).toContain('slide-date-prev');
      expect(todayCode).toContain('slide-date-next');
    });

    it('applies task-check-tick to completed StatusGlyph (A17)', () => {
      const glyphCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/StatusGlyph.tsx'), 'utf8');
      expect(glyphCode).toContain('task-check-tick');
    });
  });

  describe('Focus and Ambience (Phase M4)', () => {
    it('uses WallpaperLayers with img.decode and dur-scenic in App and FocusView', () => {
      const appCode = fs.readFileSync(path.resolve(__dirname, '../App.tsx'), 'utf8');
      const focusCode = fs.readFileSync(path.resolve(__dirname, '../views/FocusView.tsx'), 'utf8');
      const wallCode = fs.readFileSync(path.resolve(__dirname, '../components/WallpaperLayers.tsx'), 'utf8');

      expect(appCode).toContain('<WallpaperLayers');
      expect(focusCode).toContain('<WallpaperLayers');
      expect(wallCode).toContain('img.decode');
      expect(wallCode).toContain('duration-[var(--dur-scenic)]');
    });

    it('attaches theme-transition class during theme/palette change (A28)', () => {
      expect(cssContent).toContain('.theme-transition');
      expect(cssContent).toContain('transition: background-color 250ms var(--ease-out)');
      expect(cssContent).toContain(':root[data-motion="reduced"] .theme-transition');
    });

    it('declares 900ms equalizer bars in motion.css and uses eq-playing in SoundWave (A29)', () => {
      expect(cssContent).toContain('@keyframes eq-bar-1');
      expect(cssContent).toContain('animation: eq-bar-1 900ms var(--ease-inout) infinite;');
      expect(cssContent).toContain(':root[data-motion="reduced"] .eq-bar-1');

      const soundCode = fs.readFileSync(path.resolve(__dirname, '../components/SoundWave.tsx'), 'utf8');
      expect(soundCode).toContain('eq-playing');
      expect(soundCode).toContain('eq-bar-1');
      expect(soundCode).not.toContain('style={{ animationDuration');
    });

    it('declares 1.6s skeleton-shimmer with 300ms delay in Skeleton component (A31)', () => {
      expect(cssContent).toContain('@keyframes shimmer');
      expect(cssContent).toContain('animation: shimmer 1.6s var(--ease-inout) infinite;');
      expect(cssContent).toContain(':root[data-motion="reduced"] .skeleton-shimmer');

      const skelCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Skeleton.tsx'), 'utf8');
      expect(skelCode).toContain('skeleton-shimmer');
      expect(skelCode).toContain('delayMs = 300');
    });

    it('hides focus controls via opacity-only and duration-fast with focus-within:opacity-100 (A25)', () => {
      const focusCode = fs.readFileSync(path.resolve(__dirname, '../views/FocusView.tsx'), 'utf8');
      const headerCode = fs.readFileSync(path.resolve(__dirname, '../components/Header.tsx'), 'utf8');

      expect(focusCode).toContain('duration-[var(--dur-fast)]');
      expect(focusCode).toContain('focus-within:opacity-100');
      expect(headerCode).toContain('duration-[var(--dur-fast)]');
      expect(headerCode).toContain('focus-within:opacity-100');
    });

    it('renders static Live dot in ConnectionBadge without pulse (A30/DM4)', () => {
      const badgeCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/ConnectionBadge.tsx'), 'utf8');
      expect(badgeCode).not.toContain('animate-pulse');
      expect(badgeCode).toContain("label: 'Live'");
    });

    it('enforces calm budget: no infinite Tailwind animations (animate-ping/pulse/bounce) in UI components', () => {
      const todayCode = fs.readFileSync(path.resolve(__dirname, '../views/TodayView.tsx'), 'utf8');
      const badgeCode = fs.readFileSync(path.resolve(__dirname, '../components/ui/Badge.tsx'), 'utf8');

      expect(todayCode).not.toContain('animate-ping');
      expect(todayCode).toContain('dot-running');

      expect(badgeCode).not.toContain('animate-ping');
      expect(badgeCode).toContain('ring-pulse');
    });
  });

  describe('View Transitions and Polish (Phase M5)', () => {
    it('has removed .view-enter in favor of ViewHost single-engine transition (A24)', () => {
      expect(cssContent).not.toContain('.view-enter {');
      expect(cssContent).not.toContain('@keyframes view-enter');
    });

    it('isolates wallpaper with view-transition-name: none and applies view-enter to tab views', () => {
      expect(cssContent).toContain('.wallpaper-layers {\n  view-transition-name: none;\n}');

      const wallCode = fs.readFileSync(path.resolve(__dirname, '../components/WallpaperLayers.tsx'), 'utf8');
      expect(wallCode).toContain("viewTransitionName: 'none'");
    });

    it('wraps current view in App.tsx with ViewHost for single-engine flicker-free tab transitions', () => {
      const appCode = fs.readFileSync(path.resolve(__dirname, '../App.tsx'), 'utf8');
      expect(appCode).toContain('<ViewHost activeTab={currentView} className="h-full">');
      expect(appCode).not.toContain('view-enter');
    });

    it('nests animated container inside positioned container in Presence for rock-solid centering', () => {
      const presenceCode = fs.readFileSync(path.resolve(__dirname, '../motion/Presence.tsx'), 'utf8');
      expect(presenceCode).toContain('<div className={className}>');
      expect(presenceCode).toContain('data-state={animState}');
      expect(presenceCode).toContain('className={animState === \'open\' ? enterAnimation : exitAnimation}');
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

import { describe, it, expect } from 'vitest';
import { rgbToHsl, hslToHex, hexToRgb, generateDynamicPalette } from '../utils/palette';
import { contrastRatio } from '../utils/contrast';

describe('palette color utilities', () => {
  it('converts rgb to hsl and back to hex', () => {
    const [h, s, l] = rgbToHsl(255, 0, 0);
    expect(h).toBe(0);
    expect(s).toBe(100);
    expect(l).toBe(50);
    expect(hslToHex(0, 100, 50)).toBe('#ff0000');
  });

  it('parses 3-character and 6-character hex', () => {
    expect(hexToRgb('#fff')).toEqual([255, 255, 255]);
    expect(hexToRgb('#1b633e')).toEqual([27, 99, 62]);
    expect(hexToRgb('invalid')).toBeNull();
  });

  it('generates full WCAG AA compliant palette for light mode', () => {
    const colors = ['#e11d48', '#2563eb', '#16a34a', '#ca8a04', '#9333ea'];

    for (const color of colors) {
      const palette = generateDynamicPalette(color, false);
      expect(palette.bg).toBeDefined();
      expect(palette.surface).toBeDefined();
      expect(palette.surfaceHover).toBeDefined();
      expect(palette.border).toBeDefined();
      expect(palette.borderStrong).toBeDefined();
      expect(palette.accent).toBeDefined();
      expect(palette.onAccent).toBe('#ffffff');

      // Accent button: white on accent must pass AA (>= 4.5:1)
      const buttonContrast = contrastRatio(palette.onAccent, palette.accent);
      expect(buttonContrast).toBeGreaterThanOrEqual(4.5);

      // Accent text on light surface: must pass AA (>= 4.5:1)
      const accentTextContrast = contrastRatio(palette.accentText, palette.surface);
      expect(accentTextContrast).toBeGreaterThanOrEqual(4.5);

      // Main text on surface: must pass AAA (>= 7.0:1)
      const textContrast = contrastRatio(palette.text, palette.surface);
      expect(textContrast).toBeGreaterThanOrEqual(7.0);

      // Muted text on surface: must pass AA (>= 4.5:1)
      const mutedContrast = contrastRatio(palette.muted, palette.surface);
      expect(mutedContrast).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('generates full WCAG AA compliant palette for dark mode', () => {
    const colors = ['#e11d48', '#2563eb', '#16a34a', '#ca8a04', '#9333ea'];

    for (const color of colors) {
      const palette = generateDynamicPalette(color, true);
      expect(palette.bg).toBeDefined();
      expect(palette.surface).toBeDefined();
      expect(palette.surfaceHover).toBeDefined();
      expect(palette.border).toBeDefined();
      expect(palette.borderStrong).toBeDefined();
      expect(palette.accent).toBeDefined();
      expect(palette.onAccent).toBe('#ffffff');

      // Accent text on dark surface/bg: must pass AA (>= 4.5:1)
      const accentTextContrast = contrastRatio(palette.accentText, palette.bg);
      expect(accentTextContrast).toBeGreaterThanOrEqual(4.5);

      // Main text on surface: must pass AAA (>= 7.0:1)
      const textContrast = contrastRatio(palette.text, palette.surface);
      expect(textContrast).toBeGreaterThanOrEqual(7.0);

      // Muted text on surface: must pass AA (>= 4.5:1)
      const mutedContrast = contrastRatio(palette.muted, palette.surface);
      expect(mutedContrast).toBeGreaterThanOrEqual(4.5);
    }
  });
});

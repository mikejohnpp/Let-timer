import { contrastRatio } from './contrast';

/**
 * Color extraction & WCAG-compliant dynamic palette generator.
 * Harmonizes the entire application color scheme from an image.
 */

export interface DynamicTokens {
  bg: string;
  surface: string;
  surfaceHover: string;
  border: string;
  borderStrong: string;
  text: string;
  muted: string;
  accent: string;
  accentHover: string;
  accentText: string;
  accentTint: string;
  onAccent: string;
  bgGlassOverlay: string;
}

// Convert RGB to HSL
export function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      case b:
        h = (r - g) / d + 4;
        break;
    }
    h /= 6;
  }

  return [Math.round(h * 360), Math.round(s * 100), Math.round(l * 100)];
}

// Convert HSL to HEX
export function hslToHex(h: number, s: number, l: number): string {
  s /= 100;
  l /= 100;

  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) =>
    l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));

  const toHex = (x: number) => {
    const hex = Math.round(x * 255).toString(16);
    return hex.length === 1 ? '0' + hex : hex;
  };

  return `#${toHex(f(0))}${toHex(f(8))}${toHex(f(4))}`;
}

// Parse hex to RGB
export function hexToRgb(hex: string): [number, number, number] | null {
  const clean = hex.replace('#', '').trim();
  if (clean.length === 3) {
    const r = parseInt(clean[0] + clean[0], 16);
    const g = parseInt(clean[1] + clean[1], 16);
    const b = parseInt(clean[2] + clean[2], 16);
    return [r, g, b];
  }
  if (clean.length === 6) {
    const r = parseInt(clean.substring(0, 2), 16);
    const g = parseInt(clean.substring(2, 4), 16);
    const b = parseInt(clean.substring(4, 6), 16);
    return [r, g, b];
  }
  return null;
}

/**
 * Adjusts lightness iteratively until contrast meets the required minimum ratio against target color.
 */
function calibrateLightness(
  h: number,
  s: number,
  startL: number,
  targetColor: string,
  minRatio: number,
  direction: 'darken' | 'lighten'
): string {
  let l = startL;
  const step = direction === 'darken' ? -1 : 1;
  const limit = direction === 'darken' ? 4 : 96;

  while (direction === 'darken' ? l >= limit : l <= limit) {
    const hex = hslToHex(h, s, l);
    if (contrastRatio(hex, targetColor) >= minRatio) {
      return hex;
    }
    l += step;
  }
  return hslToHex(h, s, limit);
}

/**
 * Generate a complete, cohesive WCAG AA/AAA theme palette based on a dominant image color.
 */
export function generateDynamicPalette(hexColor: string, isDark: boolean): DynamicTokens {
  const rgb = hexToRgb(hexColor) || [79, 70, 229]; // fallback indigo
  const [h, s] = rgbToHsl(rgb[0], rgb[1], rgb[2]);

  // Ensure decent saturation for brand identity
  const sat = Math.max(35, Math.min(s, 85));

  if (isDark) {
    // ─── Dark Mode Palette ──────────────────────────────────────────
    // Subtle chromatic tint on deep dark surfaces
    const bg = hslToHex(h, 20, 7);
    const surface = hslToHex(h, 22, 11);
    const surfaceHover = hslToHex(h, 24, 17);
    const border = hslToHex(h, 22, 20);
    const borderStrong = hslToHex(h, 26, 45);

    // Text guarantees high readability on dark surface
    const text = calibrateLightness(h, 14, 95, surface, 7.0, 'lighten');
    const muted = calibrateLightness(h, 15, 64, surface, 4.5, 'lighten');

    // Vibrant accent tokens
    const accent = calibrateLightness(h, sat, 45, '#ffffff', 4.5, 'darken');
    const rgbAccent = hexToRgb(accent) || [79, 70, 229];
    const [, , lAccent] = rgbToHsl(...rgbAccent);
    const accentHover = hslToHex(h, sat, Math.min(lAccent + 6, 85));
    const accentText = calibrateLightness(h, sat, 68, bg, 4.5, 'lighten');

    return {
      bg,
      surface,
      surfaceHover,
      border,
      borderStrong,
      text,
      muted,
      accent,
      accentHover,
      accentText,
      accentTint: `hsla(${h}, ${sat}%, 45%, 0.20)`,
      onAccent: '#ffffff',
      bgGlassOverlay: `hsla(${h}, 22%, 7%, 0.60)`,
    };
  } else {
    // ─── Light Mode Palette ─────────────────────────────────────────
    // Subtle chromatic tint on clean bright surfaces
    const bg = hslToHex(h, 12, 97);
    const surface = hslToHex(h, 14, 99);
    const surfaceHover = hslToHex(h, 16, 93);
    const border = hslToHex(h, 18, 86);
    const borderStrong = hslToHex(h, 25, 55);

    // Text guarantees high readability on bright surface
    const text = calibrateLightness(h, 25, 11, surface, 7.0, 'darken');
    const muted = calibrateLightness(h, 18, 40, surface, 4.5, 'darken');

    // Vibrant accent tokens
    const accent = calibrateLightness(h, sat, 35, '#ffffff', 4.5, 'darken');
    const rgbAccent = hexToRgb(accent) || [79, 70, 229];
    const [, , lAccent] = rgbToHsl(...rgbAccent);
    const accentHover = hslToHex(h, sat, Math.max(lAccent - 5, 10));
    const accentText = calibrateLightness(h, sat, 30, surface, 4.5, 'darken');

    return {
      bg,
      surface,
      surfaceHover,
      border,
      borderStrong,
      text,
      muted,
      accent,
      accentHover,
      accentText,
      accentTint: `hsla(${h}, ${sat}%, 35%, 0.14)`,
      onAccent: '#ffffff',
      bgGlassOverlay: `hsla(${h}, 15%, 96%, 0.65)`,
    };
  }
}

/**
 * Extract dominant vibrant color from an image URL using offscreen canvas.
 */
export async function extractDominantColor(imageUrl: string): Promise<string> {
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';

    img.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        if (!ctx) {
          resolve('#4f46e5');
          return;
        }

        const size = 64;
        canvas.width = size;
        canvas.height = size;
        ctx.drawImage(img, 0, 0, size, size);

        const imageData = ctx.getImageData(0, 0, size, size).data;
        const colorBuckets: { [key: string]: { r: number; g: number; b: number; count: number; score: number } } = {};

        for (let i = 0; i < imageData.length; i += 16) { // sample every 4th pixel
          const r = imageData[i];
          const g = imageData[i + 1];
          const b = imageData[i + 2];
          const a = imageData[i + 3];

          if (a < 128) continue; // ignore transparent pixels

          const [, s, l] = rgbToHsl(r, g, b);

          // Skip nearly black or nearly white pixels
          if (l < 15 || l > 88) continue;

          // Bucket by rounding to 32
          const bucketKey = `${Math.round(r / 32) * 32},${Math.round(g / 32) * 32},${Math.round(b / 32) * 32}`;

          if (!colorBuckets[bucketKey]) {
            colorBuckets[bucketKey] = { r, g, b, count: 0, score: 0 };
          }

          colorBuckets[bucketKey].count++;
          // Score prioritizes saturation (vibrancy) over pure frequency
          colorBuckets[bucketKey].score += (s / 100) * 2 + 1;
        }

        let bestBucket = null;
        let maxScore = -1;

        for (const key in colorBuckets) {
          if (colorBuckets[key].score > maxScore) {
            maxScore = colorBuckets[key].score;
            bestBucket = colorBuckets[key];
          }
        }

        if (bestBucket) {
          resolve(hslToHex(...rgbToHsl(bestBucket.r, bestBucket.g, bestBucket.b)));
        } else {
          resolve('#4f46e5');
        }
      } catch {
        resolve('#4f46e5');
      }
    };

    img.onerror = () => {
      resolve('#4f46e5');
    };

    img.src = imageUrl;
  });
}

const DYNAMIC_CSS_VARS = [
  '--bg',
  '--surface',
  '--surface-hover',
  '--border',
  '--border-strong',
  '--text',
  '--muted',
  '--accent',
  '--accent-hover',
  '--accent-text',
  '--accent-tint',
  '--on-accent',
  '--bg-glass-overlay',
];

/**
 * Apply or reset dynamic palette across the entire application DOM.
 */
export function applyDynamicPalette(hexColor: string | null) {
  const root = document.documentElement;
  if (!hexColor) {
    for (const v of DYNAMIC_CSS_VARS) {
      root.style.removeProperty(v);
    }
    root.removeAttribute('data-dynamic-palette');
    return;
  }

  // Detect dark mode
  const currentTheme = root.getAttribute('data-theme');
  const isDark =
    currentTheme === 'dark' ||
    (!currentTheme && window.matchMedia('(prefers-color-scheme: dark)').matches);

  const tokens = generateDynamicPalette(hexColor, isDark);

  root.style.setProperty('--bg', tokens.bg);
  root.style.setProperty('--surface', tokens.surface);
  root.style.setProperty('--surface-hover', tokens.surfaceHover);
  root.style.setProperty('--border', tokens.border);
  root.style.setProperty('--border-strong', tokens.borderStrong);
  root.style.setProperty('--text', tokens.text);
  root.style.setProperty('--muted', tokens.muted);
  root.style.setProperty('--accent', tokens.accent);
  root.style.setProperty('--accent-hover', tokens.accentHover);
  root.style.setProperty('--accent-text', tokens.accentText);
  root.style.setProperty('--accent-tint', tokens.accentTint);
  root.style.setProperty('--on-accent', tokens.onAccent);
  root.style.setProperty('--bg-glass-overlay', tokens.bgGlassOverlay);

  root.setAttribute('data-dynamic-palette', 'true');
}

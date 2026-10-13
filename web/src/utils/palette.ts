import { contrastRatio } from './contrast';

/**
 * Color extraction & WCAG-compliant dynamic palette generator.
 */

export interface DynamicTokens {
  accent: string;
  accentHover: string;
  accentText: string;
  accentTint: string;
  onAccent: string;
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
  const limit = direction === 'darken' ? 5 : 95;

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
 * Generate WCAG AA compliant theme tokens based on a dominant color.
 */
export function generateDynamicPalette(hexColor: string, isDark: boolean): DynamicTokens {
  const rgb = hexToRgb(hexColor) || [79, 70, 229]; // fallback indigo
  const [h, s] = rgbToHsl(rgb[0], rgb[1], rgb[2]);

  // Ensure decent saturation for brand identity
  const sat = Math.max(40, Math.min(s, 85));

  if (isDark) {
    // Dark mode:
    // Background is dark (#0b0f19)
    // 1. Accent text must be readable on dark background (>= 4.5:1)
    const accentText = calibrateLightness(h, sat, 60, '#0b0f19', 4.5, 'lighten');

    // 2. Button background: white on accent must pass (>= 4.5:1)
    const accent = calibrateLightness(h, sat, 45, '#ffffff', 4.5, 'darken');
    const rgbAccent = hexToRgb(accent) || [79, 70, 229];
    const [, , lAccent] = rgbToHsl(...rgbAccent);
    const accentHover = hslToHex(h, sat, Math.min(lAccent + 6, 85));

    return {
      accent,
      accentHover,
      accentText,
      accentTint: `hsla(${h}, ${sat}%, 45%, 0.18)`,
      onAccent: '#ffffff',
    };
  } else {
    // Light mode:
    // Background/Surface is light (#ffffff)
    // 1. Accent text must be readable on white surface (>= 4.5:1)
    const accentText = calibrateLightness(h, sat, 40, '#ffffff', 4.5, 'darken');

    // 2. Button background: white text on accent must pass (>= 4.5:1)
    const accent = calibrateLightness(h, sat, 40, '#ffffff', 4.5, 'darken');
    const rgbAccent = hexToRgb(accent) || [79, 70, 229];
    const [, , lAccent] = rgbToHsl(...rgbAccent);
    const accentHover = hslToHex(h, sat, Math.max(lAccent - 5, 10));

    return {
      accent,
      accentHover,
      accentText,
      accentTint: `hsla(${h}, ${sat}%, 35%, 0.12)`,
      onAccent: '#ffffff',
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

/**
 * Apply or reset dynamic palette in DOM.
 */
export function applyDynamicPalette(hexColor: string | null) {
  const root = document.documentElement;
  if (!hexColor) {
    root.style.removeProperty('--accent');
    root.style.removeProperty('--accent-hover');
    root.style.removeProperty('--accent-text');
    root.style.removeProperty('--accent-tint');
    root.style.removeProperty('--on-accent');
    root.removeAttribute('data-dynamic-palette');
    return;
  }

  // Detect dark mode
  const currentTheme = root.getAttribute('data-theme');
  const isDark =
    currentTheme === 'dark' ||
    (!currentTheme && window.matchMedia('(prefers-color-scheme: dark)').matches);

  const tokens = generateDynamicPalette(hexColor, isDark);

  root.style.setProperty('--accent', tokens.accent);
  root.style.setProperty('--accent-hover', tokens.accentHover);
  root.style.setProperty('--accent-text', tokens.accentText);
  root.style.setProperty('--accent-tint', tokens.accentTint);
  root.style.setProperty('--on-accent', tokens.onAccent);
  root.setAttribute('data-dynamic-palette', 'true');
}

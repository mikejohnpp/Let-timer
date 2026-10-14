import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { contrastRatio } from '../utils/contrast';
import { readThemes, type TokenMap } from '../utils/cssTokens';

const css = readFileSync(resolve(__dirname, '../styles/tokens.css'), 'utf8');
const themes = readThemes(css);

const SURFACES = ['bg', 'surface', 'surface-hover'] as const;
const TEXT_ON_SURFACE = ['text', 'muted', 'accent-text', 'over-text', 'danger-text', 'warning-text', 'success-text'];
const NON_TEXT_ON_SURFACE = ['border-strong', 'accent-text', 'timer-over', 'danger', 'warning', 'success'];
const TINTS = ['danger-bg', 'warning-bg', 'success-bg', 'over-bg', 'info-bg'];

function tok(t: TokenMap, name: string): string {
  const v = t[name];
  if (!v || !v.startsWith('#')) throw new Error(`token --${name} missing or not a hex colour: ${v}`);
  return v;
}

const themeEntries = {
  'light (default)': themes.light,
  'dark (default)': themes.dark,
  'light (forest)': themes.forestLight,
  'dark (forest)': themes.forestDark,
  'light (tokyo-night)': themes.tokyoNightLight,
  'dark (tokyo-night)': themes.tokyoNightDark,
  'light (catppuccin)': themes.catppuccinLight,
  'dark (catppuccin)': themes.catppuccinDark,
  'light (kanagawa)': themes.kanagawaLight,
  'dark (kanagawa)': themes.kanagawaDark,
  'light (everforest)': themes.everforestLight,
  'dark (everforest)': themes.everforestDark,
};

for (const [themeName, t] of Object.entries(themeEntries)) {
  describe(`contrast (${themeName})`, () => {
    for (const s of SURFACES) {
      for (const name of TEXT_ON_SURFACE) {
        it(`--${name} on --${s} >= 4.5`, () => {
          expect(contrastRatio(tok(t, name), tok(t, s))).toBeGreaterThanOrEqual(4.5);
        });
      }
      for (const name of NON_TEXT_ON_SURFACE) {
        it(`--${name} on --${s} >= 3 (non-text)`, () => {
          expect(contrastRatio(tok(t, name), tok(t, s))).toBeGreaterThanOrEqual(3);
        });
      }
    }
    for (const [fg, bg] of [
      ['on-accent', 'accent'],
      ['on-accent', 'accent-hover'],
      ['on-over', 'timer-over'],
      ['on-danger', 'danger'],
    ]) {
      it(`--${fg} on --${bg} >= 4.5`, () => {
        expect(contrastRatio(tok(t, fg), tok(t, bg))).toBeGreaterThanOrEqual(4.5);
      });
    }
    it('white tick on --success >= 3 (non-text)', () => {
      expect(contrastRatio('#ffffff', tok(t, 'success'))).toBeGreaterThanOrEqual(3);
    });
    for (const tint of TINTS) {
      it(`--text and --muted on --${tint} >= 4.5`, () => {
        expect(contrastRatio(tok(t, 'text'), tok(t, tint))).toBeGreaterThanOrEqual(4.5);
        expect(contrastRatio(tok(t, 'muted'), tok(t, tint))).toBeGreaterThanOrEqual(4.5);
      });
    }
  });
}

describe('immersive scope (Focus over a photo)', () => {
  it('white text stays >= 4.5 over the overlay on a pure-white photo', () => {
    const m = /rgb\(0 0 0 \/ ([0-9.]+)\)/.exec(themes.light['imm-overlay'] ?? '');
    expect(m).not.toBeNull();
    const alpha = Number(m![1]);
    const c = Math.round(255 * (1 - alpha))
      .toString(16)
      .padStart(2, '0');
    const worstBg = `#${c}${c}${c}`;
    expect(contrastRatio(tok(themes.light, 'imm-text'), worstBg)).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio(tok(themes.light, 'imm-muted'), worstBg)).toBeGreaterThanOrEqual(4.5);
  });
});

describe('theme blocks', () => {
  it('prefers-color-scheme dark block matches [data-theme="dark"]', () => {
    for (const [k, v] of Object.entries(themes.dark)) {
      if (k === 'shadow-popover') continue;
      expect(themes.darkMedia[k], `--${k}`).toBe(v);
    }
    expect(themes.darkMedia['shadow-popover']).toBe(themes.dark['shadow-popover']);
  });

  it('prefers-color-scheme dark forest block matches [data-theme="dark"][data-palette="forest"]', () => {
    for (const [k, v] of Object.entries(themes.forestDark)) {
      if (k === 'shadow-popover') continue;
      expect(themes.forestDarkMedia[k], `--${k}`).toBe(v);
    }
  });

  it('prefers-color-scheme dark tokyo-night block matches [data-theme="dark"][data-palette="tokyo-night"]', () => {
    for (const [k, v] of Object.entries(themes.tokyoNightDark)) {
      if (k === 'shadow-popover') continue;
      expect(themes.tokyoNightDarkMedia[k], `--${k}`).toBe(v);
    }
  });

  it('prefers-color-scheme dark catppuccin block matches [data-theme="dark"][data-palette="catppuccin"]', () => {
    for (const [k, v] of Object.entries(themes.catppuccinDark)) {
      if (k === 'shadow-popover') continue;
      expect(themes.catppuccinDarkMedia[k], `--${k}`).toBe(v);
    }
  });

  it('prefers-color-scheme dark kanagawa block matches [data-theme="dark"][data-palette="kanagawa"]', () => {
    for (const [k, v] of Object.entries(themes.kanagawaDark)) {
      if (k === 'shadow-popover') continue;
      expect(themes.kanagawaDarkMedia[k], `--${k}`).toBe(v);
    }
  });

  it('prefers-color-scheme dark everforest block matches [data-theme="dark"][data-palette="everforest"]', () => {
    for (const [k, v] of Object.entries(themes.everforestDark)) {
      if (k === 'shadow-popover') continue;
      expect(themes.everforestDarkMedia[k], `--${k}`).toBe(v);
    }
  });
});

describe('small text typography contrast (10-12px, T5)', () => {
  for (const [themeName, t] of Object.entries(themeEntries)) {
    describe(`small text contrast (${themeName})`, () => {
      for (const s of SURFACES) {
        it(`small --muted on --${s} meets WCAG AA (>= 4.5:1)`, () => {
          const ratio = contrastRatio(tok(t, 'muted'), tok(t, s));
          expect(ratio).toBeGreaterThanOrEqual(4.5);
        });

        it(`small --text on --${s} meets WCAG AA (>= 4.5:1)`, () => {
          const ratio = contrastRatio(tok(t, 'text'), tok(t, s));
          expect(ratio).toBeGreaterThanOrEqual(4.5);
        });
      }
    });
  }
});



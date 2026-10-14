export type TokenMap = Record<string, string>;

/** Returns the text between the braces of the first block that starts with `opener`. */
export function extractBlock(css: string, opener: string): string {
  const start = css.indexOf(opener);
  if (start < 0) throw new Error(`block not found: ${opener}`);
  const open = css.indexOf('{', start);
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === '{') depth++;
    else if (css[i] === '}') {
      depth--;
      if (depth === 0) return css.slice(open + 1, i);
    }
  }
  throw new Error(`unbalanced braces after: ${opener}`);
}

export function parseVars(body: string): TokenMap {
  const out: TokenMap = {};
  for (const m of body.matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/gi)) out[m[1]] = m[2].trim();
  return out;
}

export function readThemes(css: string): {
  light: TokenMap;
  dark: TokenMap;
  darkMedia: TokenMap;
  forestLight: TokenMap;
  forestDark: TokenMap;
  forestDarkMedia: TokenMap;
  tokyoNightLight: TokenMap;
  tokyoNightDark: TokenMap;
  tokyoNightDarkMedia: TokenMap;
  catppuccinLight: TokenMap;
  catppuccinDark: TokenMap;
  catppuccinDarkMedia: TokenMap;
  kanagawaLight: TokenMap;
  kanagawaDark: TokenMap;
  kanagawaDarkMedia: TokenMap;
} {
  const light = parseVars(extractBlock(css, ':root {'));
  const dark = { ...light, ...parseVars(extractBlock(css, ':root[data-theme="dark"] {')) };
  const media = extractBlock(css, '@media (prefers-color-scheme: dark) {');
  const darkMedia = { ...light, ...parseVars(extractBlock(media, ':root:not([data-theme="light"]) {')) };

  const forestLight = { ...light, ...parseVars(extractBlock(css, ':root[data-palette="forest"] {')) };
  const forestDark = { ...dark, ...parseVars(extractBlock(css, ':root[data-theme="dark"][data-palette="forest"] {')) };
  const forestDarkMedia = { ...darkMedia, ...parseVars(extractBlock(media, ':root:not([data-theme="light"])[data-palette="forest"] {')) };

  const tokyoNightLight = { ...light, ...parseVars(extractBlock(css, ':root[data-palette="tokyo-night"] {')) };
  const tokyoNightDark = { ...dark, ...parseVars(extractBlock(css, ':root[data-theme="dark"][data-palette="tokyo-night"] {')) };
  const tokyoNightDarkMedia = { ...darkMedia, ...parseVars(extractBlock(media, ':root:not([data-theme="light"])[data-palette="tokyo-night"] {')) };

  const catppuccinLight = { ...light, ...parseVars(extractBlock(css, ':root[data-palette="catppuccin"] {')) };
  const catppuccinDark = { ...dark, ...parseVars(extractBlock(css, ':root[data-theme="dark"][data-palette="catppuccin"] {')) };
  const catppuccinDarkMedia = { ...darkMedia, ...parseVars(extractBlock(media, ':root:not([data-theme="light"])[data-palette="catppuccin"] {')) };

  const kanagawaLight = { ...light, ...parseVars(extractBlock(css, ':root[data-palette="kanagawa"] {')) };
  const kanagawaDark = { ...dark, ...parseVars(extractBlock(css, ':root[data-theme="dark"][data-palette="kanagawa"] {')) };
  const kanagawaDarkMedia = { ...darkMedia, ...parseVars(extractBlock(media, ':root:not([data-theme="light"])[data-palette="kanagawa"] {')) };

  return {
    light,
    dark,
    darkMedia,
    forestLight,
    forestDark,
    forestDarkMedia,
    tokyoNightLight,
    tokyoNightDark,
    tokyoNightDarkMedia,
    catppuccinLight,
    catppuccinDark,
    catppuccinDarkMedia,
    kanagawaLight,
    kanagawaDark,
    kanagawaDarkMedia,
  };
}


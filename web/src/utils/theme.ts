export type PaletteId =
  | 'default'
  | 'forest'
  | 'tokyo-night'
  | 'catppuccin'
  | 'kanagawa'
  | 'everforest'
  | 'gruvbox';

export interface PaletteDefinition {
  id: PaletteId;
  name: string;
  label: string;
  swatch: {
    bg: string;
    accent: string;
    text: string;
  };
}

export const PALETTES: PaletteDefinition[] = [
  {
    id: 'default',
    name: 'Default (Indigo)',
    label: 'Indigo',
    swatch: { bg: '#111827', accent: '#4f46e5', text: '#f9fafb' },
  },
  {
    id: 'forest',
    name: 'Forest & Moss',
    label: 'Forest',
    swatch: { bg: '#122017', accent: '#2d7a50', text: '#f0fdf4' },
  },
  {
    id: 'tokyo-night',
    name: 'Tokyo Night',
    label: 'Tokyo Night',
    swatch: { bg: '#1f2335', accent: '#7aa2f7', text: '#c0caf5' },
  },
  {
    id: 'catppuccin',
    name: 'Catppuccin',
    label: 'Catppuccin',
    swatch: { bg: '#1e1e2e', accent: '#cba6f7', text: '#cdd6f4' },
  },
  {
    id: 'kanagawa',
    name: 'Kanagawa',
    label: 'Kanagawa',
    swatch: { bg: '#1f1f28', accent: '#7e9cd8', text: '#dcd7ba' },
  },
  {
    id: 'everforest',
    name: 'Everforest',
    label: 'Everforest',
    swatch: { bg: '#272e33', accent: '#a7c080', text: '#d3c6aa' },
  },
  {
    id: 'gruvbox',
    name: 'Gruvbox',
    label: 'Gruvbox',
    swatch: { bg: '#282828', accent: '#83a598', text: '#ebdbb2' },
  },
];

export function parsePaletteId(val: string | null | undefined): PaletteId {
  if (
    val === 'default' ||
    val === 'forest' ||
    val === 'tokyo-night' ||
    val === 'catppuccin' ||
    val === 'kanagawa' ||
    val === 'everforest' ||
    val === 'gruvbox'
  ) {
    return val;
  }
  return 'default';
}

export type PaletteId =
  | 'default'
  | 'forest'
  | 'tokyo-night'
  | 'catppuccin'
  | 'kanagawa'
  | 'everforest'
  | 'gruvbox';

export const PALETTES: { id: PaletteId; name: string }[] = [
  { id: 'default', name: 'Default (Indigo)' },
  { id: 'forest', name: 'Forest & Moss' },
  { id: 'tokyo-night', name: 'Tokyo Night' },
  { id: 'catppuccin', name: 'Catppuccin' },
  { id: 'kanagawa', name: 'Kanagawa' },
  { id: 'everforest', name: 'Everforest' },
  { id: 'gruvbox', name: 'Gruvbox' },
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

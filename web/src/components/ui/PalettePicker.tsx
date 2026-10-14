import React, { useRef } from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { PALETTES, type PaletteId } from '../../utils/theme';

export interface PalettePickerProps {
  value: PaletteId;
  onValueChange: (val: PaletteId) => void;
  className?: string;
}

export const PalettePicker: React.FC<PalettePickerProps> = ({
  value,
  onValueChange,
  className,
}) => {
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleKeyDown = (e: React.KeyboardEvent, index: number) => {
    let nextIndex = index;
    const count = PALETTES.length;

    switch (e.key) {
      case 'ArrowRight':
      case 'ArrowDown':
        e.preventDefault();
        nextIndex = (index + 1) % count;
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
        e.preventDefault();
        nextIndex = (index - 1 + count) % count;
        break;
      case 'Home':
        e.preventDefault();
        nextIndex = 0;
        break;
      case 'End':
        e.preventDefault();
        nextIndex = count - 1;
        break;
      default:
        return;
    }

    const nextPalette = PALETTES[nextIndex];
    if (nextPalette) {
      onValueChange(nextPalette.id);
      itemRefs.current[nextIndex]?.focus();
    }
  };

  return (
    <div
      role="radiogroup"
      aria-label="Color Palette"
      className={twMerge('grid grid-cols-4 sm:grid-cols-7 gap-2 w-full', className)}
    >
      {PALETTES.map((palette, idx) => {
        const isSelected = palette.id === value;
        return (
          <button
            key={palette.id}
            ref={(el) => {
              itemRefs.current[idx] = el;
            }}
            type="button"
            role="radio"
            aria-checked={isSelected}
            tabIndex={isSelected ? 0 : -1}
            onClick={() => onValueChange(palette.id)}
            onKeyDown={(e) => handleKeyDown(e, idx)}
            className={clsx(
              'group relative flex flex-col items-center justify-center py-2.5 px-1.5 rounded-[var(--radius-card)] cursor-pointer text-center select-none transition-[background-color,border-color,box-shadow,color] duration-[var(--dur-base)] ease-[var(--ease-out)]',
              'focus-visible:outline-2 focus-visible:outline-[var(--accent-text)] focus-visible:outline-offset-2',
              isSelected
                ? 'border-2 border-[var(--accent)] bg-[var(--surface-hover)]/70 text-[var(--text)] shadow-xs'
                : 'border border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)] text-[var(--muted)] hover:text-[var(--text)]'
            )}
          >
            {/* 3 Swatch Dots: bg / accent / text */}
            <div className="flex items-center gap-1 mb-1.5" aria-hidden="true">
              <span
                className="w-2.5 h-2.5 rounded-full border border-black/15 dark:border-white/20 shrink-0"
                style={{ backgroundColor: palette.swatch.bg }}
              />
              <span
                className="w-2.5 h-2.5 rounded-full border border-black/15 dark:border-white/20 shrink-0"
                style={{ backgroundColor: palette.swatch.accent }}
              />
              <span
                className="w-2.5 h-2.5 rounded-full border border-black/15 dark:border-white/20 shrink-0"
                style={{ backgroundColor: palette.swatch.text }}
              />
            </div>

            {/* Palette label */}
            <span
              className={clsx(
                'text-xs leading-tight font-medium truncate max-w-full tracking-tight',
                isSelected ? 'text-[var(--text)] font-semibold' : 'text-[var(--muted)] group-hover:text-[var(--text)]'
              )}
            >
              {palette.label}
            </span>
          </button>
        );
      })}
    </div>
  );
};

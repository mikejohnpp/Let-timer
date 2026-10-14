import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { PalettePicker } from '../components/ui/PalettePicker';
import { PALETTES } from '../utils/theme';

describe('PalettePicker', () => {
  it('renders all 7 palette options with role="radio"', () => {
    const handleChange = vi.fn();
    render(<PalettePicker value="default" onValueChange={handleChange} />);

    const radiogroup = screen.getByRole('radiogroup', { name: /color palette/i });
    expect(radiogroup).toBeInTheDocument();

    const radios = screen.getAllByRole('radio');
    expect(radios).toHaveLength(7);

    PALETTES.forEach((palette, idx) => {
      expect(radios[idx]).toHaveTextContent(palette.label);
    });
  });

  it('marks current value with aria-checked="true" and tabIndex=0, others false and -1', () => {
    const handleChange = vi.fn();
    render(<PalettePicker value="tokyo-night" onValueChange={handleChange} />);

    const radios = screen.getAllByRole('radio');
    const tokyoRadio = radios.find((r) => r.textContent?.includes('Tokyo Night'));
    expect(tokyoRadio).toHaveAttribute('aria-checked', 'true');
    expect(tokyoRadio).toHaveAttribute('tabindex', '0');

    const defaultRadio = radios.find((r) => r.textContent?.includes('Indigo'));
    expect(defaultRadio).toHaveAttribute('aria-checked', 'false');
    expect(defaultRadio).toHaveAttribute('tabindex', '-1');
  });

  it('calls onValueChange when an item is clicked', () => {
    const handleChange = vi.fn();
    render(<PalettePicker value="default" onValueChange={handleChange} />);

    const forestRadio = screen.getByRole('radio', { name: /^forest$/i });
    fireEvent.click(forestRadio);

    expect(handleChange).toHaveBeenCalledWith('forest');
  });

  it('navigates with ArrowRight and ArrowLeft keys (circular navigation)', () => {
    const handleChange = vi.fn();
    render(<PalettePicker value="default" onValueChange={handleChange} />);

    const radios = screen.getAllByRole('radio');
    const firstRadio = radios[0];

    // ArrowRight moves from index 0 ('default') to index 1 ('forest')
    fireEvent.keyDown(firstRadio, { key: 'ArrowRight' });
    expect(handleChange).toHaveBeenCalledWith('forest');

    // ArrowLeft from index 0 wraps to last item ('gruvbox')
    fireEvent.keyDown(firstRadio, { key: 'ArrowLeft' });
    expect(handleChange).toHaveBeenCalledWith('gruvbox');
  });

  it('navigates with ArrowDown, ArrowUp, Home, and End keys', () => {
    const handleChange = vi.fn();
    render(<PalettePicker value="catppuccin" onValueChange={handleChange} />);

    const radios = screen.getAllByRole('radio');
    const catppuccinRadio = radios[3]; // index 3

    fireEvent.keyDown(catppuccinRadio, { key: 'ArrowDown' });
    expect(handleChange).toHaveBeenCalledWith('kanagawa'); // index 4

    fireEvent.keyDown(catppuccinRadio, { key: 'ArrowUp' });
    expect(handleChange).toHaveBeenCalledWith('tokyo-night'); // index 2

    fireEvent.keyDown(catppuccinRadio, { key: 'Home' });
    expect(handleChange).toHaveBeenCalledWith('default'); // index 0

    fireEvent.keyDown(catppuccinRadio, { key: 'End' });
    expect(handleChange).toHaveBeenCalledWith('gruvbox'); // index 6
  });
});

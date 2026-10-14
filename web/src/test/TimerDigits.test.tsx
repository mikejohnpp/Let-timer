import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TimerDigits } from '../components/ui/TimerDigits';

describe('TimerDigits component (T4)', () => {
  it('renders digits with font-mono and tabular-nums', () => {
    render(<TimerDigits digits="25:00" />);
    const el = screen.getByText('25:00');
    expect(el).toBeInTheDocument();
    expect(el.className).toContain('font-mono');
    expect(el.className).toContain('tabular-nums');
  });

  it('renders countdown mode with accent color token', () => {
    render(<TimerDigits digits="12:34" mode="countdown" />);
    const el = screen.getByText('12:34');
    expect(el.className).toContain('text-[var(--accent-text)]');
  });

  it('renders overtime mode with over color token', () => {
    render(<TimerDigits digits="+03:15" mode="overtime" />);
    const el = screen.getByText('+03:15');
    expect(el.className).toContain('text-[var(--over-text)]');
  });

  it('renders idle and paused modes with muted color token', () => {
    const { rerender } = render(<TimerDigits digits="00:00" mode="idle" />);
    expect(screen.getByText('00:00').className).toContain('text-[var(--muted)]');

    rerender(<TimerDigits digits="10:00" mode="paused" />);
    expect(screen.getByText('10:00').className).toContain('text-[var(--muted)]');
  });

  it('renders display size with tracking-tighter for large timer digits', () => {
    render(<TimerDigits digits="25:00" size="display" />);
    const el = screen.getByText('25:00');
    expect(el.className).toContain('tracking-tighter');
    expect(el.className).toContain('text-6xl');
  });

  it('renders pill size with font-bold for timer bar', () => {
    render(<TimerDigits digits="24:59" size="pill" />);
    const el = screen.getByText('24:59');
    expect(el.className).toContain('font-bold');
    expect(el.className).not.toContain('tracking-tighter');
  });

  it('passes aria-hidden attribute when provided', () => {
    render(<TimerDigits digits="25:00" aria-hidden="true" />);
    const el = screen.getByText('25:00');
    expect(el).toHaveAttribute('aria-hidden', 'true');
  });
});


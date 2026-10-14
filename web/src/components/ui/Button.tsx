import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-solid' | 'pill' | 'pill-primary';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'secondary', size = 'md', loading, disabled, children, ...props }, ref) => {
    const base = 'inline-flex items-center justify-center font-medium transition-[background-color,color,border-color,transform] duration-[var(--dur-fast)] ease-[var(--ease-out)] active:scale-[0.98] active:duration-[var(--dur-instant)] select-none disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--accent-text)] focus-visible:outline-offset-2';

    const variants = {
      primary: 'bg-[var(--accent)] text-[var(--on-accent)] hover:bg-[var(--accent-hover)] rounded-[var(--radius-control)] shadow-sm',
      secondary: 'bg-[var(--surface)] border border-[var(--border-strong)] text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-[var(--radius-control)] shadow-xs',
      ghost: 'bg-transparent text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-[var(--radius-control)]',
      danger: 'bg-transparent text-[var(--danger-text)] hover:bg-[var(--danger-bg)] rounded-[var(--radius-control)]',
      'danger-solid': 'bg-[var(--danger)] text-[var(--on-danger)] hover:opacity-90 rounded-[var(--radius-control)] shadow-sm',
      pill: 'bg-[var(--surface-hover)]/70 hover:bg-[var(--surface)] text-[var(--text)] border border-[var(--border)]/60 rounded-full backdrop-blur-md shadow-xs',
      'pill-primary': 'bg-[var(--accent)]/85 hover:bg-[var(--accent)] text-[var(--on-accent)] border border-white/20 rounded-full backdrop-blur-md shadow-sm',
    };

    const sizes = {
      sm: 'h-7 px-2.5 text-xs gap-1.5',
      md: 'h-9 px-3.5 text-xs gap-2',
      lg: 'h-11 px-5 text-sm gap-2.5',
    };

    return (
      <button
        ref={ref}
        disabled={disabled || loading}
        className={twMerge(clsx(base, variants[variant], sizes[size], className))}
        {...props}
      >
        {loading ? <span className="opacity-70">Saving...</span> : children}
      </button>
    );
  }
);
Button.displayName = 'Button';

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'round';
  size?: 'sm' | 'md' | 'lg';
  'aria-label': string;
}

export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ className, variant = 'ghost', size = 'md', disabled, children, ...props }, ref) => {
    const base = 'inline-flex items-center justify-center transition-[background-color,color,border-color,transform] duration-[var(--dur-fast)] ease-[var(--ease-out)] active:scale-[0.98] active:duration-[var(--dur-instant)] select-none disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--accent-text)] focus-visible:outline-offset-2';

    const variants = {
      primary: 'bg-[var(--accent)] text-[var(--on-accent)] hover:bg-[var(--accent-hover)] rounded-[var(--radius-control)]',
      secondary: 'bg-[var(--surface)] border border-[var(--border-strong)] text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-[var(--radius-control)]',
      ghost: 'bg-transparent text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] rounded-[var(--radius-control)]',
      danger: 'bg-transparent text-[var(--muted)] hover:text-[var(--danger-text)] hover:bg-[var(--danger-bg)] rounded-[var(--radius-control)]',
      round: 'rounded-full border border-[var(--border)] text-[var(--text)] hover:bg-[var(--surface-hover)]',
    };

    const sizes = {
      sm: 'w-7 h-7 text-xs',
      md: 'w-9 h-9 text-sm',
      lg: 'w-11 h-11 text-base',
    };

    return (
      <button
        ref={ref}
        disabled={disabled}
        className={twMerge(clsx(base, variants[variant], sizes[size], className))}
        {...props}
      >
        {children}
      </button>
    );
  }
);
IconButton.displayName = 'IconButton';


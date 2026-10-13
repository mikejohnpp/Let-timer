import React, { useId } from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface FormFieldProps {
  label?: string;
  helperText?: string;
  error?: string;
  required?: boolean;
  children: (id: string, ariaDescribedBy?: string) => React.ReactNode;
  className?: string;
}

export const FormField: React.FC<FormFieldProps> = ({
  label,
  helperText,
  error,
  required,
  children,
  className,
}) => {
  const id = useId();
  const helperId = helperText ? `${id}-helper` : undefined;
  const errorId = error ? `${id}-error` : undefined;
  const describedBy = errorId || helperId;

  return (
    <div className={twMerge(clsx('flex flex-col gap-1.5', className))}>
      {label && (
        <label htmlFor={id} className="text-xs font-medium text-[var(--muted)] flex items-center gap-1">
          <span>{label}</span>
          {required && <span className="text-[var(--danger)]">*</span>}
        </label>
      )}
      {children(id, describedBy)}
      {error ? (
        <p id={errorId} className="text-[11px] text-[var(--danger-text)] font-medium">
          {error}
        </p>
      ) : helperText ? (
        <p id={helperId} className="text-[11px] text-[var(--muted)]">
          {helperText}
        </p>
      ) : null}
    </div>
  );
};

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={twMerge(
          clsx(
            'h-9 px-3 rounded-[var(--radius-control)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] outline-none transition focus:border-[var(--accent-text)] focus:ring-2 focus:ring-[var(--accent-text)]/20 disabled:opacity-50 disabled:bg-[var(--surface-hover)]',
            className
          )
        )}
        {...props}
      />
    );
  }
);
Input.displayName = 'Input';

export const Textarea = React.forwardRef<HTMLTextAreaElement, React.TextareaHTMLAttributes<HTMLTextAreaElement>>(
  ({ className, ...props }, ref) => {
    return (
      <textarea
        ref={ref}
        className={twMerge(
          clsx(
            'p-3 rounded-[var(--radius-control)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] outline-none transition focus:border-[var(--accent-text)] focus:ring-2 focus:ring-[var(--accent-text)]/20 disabled:opacity-50 disabled:bg-[var(--surface-hover)] resize-none',
            className
          )
        )}
        {...props}
      />
    );
  }
);
Textarea.displayName = 'Textarea';

export const Select = React.forwardRef<HTMLSelectElement, React.SelectHTMLAttributes<HTMLSelectElement>>(
  ({ className, children, ...props }, ref) => {
    return (
      <select
        ref={ref}
        className={twMerge(
          clsx(
            'h-9 px-3 rounded-[var(--radius-control)] bg-[var(--surface)] border border-[var(--border-strong)] text-xs text-[var(--text)] outline-none transition focus:border-[var(--accent-text)] focus:ring-2 focus:ring-[var(--accent-text)]/20 disabled:opacity-50 disabled:bg-[var(--surface-hover)]',
            className
          )
        )}
        {...props}
      >
        {children}
      </select>
    );
  }
);
Select.displayName = 'Select';


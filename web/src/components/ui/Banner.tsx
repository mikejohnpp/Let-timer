import React from 'react';
import { AlertCircle, AlertTriangle, CheckCircle, Info } from 'lucide-react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface BannerProps {
  tone?: 'danger' | 'warning' | 'success' | 'over' | 'info';
  children: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
}

export const Banner: React.FC<BannerProps> = ({
  tone = 'info',
  children,
  action,
  className,
}) => {
  const tones = {
    danger: {
      bg: 'bg-[var(--danger-bg)] border-[var(--danger)]/30',
      icon: <AlertCircle className="w-4 h-4 text-[var(--danger)] shrink-0" />,
      role: 'alert' as const,
    },
    warning: {
      bg: 'bg-[var(--warning-bg)] border-[var(--warning)]/30',
      icon: <AlertTriangle className="w-4 h-4 text-[var(--warning)] shrink-0" />,
      role: 'status' as const,
    },
    success: {
      bg: 'bg-[var(--success-bg)] border-[var(--success)]/30',
      icon: <CheckCircle className="w-4 h-4 text-[var(--success)] shrink-0" />,
      role: 'status' as const,
    },
    over: {
      bg: 'bg-[var(--over-bg)] border-[var(--timer-over)]/30',
      icon: <AlertTriangle className="w-4 h-4 text-[var(--timer-over)] shrink-0" />,
      role: 'status' as const,
    },
    info: {
      bg: 'bg-[var(--info-bg)] border-[var(--accent-text)]/30',
      icon: <Info className="w-4 h-4 text-[var(--accent-text)] shrink-0" />,
      role: 'status' as const,
    },
  };

  const current = tones[tone];

  return (
    <div
      role={current.role}
      className={twMerge(
        clsx(
          'flex items-center justify-between gap-3 p-3 rounded-[var(--radius-control)] border text-xs text-[var(--text)] animate-[fade-in_var(--dur-fast)_var(--ease-out)]',
          current.bg,
          className
        )
      )}
    >
      <div className="flex items-center gap-2">
        {current.icon}
        <div>{children}</div>
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
};


import { RefreshCw, WifiOff } from 'lucide-react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface ConnectionBadgeProps {
  status: 'connected' | 'reconnecting' | 'disconnected';
  borderless?: boolean;
  className?: string;
}

export const ConnectionBadge: React.FC<ConnectionBadgeProps> = ({
  status,
  borderless = true,
  className,
}) => {
  const configs = {
    connected: {
      icon: <span className="w-2 h-2 rounded-full bg-[var(--success)] animate-pulse" />,
      label: 'Live',
      textClass: 'text-[var(--muted)]',
    },
    reconnecting: {
      icon: <RefreshCw className="w-3 h-3 text-[var(--warning)] animate-spin" />,
      label: 'Syncing',
      textClass: 'text-[var(--warning-text)]',
    },
    disconnected: {
      icon: <WifiOff className="w-3 h-3 text-[var(--danger)]" />,
      label: 'Offline',
      textClass: 'text-[var(--danger-text)]',
    },
  };

  const current = configs[status];

  return (
    <div
      className={twMerge(
        clsx(
          borderless
            ? 'inline-flex items-center gap-1.5 px-1.5 py-0.5 text-xs font-medium select-none'
            : 'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-control)] border border-[var(--border)] bg-[var(--surface)] text-xs font-medium',
          className
        )
      )}
    >
      {current.icon}
      <span className={clsx('text-[11px]', current.textClass)}>{current.label}</span>
    </div>
  );
};


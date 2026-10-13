import React from 'react';
import { Wifi, RefreshCw, WifiOff } from 'lucide-react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface ConnectionBadgeProps {
  status: 'connected' | 'reconnecting' | 'disconnected';
  className?: string;
}

export const ConnectionBadge: React.FC<ConnectionBadgeProps> = ({ status, className }) => {
  const configs = {
    connected: {
      icon: <Wifi className="w-3.5 h-3.5 text-[var(--success)]" />,
      label: 'Live',
      textClass: 'text-[var(--text)]',
    },
    reconnecting: {
      icon: <RefreshCw className="w-3.5 h-3.5 text-[var(--warning)] animate-spin" />,
      label: 'Syncing',
      textClass: 'text-[var(--warning-text)]',
    },
    disconnected: {
      icon: <WifiOff className="w-3.5 h-3.5 text-[var(--danger)]" />,
      label: 'Offline',
      textClass: 'text-[var(--danger-text)]',
    },
  };

  const current = configs[status];

  return (
    <div
      className={twMerge(
        clsx(
          'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-control)] border border-[var(--border)] bg-[var(--surface)] text-xs font-medium',
          className
        )
      )}
    >
      {current.icon}
      <span className={clsx('hidden xl:inline text-[11px]', current.textClass)}>{current.label}</span>
    </div>
  );
};


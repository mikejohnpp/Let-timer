import React from 'react';
import { Clock, AlertCircle } from 'lucide-react';
import { Button } from './Button';

export interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  icon = <Clock className="w-8 h-8 text-[var(--muted)]" />,
  title,
  description,
  action,
}) => {
  return (
    <div className="flex flex-col items-center justify-center p-8 text-center rounded-[var(--radius-card)] border border-dashed border-[var(--border-strong)] bg-[var(--surface)]/50 gap-2">
      <div className="p-2 rounded-full bg-[var(--surface-hover)] mb-1">{icon}</div>
      <h3 className="text-sm font-semibold text-[var(--text)]">{title}</h3>
      {description && <p className="text-xs text-[var(--muted)] max-w-sm">{description}</p>}
      {action && (
        <Button variant="primary" size="sm" onClick={action.onClick} className="mt-3">
          {action.label}
        </Button>
      )}
    </div>
  );
};

export interface ErrorStateProps {
  title?: string;
  description?: string;
  onRetry?: () => void;
}

export const ErrorState: React.FC<ErrorStateProps> = ({
  title = "Can't reach the server",
  description,
  onRetry,
}) => {
  return (
    <div className="flex flex-col items-center justify-center p-8 text-center rounded-[var(--radius-card)] border border-[var(--danger)]/30 bg-[var(--danger-bg)] gap-2">
      <AlertCircle className="w-8 h-8 text-[var(--danger)] mb-1" />
      <h3 className="text-sm font-semibold text-[var(--text)]">{title}</h3>
      {description && <p className="text-xs text-[var(--danger-text)] max-w-sm">{description}</p>}
      {onRetry && (
        <Button variant="secondary" size="sm" onClick={onRetry} className="mt-3">
          Retry
        </Button>
      )}
    </div>
  );
};


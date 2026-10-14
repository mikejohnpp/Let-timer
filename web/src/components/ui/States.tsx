import React from 'react';
import { Clock, AlertCircle, Plus } from 'lucide-react';
import { Button } from './Button';

export interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
    icon?: React.ReactNode;
  };
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  icon = <Clock className="w-8 h-8 text-[var(--muted)]" />,
  title,
  description,
  action,
}) => {
  return (
    <div className="flex flex-col items-center justify-center p-8 text-center rounded-[var(--radius-card)] border border-dashed border-[var(--border)]/60 bg-[var(--surface)]/40 backdrop-blur-sm gap-2">
      <div className="p-2 rounded-full bg-[var(--surface-hover)]/70 mb-1">{icon}</div>
      <h3 className="text-sm font-semibold text-[var(--text)]">{title}</h3>
      {description && <p className="text-xs text-[var(--muted)] max-w-sm">{description}</p>}
      {action && (
        <button
          type="button"
          onClick={action.onClick}
          className="mt-3 inline-flex items-center gap-1.5 px-4 py-2 rounded-full text-xs sm:text-sm font-medium transition cursor-pointer backdrop-blur-md shadow-xs active:scale-[0.98] bg-[var(--surface-hover)]/70 hover:bg-[var(--surface)] text-[var(--text)] border border-[var(--border)]/60 hover:border-[var(--border-strong)]"
        >
          {action.icon ?? <Plus className="w-3.5 h-3.5 text-[var(--accent-text)]" />}
          <span>{action.label}</span>
        </button>
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


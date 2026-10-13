import React from 'react';
import { ChevronUp, ChevronsUp } from 'lucide-react';

export interface PriorityMarkerProps {
  priority: number; // 0: not-yet, 1: immediate, 2: urgent
  showLabel?: boolean;
  className?: string;
}

export const PriorityMarker: React.FC<PriorityMarkerProps> = ({ priority, showLabel = false, className }) => {
  if (priority === 2) {
    return (
      <span className={`inline-flex items-center gap-0.5 text-[var(--danger)] ${className || ''}`} aria-label="Urgent priority">
        <ChevronsUp className="w-3.5 h-3.5" />
        {showLabel && <span className="text-[11px] font-medium text-[var(--danger-text)]">Urgent</span>}
      </span>
    );
  }

  if (priority === 1) {
    return (
      <span className={`inline-flex items-center gap-0.5 text-[var(--warning)] ${className || ''}`} aria-label="Immediate priority">
        <ChevronUp className="w-3.5 h-3.5" />
        {showLabel && <span className="text-[11px] font-medium text-[var(--warning-text)]">Immediate</span>}
      </span>
    );
  }

  return null;
};


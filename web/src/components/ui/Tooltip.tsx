import React from 'react';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export const TooltipProvider = TooltipPrimitive.Provider;

export interface TooltipProps {
  content: React.ReactNode;
  children: React.ReactNode;
  side?: 'top' | 'right' | 'bottom' | 'left';
  className?: string;
}

export const Tooltip: React.FC<TooltipProps> = ({ content, children, side = 'top', className }) => {
  return (
    <TooltipPrimitive.Root delayDuration={200}>
      <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          side={side}
          className={twMerge(
            clsx(
              'z-50 overflow-hidden rounded-[var(--radius-control)] border border-[var(--border)]/40 bg-[var(--surface)]/95 backdrop-blur-md px-2 py-1 text-xs text-[var(--text)] shadow-sm select-none',
              'data-[state=delayed-open]:animate-[pop-in_var(--dur-fast)_var(--ease-out)] data-[state=closed]:animate-[fade-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]',
              className
            )
          )}
        >
          {content}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
};


import React from 'react';
import * as DialogPrimitive from '@radix-ui/react-dialog';
import { X } from 'lucide-react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface ModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title?: string;
  description?: string;
  children: React.ReactNode;
  className?: string;
}

export const Modal: React.FC<ModalProps> = ({
  open,
  onOpenChange,
  title,
  description,
  children,
  className,
}) => {
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 bg-black/45 backdrop-blur-sm sm:backdrop-blur-md z-50 data-[state=open]:animate-[fade-in_var(--dur-base)_var(--ease-out)] data-[state=closed]:animate-[fade-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]" />
        <DialogPrimitive.Content
          className={twMerge(
            clsx(
              'fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-lg max-h-[85vh] bg-[var(--surface)]/90 backdrop-blur-xl sm:backdrop-blur-2xl border border-[var(--border)]/40 rounded-[var(--radius-modal)] p-5 shadow-2xl z-50 overflow-y-auto flex flex-col gap-4 outline-none data-[state=open]:animate-[rise-in_var(--dur-base)_var(--ease-out)] data-[state=closed]:animate-[rise-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]',
              className
            )
          )}
        >
          <div className="flex items-center justify-between">
            {title && (
              <DialogPrimitive.Title className="font-serif text-lg font-semibold leading-tight text-[var(--text)] tracking-tight">
                {title}
              </DialogPrimitive.Title>
            )}
            <DialogPrimitive.Close asChild>
              <button
                className="p-1 rounded-[var(--radius-control)] text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition cursor-pointer"
                aria-label="Close"
              >
                <X className="w-4 h-4" />
              </button>
            </DialogPrimitive.Close>
          </div>
          {description && (
            <DialogPrimitive.Description className="text-xs text-[var(--muted)] leading-relaxed -mt-2">
              {description}
            </DialogPrimitive.Description>
          )}
          {children}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
};


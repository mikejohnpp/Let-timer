import React from 'react';
import * as DialogPrimitive from '@radix-ui/react-dialog';
import { X } from 'lucide-react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface DrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title?: string;
  description?: string;
  children: React.ReactNode;
  className?: string;
}

export const Drawer: React.FC<DrawerProps> = ({
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
        <DialogPrimitive.Overlay className="fixed inset-0 bg-black/55 backdrop-blur-[2px] z-50 animate-in fade-in duration-150" />
        <DialogPrimitive.Content
          className={twMerge(
            clsx(
              'fixed z-50 bg-[var(--surface)] border-[var(--border)] shadow-xl flex flex-col outline-none transition ease-out duration-200',
              // Base (mobile bottom sheet):
              'inset-x-0 bottom-0 max-h-[92vh] border-t rounded-t-[var(--radius-modal)]',
              // Desktop (right-side drawer, 100% full viewport height):
              'sm:inset-y-0 sm:top-0 sm:bottom-0 sm:right-0 sm:left-auto sm:h-screen sm:max-h-none sm:w-full sm:max-w-md sm:border-l sm:border-t-0 sm:rounded-none',
              className
            )
          )}
        >
          <div className="flex items-center justify-between p-4 sm:p-5 border-b border-[var(--border)] shrink-0">
            <div>
              {title && (
                <DialogPrimitive.Title className="text-base font-bold text-[var(--text)]">
                  {title}
                </DialogPrimitive.Title>
              )}
              {description && (
                <DialogPrimitive.Description className="text-xs text-[var(--muted)] mt-0.5">
                  {description}
                </DialogPrimitive.Description>
              )}
            </div>
            <DialogPrimitive.Close asChild>
              <button
                className="p-1 rounded-[var(--radius-control)] text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition cursor-pointer"
                aria-label="Close drawer"
              >
                <X className="w-4 h-4" />
              </button>
            </DialogPrimitive.Close>
          </div>
          <div className="flex-1 overflow-y-auto p-4 sm:p-5">{children}</div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
};


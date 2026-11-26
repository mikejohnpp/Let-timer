import React from 'react';
import * as PopoverPrimitive from '@radix-ui/react-popover';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;
export const PopoverAnchor = PopoverPrimitive.Anchor;

export const PopoverContent = React.forwardRef<
  React.ComponentRef<typeof PopoverPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof PopoverPrimitive.Content>
>(({ className, align = 'center', sideOffset = 4, ...props }, ref) => (
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      ref={ref}
      align={align}
      sideOffset={sideOffset}
      className={twMerge(
        clsx(
          'z-50 w-72 rounded-[var(--radius-card)] border border-[var(--border)]/40 bg-[var(--surface)]/90 backdrop-blur-xl p-4 text-[var(--text)] shadow-xl outline-none data-[state=open]:animate-[pop-in_var(--dur-fast)_var(--ease-out)] data-[state=closed]:animate-[fade-out_var(--dur-instant)_var(--ease-out)_forwards] [transform-origin:var(--radix-popover-content-transform-origin)]',
          className
        )
      )}
      {...props}
    />
  </PopoverPrimitive.Portal>
));
PopoverContent.displayName = PopoverPrimitive.Content.displayName;


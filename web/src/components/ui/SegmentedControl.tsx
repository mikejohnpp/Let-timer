import React, { useId } from 'react';
import * as ToggleGroupPrimitive from '@radix-ui/react-toggle-group';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { motion } from 'motion/react';

export interface SegmentItem {
  value: string;
  label: React.ReactNode;
}

export interface SegmentedControlProps {
  value: string;
  onValueChange: (val: string) => void;
  items: SegmentItem[];
  className?: string;
}

export const SegmentedControl: React.FC<SegmentedControlProps> = ({
  value,
  onValueChange,
  items,
  className,
}) => {
  const instanceId = useId();

  return (
    <ToggleGroupPrimitive.Root
      type="single"
      value={value}
      onValueChange={(val) => {
        if (val) onValueChange(val);
      }}
      className={twMerge(
        clsx(
          'inline-flex p-0.5 rounded-[var(--radius-control)] bg-[var(--surface-hover)]/40 backdrop-blur-md border border-[var(--border)]/40 relative',
          className
        )
      )}
    >
      {items.map((item) => {
        const isSelected = item.value === value;
        return (
          <ToggleGroupPrimitive.Item
            key={item.value}
            value={item.value}
            className="relative px-3 py-1 text-xs font-medium rounded-[calc(var(--radius-control)-2px)] transition-[color,transform] duration-[var(--dur-base)] ease-[var(--ease-out)] select-none cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--accent-text)] data-[state=on]:text-[var(--on-accent)] data-[state=off]:text-[var(--muted)] data-[state=off]:hover:text-[var(--text)]"
          >
            {isSelected && (
              <motion.span
                layoutId={`segmented-thumb-${instanceId}`}
                className="absolute inset-0 bg-[var(--accent)] rounded-[calc(var(--radius-control)-2px)] shadow-xs"
                transition={{ duration: 0.18, ease: [0.2, 0, 0, 1] }}
              />
            )}
            <span className="relative z-10">{item.label}</span>
          </ToggleGroupPrimitive.Item>
        );
      })}
    </ToggleGroupPrimitive.Root>
  );
};


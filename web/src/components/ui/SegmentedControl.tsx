import React from 'react';
import * as ToggleGroupPrimitive from '@radix-ui/react-toggle-group';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

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
  return (
    <ToggleGroupPrimitive.Root
      type="single"
      value={value}
      onValueChange={(val) => {
        if (val) onValueChange(val);
      }}
      className={twMerge(
        clsx(
          'inline-flex p-0.5 rounded-[var(--radius-control)] bg-[var(--surface-hover)] border border-[var(--border-strong)]',
          className
        )
      )}
    >
      {items.map((item) => (
        <ToggleGroupPrimitive.Item
          key={item.value}
          value={item.value}
          className="px-3 py-1 text-xs font-medium rounded-[calc(var(--radius-control)-2px)] transition select-none cursor-pointer data-[state=on]:bg-[var(--accent)] data-[state=on]:text-[var(--on-accent)] data-[state=off]:text-[var(--muted)] data-[state=off]:hover:text-[var(--text)] focus-visible:outline-2 focus-visible:outline-[var(--accent-text)]"
        >
          {item.label}
        </ToggleGroupPrimitive.Item>
      ))}
    </ToggleGroupPrimitive.Root>
  );
};


import React, { useState, useEffect } from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  width?: string;
  height?: string;
  delayMs?: number;
}

export const Skeleton: React.FC<SkeletonProps> = ({
  className,
  style,
  width,
  height,
  delayMs = 300,
  ...props
}) => {
  const [visible, setVisible] = useState(delayMs <= 0);

  useEffect(() => {
    if (delayMs <= 0) return;
    const timer = setTimeout(() => setVisible(true), delayMs);
    return () => clearTimeout(timer);
  }, [delayMs]);

  if (!visible) return null;

  return (
    <div
      className={twMerge(
        clsx('bg-[var(--surface-hover)] rounded-[var(--radius-control)] skeleton-shimmer', className)
      )}
      style={{ width, height, ...style }}
      {...props}
    />
  );
};


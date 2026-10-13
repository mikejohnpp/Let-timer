import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  width?: string;
  height?: string;
}

export const Skeleton: React.FC<SkeletonProps> = ({ className, style, width, height, ...props }) => {
  return (
    <div
      className={twMerge(
        clsx('bg-[var(--surface-hover)] rounded-[var(--radius-control)] animate-pulse', className)
      )}
      style={{ width, height, ...style }}
      {...props}
    />
  );
};


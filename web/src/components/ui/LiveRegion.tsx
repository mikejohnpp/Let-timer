import React from 'react';
import * as VisuallyHiddenPrimitive from '@radix-ui/react-visually-hidden';

export interface LiveRegionProps {
  message: string;
  role?: 'status' | 'alert';
}

export const LiveRegion: React.FC<LiveRegionProps> = ({ message, role = 'status' }) => {
  return (
    <VisuallyHiddenPrimitive.Root>
      <div role={role} aria-live={role === 'alert' ? 'assertive' : 'polite'} aria-atomic="true">
        {message}
      </div>
    </VisuallyHiddenPrimitive.Root>
  );
};


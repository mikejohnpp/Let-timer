import React, { useEffect, useState, useRef } from 'react';
import { readMotionTokens } from './tokens';

export interface PresenceProps {
  present: boolean;
  children: React.ReactNode;
  className?: string;
  enterAnimation?: string;
  exitAnimation?: string;
}

/**
 * Presence helper for elements that enter and exit without Radix primitives (e.g. Floating timer pill).
 * Caches children so the exit animation runs cleanly before DOM unmount.
 */
export const Presence: React.FC<PresenceProps> = ({
  present,
  children,
  className = '',
  enterAnimation = 'animate-[rise-in_var(--dur-base)_var(--ease-out)]',
  exitAnimation = 'animate-[rise-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]',
}) => {
  const [isMounted, setIsMounted] = useState(present);
  const [animState, setAnimState] = useState<'open' | 'closed'>(present ? 'open' : 'closed');
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastChildrenRef = useRef<React.ReactNode>(children);

  if (present) {
    lastChildrenRef.current = children;
  }

  useEffect(() => {
    if (present) {
      if (timerRef.current) clearTimeout(timerRef.current);
      setIsMounted(true);
      setAnimState('open');
    } else if (isMounted) {
      setAnimState('closed');
      const tokens = readMotionTokens();
      const exitDur = tokens.isOff ? 0 : Math.round(tokens.durBase * tokens.exitRatio);
      timerRef.current = setTimeout(() => {
        setIsMounted(false);
      }, exitDur);
    }
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [present, isMounted]);

  if (!isMounted) return null;

  return (
    <div
      data-state={animState}
      className={`${className} ${animState === 'open' ? enterAnimation : exitAnimation}`}
    >
      {present ? children : lastChildrenRef.current}
    </div>
  );
};

import React, { useEffect, useState } from 'react';
import { MotionConfig } from 'motion/react';
import { readMotionTokens } from './tokens';

export const MotionProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [tokens, setTokens] = useState(() => readMotionTokens());

  useEffect(() => {
    const handleUpdate = () => {
      setTokens(readMotionTokens(true));
    };
    window.addEventListener('let-timer-motion-changed', handleUpdate);
    const mql = window.matchMedia('(prefers-reduced-motion: reduce)');
    mql.addEventListener('change', handleUpdate);
    return () => {
      window.removeEventListener('let-timer-motion-changed', handleUpdate);
      mql.removeEventListener('change', handleUpdate);
    };
  }, []);

  const reducedMotionSetting = tokens.isReduced || tokens.isOff ? 'always' : 'user';

  return (
    <MotionConfig
      reducedMotion={reducedMotionSetting}
      transition={{
        duration: tokens.isOff ? 0 : tokens.durBase / 1000,
        ease: [0.2, 0, 0, 1],
      }}
    >
      {children}
    </MotionConfig>
  );
};

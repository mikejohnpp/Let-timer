import React, { createContext, useContext, useEffect, useState } from 'react';
import { MotionConfig } from 'motion/react';
import { readMotionTokens, parseBezier, type MotionTokens } from './tokens';

const MotionTokensContext = createContext<MotionTokens>(readMotionTokens());

export function useMotionTokens(): MotionTokens {
  return useContext(MotionTokensContext);
}

export const MotionProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [tokens, setTokens] = useState(() => readMotionTokens());

  useEffect(() => {
    const handleUpdate = () => {
      setTokens(readMotionTokens(true));
    };
    window.addEventListener('let-timer-motion-changed', handleUpdate);
    const mql =
      typeof window.matchMedia === 'function'
        ? window.matchMedia('(prefers-reduced-motion: reduce)')
        : null;
    if (mql && typeof mql.addEventListener === 'function') {
      mql.addEventListener('change', handleUpdate);
    }
    return () => {
      window.removeEventListener('let-timer-motion-changed', handleUpdate);
      if (mql && typeof mql.removeEventListener === 'function') {
        mql.removeEventListener('change', handleUpdate);
      }
    };
  }, []);

  const reducedMotion =
    tokens.mode === 'off' || tokens.mode === 'reduced'
      ? 'always'
      : tokens.mode === 'full'
      ? 'never'
      : 'user'; // system: follow the OS

  return (
    <MotionTokensContext.Provider value={tokens}>
      <MotionConfig
        reducedMotion={reducedMotion}
        transition={{
          duration: tokens.mode === 'off' ? 0 : tokens.durBase / 1000,
          ease: parseBezier(tokens.easeOut),
        }}
      >
        {children}
      </MotionConfig>
    </MotionTokensContext.Provider>
  );
};

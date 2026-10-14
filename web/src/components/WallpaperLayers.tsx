import React, { useEffect, useState, useRef } from 'react';
import { readMotionTokens } from '../motion/tokens';

export interface WallpaperLayersProps {
  url: string | null;
  className?: string;
  overlayClassName?: string;
}

/**
 * WallpaperLayers renders two stacked layers for buttery smooth crossfades (A27).
 * Preloads and decodes the new image via img.decode() before starting the crossfade.
 * Respects --dur-scenic (1200ms) in full motion, 100ms in reduced, 0ms in off.
 */
export const WallpaperLayers: React.FC<WallpaperLayersProps> = ({
  url,
  className = 'absolute inset-0 bg-cover bg-center',
  overlayClassName = 'absolute inset-0 bg-[var(--imm-overlay)] pointer-events-none',
}) => {
  const [currentUrl, setCurrentUrl] = useState<string | null>(url);
  const [nextUrl, setNextUrl] = useState<string | null>(null);
  const [isCrossfading, setIsCrossfading] = useState(false);
  const activeUrlRef = useRef<string | null>(url);

  useEffect(() => {
    if (url === activeUrlRef.current) return;

    if (!url) {
      setCurrentUrl(null);
      setNextUrl(null);
      activeUrlRef.current = null;
      return;
    }

    let isCancelled = false;
    const tokens = readMotionTokens();

    const proceed = () => {
      if (isCancelled) return;
      setNextUrl(url);
      setIsCrossfading(true);

      const crossfadeDuration = tokens.isOff ? 0 : tokens.durScenic;
      const timer = setTimeout(() => {
        if (!isCancelled) {
          setCurrentUrl(url);
          setNextUrl(null);
          setIsCrossfading(false);
          activeUrlRef.current = url;
        }
      }, crossfadeDuration);

      return () => clearTimeout(timer);
    };

    const img = new Image();
    img.src = url;

    if (typeof img.decode === 'function') {
      img.decode().then(proceed).catch(proceed);
    } else {
      img.onload = proceed;
      img.onerror = proceed;
    }

    return () => {
      isCancelled = true;
    };
  }, [url]);

  if (!currentUrl && !nextUrl) return null;

  return (
    <div
      className="wallpaper-layers absolute inset-0 pointer-events-none z-0 overflow-hidden"
      style={{ viewTransitionName: 'none' }}
      aria-hidden="true"
    >
      {/* Current base layer */}
      {currentUrl && (
        <div
          className={`${className} transition-opacity duration-[var(--dur-scenic)] ease-[var(--ease-inout)] ${
            isCrossfading && nextUrl ? 'opacity-0' : 'opacity-100'
          }`}
          style={{ backgroundImage: `url(${currentUrl})` }}
        >
          {overlayClassName && <div className={overlayClassName} />}
        </div>
      )}

      {/* Next incoming layer (fade in) */}
      {nextUrl && (
        <div
          className={`${className} transition-opacity duration-[var(--dur-scenic)] ease-[var(--ease-inout)] ${
            isCrossfading ? 'opacity-100' : 'opacity-0'
          }`}
          style={{ backgroundImage: `url(${nextUrl})` }}
        >
          {overlayClassName && <div className={overlayClassName} />}
        </div>
      )}
    </div>
  );
};

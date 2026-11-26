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
  // Double-buffer layers
  const [layerA, setLayerA] = useState<string | null>(url);
  const [layerB, setLayerB] = useState<string | null>(null);
  const [activeSlot, setActiveSlot] = useState<'A' | 'B'>('A');
  const [isFadingOut, setIsFadingOut] = useState<boolean>(false);
  const activeUrlRef = useRef<string | null>(url);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (url === activeUrlRef.current) return;

    if (!url) {
      activeUrlRef.current = null;
      setIsFadingOut(true);
      const tokens = readMotionTokens();
      const fadeDuration = tokens.isOff ? 0 : tokens.durBase;
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        setLayerA(null);
        setLayerB(null);
        setIsFadingOut(false);
      }, fadeDuration + 50);
      return;
    }

    setIsFadingOut(false);
    let isCancelled = false;
    const tokens = readMotionTokens();
    const isCrossfade = Boolean(activeUrlRef.current && (layerA || layerB));
    const targetSlot = !isCrossfade ? activeSlot : (activeSlot === 'A' ? 'B' : 'A');

    // Assign new URL to target slot
    if (targetSlot === 'A') {
      setLayerA(url);
    } else {
      setLayerB(url);
    }

    const proceed = () => {
      if (isCancelled) return;

      requestAnimationFrame(() => {
        if (isCancelled) return;
        requestAnimationFrame(() => {
          if (isCancelled) return;
          setActiveSlot(targetSlot);
          activeUrlRef.current = url;

          const duration = tokens.isOff ? 0 : (isCrossfade ? tokens.durScenic : tokens.durBase);
          if (timerRef.current) clearTimeout(timerRef.current);
          timerRef.current = setTimeout(() => {
            if (!isCancelled) {
              if (targetSlot === 'A') {
                setLayerB(null);
              } else {
                setLayerA(null);
              }
            }
          }, duration + 50);
        });
      });
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
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [url, activeSlot, layerA, layerB]);

  const isCrossfade = Boolean(layerA && layerB);
  const transitionClass = isFadingOut || !isCrossfade
    ? 'duration-[var(--dur-base)]'
    : 'duration-[var(--dur-scenic)]';

  return (
    <div
      className="wallpaper-layers fixed inset-0 pointer-events-none z-0 overflow-hidden"
      style={{ viewTransitionName: 'none' }}
      aria-hidden="true"
    >
      {/* Buffer Layer A */}
      {layerA && (
        <div
          className={`${className} transition-opacity ${transitionClass} ease-[var(--ease-inout)] ${
            !isFadingOut && activeSlot === 'A' ? 'opacity-100' : 'opacity-0'
          }`}
          style={{ backgroundImage: `url(${layerA})` }}
        >
          {overlayClassName && <div className={overlayClassName} />}
        </div>
      )}

      {/* Buffer Layer B */}
      {layerB && (
        <div
          className={`${className} transition-opacity ${transitionClass} ease-[var(--ease-inout)] ${
            !isFadingOut && activeSlot === 'B' ? 'opacity-100' : 'opacity-0'
          }`}
          style={{ backgroundImage: `url(${layerB})` }}
        >
          {overlayClassName && <div className={overlayClassName} />}
        </div>
      )}
    </div>
  );
};

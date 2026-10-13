import { useState, useEffect, useCallback } from 'react';
import { extractDominantColor, applyDynamicPalette } from '../utils/palette';

export interface WallpaperState {
  wallpaperUrl: string | null;
  dynamicColor: boolean;
  extractedColor: string | null;
  isLoading: boolean;
  setWallpaper: (url: string | null) => Promise<void>;
  setDynamicColor: (enabled: boolean) => void;
  clearWallpaper: () => void;
}

export function useWallpaper(): WallpaperState {
  const [wallpaperUrl, setWallpaperUrl] = useState<string | null>(() => {
    return localStorage.getItem('let_timer_wallpaper');
  });

  const [dynamicColor, setDynamicColorState] = useState<boolean>(() => {
    const val = localStorage.getItem('let_timer_dynamic_color');
    return val === null ? true : val === 'true';
  });

  const [extractedColor, setExtractedColor] = useState<string | null>(() => {
    return localStorage.getItem('let_timer_extracted_color');
  });

  const [isLoading, setIsLoading] = useState(false);

  // Sync state with DOM
  useEffect(() => {
    if (wallpaperUrl) {
      document.documentElement.setAttribute('data-has-wallpaper', 'true');
      if (dynamicColor && !extractedColor) {
        extractDominantColor(wallpaperUrl).then((color) => {
          localStorage.setItem('let_timer_extracted_color', color);
          setExtractedColor(color);
          applyDynamicPalette(color);
        });
      }
    } else {
      document.documentElement.removeAttribute('data-has-wallpaper');
    }

    if (dynamicColor && extractedColor) {
      applyDynamicPalette(extractedColor);
    } else if (!dynamicColor) {
      applyDynamicPalette(null);
    }
  }, [wallpaperUrl, dynamicColor, extractedColor]);

  // Re-apply palette when theme changes (light/dark switch)
  useEffect(() => {
    const observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        if (m.type === 'attributes' && m.attributeName === 'data-theme') {
          if (dynamicColor && extractedColor) {
            applyDynamicPalette(extractedColor);
          }
        }
      }
    });

    observer.observe(document.documentElement, { attributes: true });
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleMediaChange = () => {
      if (dynamicColor && extractedColor) {
        applyDynamicPalette(extractedColor);
      }
    };
    mediaQuery.addEventListener('change', handleMediaChange);

    return () => {
      observer.disconnect();
      mediaQuery.removeEventListener('change', handleMediaChange);
    };
  }, [dynamicColor, extractedColor]);

  const setWallpaper = useCallback(
    async (url: string | null) => {
      if (!url) {
        localStorage.removeItem('let_timer_wallpaper');
        localStorage.removeItem('let_timer_extracted_color');
        setWallpaperUrl(null);
        setExtractedColor(null);
        applyDynamicPalette(null);
        document.documentElement.removeAttribute('data-has-wallpaper');
        window.dispatchEvent(new CustomEvent('let-timer-wallpaper-changed'));
        return;
      }

      setIsLoading(true);
      try {
        localStorage.setItem('let_timer_wallpaper', url);
        setWallpaperUrl(url);
        document.documentElement.setAttribute('data-has-wallpaper', 'true');

        if (dynamicColor) {
          const color = await extractDominantColor(url);
          localStorage.setItem('let_timer_extracted_color', color);
          setExtractedColor(color);
          applyDynamicPalette(color);
        }
      } finally {
        setIsLoading(false);
        window.dispatchEvent(new CustomEvent('let-timer-wallpaper-changed'));
      }
    },
    [dynamicColor]
  );

  const setDynamicColor = useCallback(
    (enabled: boolean) => {
      setDynamicColorState(enabled);
      localStorage.setItem('let_timer_dynamic_color', String(enabled));

      if (enabled && extractedColor) {
        applyDynamicPalette(extractedColor);
      } else {
        applyDynamicPalette(null);
      }
      window.dispatchEvent(new CustomEvent('let-timer-wallpaper-changed'));
    },
    [extractedColor]
  );

  const clearWallpaper = useCallback(() => {
    setWallpaper(null);
  }, [setWallpaper]);

  // Sync across tabs/instances
  useEffect(() => {
    const handleSync = () => {
      const storedWp = localStorage.getItem('let_timer_wallpaper');
      const storedColor = localStorage.getItem('let_timer_extracted_color');
      const storedDyn = localStorage.getItem('let_timer_dynamic_color');
      setWallpaperUrl(storedWp);
      setExtractedColor(storedColor);
      setDynamicColorState(storedDyn === null ? true : storedDyn === 'true');
    };

    window.addEventListener('let-timer-wallpaper-changed', handleSync);
    return () => window.removeEventListener('let-timer-wallpaper-changed', handleSync);
  }, []);

  return {
    wallpaperUrl,
    dynamicColor,
    extractedColor,
    isLoading,
    setWallpaper,
    setDynamicColor,
    clearWallpaper,
  };
}


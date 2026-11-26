import { useState, useEffect } from 'react';
import { apiFetch } from '../api/client';

// Cache structure: mediaId or mediaUrl -> { url: string; expiresAt: number }
const signedUrlCache = new Map<number | string, { url: string; expiresAt: number }>();

export function getCachedSignedUrl(idOrUrl: number | string): string | null {
  const cached = signedUrlCache.get(idOrUrl);
  if (!cached) return null;
  const now = Math.floor(Date.now() / 1000);
  if (cached.expiresAt > now + 300) {
    return cached.url;
  }
  return null;
}

export function cacheSignedUrl(idOrUrl: number | string, url: string, expiresAt: number): void {
  signedUrlCache.set(idOrUrl, { url, expiresAt });
}

export function clearSignedUrlCache(): void {
  signedUrlCache.clear();
}

/**
 * Hook to resolve and refresh signed media URLs before expiry.
 */
export function useSignedMediaUrl(media?: { id?: number; url?: string } | null): string | null {
  const [resolvedUrl, setResolvedUrl] = useState<string | null>(media?.url || null);

  useEffect(() => {
    if (!media || !media.url) {
      setResolvedUrl(null);
      return;
    }

    // External URLs (http://, https://) don't need signature
    if (media.url.startsWith('http://') || media.url.startsWith('https://')) {
      setResolvedUrl(media.url);
      return;
    }

    const now = Math.floor(Date.now() / 1000);

    // If media.url itself has exp and sig and has not expired (with 5-minute safety buffer)
    const expMatch = media.url.match(/[?&]exp=(\d+)/);
    if (expMatch) {
      const exp = parseInt(expMatch[1], 10);
      if (exp > now + 300) {
        setResolvedUrl(media.url);
        return;
      }
    }

    // Check cache
    if (media.id) {
      const cached = getCachedSignedUrl(media.id);
      if (cached) {
        setResolvedUrl(cached);
        return;
      }
    }

    // Fetch fresh signed URL if id is available
    if (media.id) {
      apiFetch(`/api/v1/media/${media.id}/url`, { method: 'POST' })
        .then(async (res) => {
          if (res.ok) {
            const data = (await res.json()) as { url: string; expires_at: number };
            if (data?.url) {
              cacheSignedUrl(media.id!, data.url, data.expires_at);
              setResolvedUrl(data.url);
            }
          } else {
            setResolvedUrl(media.url || null);
          }
        })
        .catch(() => {
          setResolvedUrl(media.url || null);
        });
    } else {
      setResolvedUrl(media.url);
    }
  }, [media?.id, media?.url]);

  return resolvedUrl;
}


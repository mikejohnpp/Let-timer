import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import {
  useSignedMediaUrl,
  clearSignedUrlCache,
  cacheSignedUrl,
  getCachedSignedUrl,
} from '../hooks/useSignedMediaUrl';

vi.mock('../api/client', () => ({
  apiFetch: vi.fn(),
}));

import { apiFetch } from '../api/client';

describe('useSignedMediaUrl', () => {
  beforeEach(() => {
    clearSignedUrlCache();
    vi.clearAllMocks();
  });

  it('returns null when media is null or empty', () => {
    const { result } = renderHook(() => useSignedMediaUrl(null));
    expect(result.current).toBeNull();
  });

  it('passes external http/https URLs through without fetching', () => {
    const { result } = renderHook(() =>
      useSignedMediaUrl({ id: 1, url: 'https://example.com/audio.mp3' })
    );
    expect(result.current).toBe('https://example.com/audio.mp3');
    expect(apiFetch).not.toHaveBeenCalled();
  });

  it('uses already signed URL if exp is in the future', () => {
    const futureExp = Math.floor(Date.now() / 1000) + 1800;
    const url = `/api/v1/media/files/track.mp3?exp=${futureExp}&sig=abcdef`;

    const { result } = renderHook(() => useSignedMediaUrl({ id: 2, url }));
    expect(result.current).toBe(url);
    expect(apiFetch).not.toHaveBeenCalled();
  });

  it('uses cached signed URL if valid', () => {
    const futureExp = Math.floor(Date.now() / 1000) + 1800;
    const cachedUrl = `/api/v1/media/files/track.mp3?exp=${futureExp}&sig=cached123`;
    cacheSignedUrl(3, cachedUrl, futureExp);

    expect(getCachedSignedUrl(3)).toBe(cachedUrl);

    const { result } = renderHook(() =>
      useSignedMediaUrl({ id: 3, url: '/api/v1/media/files/track.mp3' })
    );
    expect(result.current).toBe(cachedUrl);
    expect(apiFetch).not.toHaveBeenCalled();
  });

  it('fetches fresh signed URL when media has no signature and caches it', async () => {
    const futureExp = Math.floor(Date.now() / 1000) + 3600;
    const freshSignedUrl = `/api/v1/media/files/track.mp3?exp=${futureExp}&sig=fresh456`;

    (apiFetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        url: freshSignedUrl,
        expires_at: futureExp,
      }),
    });

    const { result } = renderHook(() =>
      useSignedMediaUrl({ id: 4, url: '/api/v1/media/files/track.mp3' })
    );

    await waitFor(() => {
      expect(result.current).toBe(freshSignedUrl);
    });

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/media/4/url', { method: 'POST' });
    expect(getCachedSignedUrl(4)).toBe(freshSignedUrl);
  });
});


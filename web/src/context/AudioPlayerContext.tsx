import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useRef,
  useCallback,
  type ReactNode,
} from 'react';
import type { components } from '../api/schema';
import { apiFetch } from '../api/client';

export type MediaItem = components['schemas']['MediaDto'];
export type MediaListDetail = components['schemas']['MediaListDetailDto'];

export interface AudioPlayerContextType {
  currentTrack: MediaItem | null;
  playlist: MediaItem[];
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  isMuted: boolean;
  mediaListDetail: MediaListDetail | null;
  currentImageIndex: number;
  setCurrentImageIndex: React.Dispatch<React.SetStateAction<number>>;
  playTrack: (track: MediaItem, newPlaylist?: MediaItem[]) => void;
  togglePlay: () => void;
  pause: () => void;
  resume: () => void;
  seek: (seconds: number) => void;
  nextTrack: () => void;
  prevTrack: () => void;
  setVolume: (vol: number) => void;
  toggleMute: () => void;
  loadMediaListForTask: (mediaListId: number | null | undefined) => Promise<void>;
  clearPlayer: () => void;
}

const fallbackContext: AudioPlayerContextType = {
  currentTrack: null,
  playlist: [],
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  volume: 0.8,
  isMuted: false,
  mediaListDetail: null,
  currentImageIndex: 0,
  setCurrentImageIndex: () => {},
  playTrack: () => {},
  togglePlay: () => {},
  pause: () => {},
  resume: () => {},
  seek: () => {},
  nextTrack: () => {},
  prevTrack: () => {},
  setVolume: () => {},
  toggleMute: () => {},
  loadMediaListForTask: async () => {},
  clearPlayer: () => {},
};

export const AudioPlayerContext = createContext<AudioPlayerContextType>(fallbackContext);

export const useAudioPlayer = () => useContext(AudioPlayerContext);

interface AudioPlayerProviderProps {
  children: ReactNode;
}

export const AudioPlayerProvider: React.FC<AudioPlayerProviderProps> = ({ children }) => {
  const [currentTrack, setCurrentTrack] = useState<MediaItem | null>(null);
  const [playlist, setPlaylist] = useState<MediaItem[]>([]);
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [currentTime, setCurrentTime] = useState<number>(0);
  const [duration, setDuration] = useState<number>(0);
  const [volume, setVolumeState] = useState<number>(() => {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem('let_timer_player_volume');
      if (saved) {
        const parsed = parseFloat(saved);
        if (!isNaN(parsed) && parsed >= 0 && parsed <= 1) return parsed;
      }
    }
    return 0.8;
  });
  const [isMuted, setIsMuted] = useState<boolean>(false);
  const [mediaListDetail, setMediaListDetail] = useState<MediaListDetail | null>(null);
  const [currentImageIndex, setCurrentImageIndex] = useState<number>(0);

  const audioRef = useRef<HTMLAudioElement | null>(null);
  const loadedListIdRef = useRef<number | null>(null);

  // Initialize and update volume
  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.volume = isMuted ? 0 : volume;
    }
  }, [volume, isMuted]);

  // Audio element event bindings
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    const handleTimeUpdate = () => {
      setCurrentTime(audio.currentTime);
    };

    const handleDurationChange = () => {
      setDuration(audio.duration || 0);
    };

    const handlePlay = () => {
      setIsPlaying(true);
    };

    const handlePause = () => {
      setIsPlaying(false);
    };

    const handleEnded = () => {
      // Auto-advance to next track in playlist
      setPlaylist((currentPlaylist) => {
        if (currentPlaylist.length > 1) {
          setCurrentTrack((prevTrack) => {
            if (!prevTrack) return null;
            const idx = currentPlaylist.findIndex((t) => t.id === prevTrack.id);
            const nextIdx = (idx + 1) % currentPlaylist.length;
            const next = currentPlaylist[nextIdx];
            if (audio) {
              audio.src = next.url;
              audio.currentTime = 0;
              audio.play().catch(() => {});
            }
            return next;
          });
        } else if (currentPlaylist.length === 1) {
          audio.currentTime = 0;
          audio.play().catch(() => {});
        } else {
          setIsPlaying(false);
        }
        return currentPlaylist;
      });
    };

    const handleError = () => {
      setIsPlaying(false);
    };

    audio.addEventListener('timeupdate', handleTimeUpdate);
    audio.addEventListener('durationchange', handleDurationChange);
    audio.addEventListener('loadedmetadata', handleDurationChange);
    audio.addEventListener('play', handlePlay);
    audio.addEventListener('pause', handlePause);
    audio.addEventListener('ended', handleEnded);
    audio.addEventListener('error', handleError);

    return () => {
      audio.removeEventListener('timeupdate', handleTimeUpdate);
      audio.removeEventListener('durationchange', handleDurationChange);
      audio.removeEventListener('loadedmetadata', handleDurationChange);
      audio.removeEventListener('play', handlePlay);
      audio.removeEventListener('pause', handlePause);
      audio.removeEventListener('ended', handleEnded);
      audio.removeEventListener('error', handleError);
    };
  }, []);

  const playTrack = useCallback((track: MediaItem, newPlaylist?: MediaItem[]) => {
    setCurrentTrack(track);
    if (newPlaylist) {
      setPlaylist(newPlaylist);
    } else {
      setPlaylist((prev) => {
        if (!prev.some((t) => t.id === track.id)) {
          return [track, ...prev];
        }
        return prev;
      });
    }

    if (audioRef.current) {
      if (audioRef.current.src !== track.url) {
        audioRef.current.src = track.url;
      }
      audioRef.current.currentTime = 0;
      audioRef.current.play().catch((err) => {
        console.warn('Playback prevented or failed:', err);
      });
    }
  }, []);

  const togglePlay = useCallback(() => {
    if (!audioRef.current) return;

    if (!currentTrack) {
      if (playlist.length > 0) {
        playTrack(playlist[0]);
      }
      return;
    }

    if (isPlaying) {
      audioRef.current.pause();
    } else {
      audioRef.current.play().catch((err) => {
        console.warn('Playback error:', err);
      });
    }
  }, [currentTrack, isPlaying, playlist, playTrack]);

  const pause = useCallback(() => {
    if (audioRef.current && isPlaying) {
      audioRef.current.pause();
    }
  }, [isPlaying]);

  const resume = useCallback(() => {
    if (audioRef.current && !isPlaying && currentTrack) {
      audioRef.current.play().catch(() => {});
    }
  }, [isPlaying, currentTrack]);

  const seek = useCallback((seconds: number) => {
    if (audioRef.current) {
      audioRef.current.currentTime = seconds;
      setCurrentTime(seconds);
    }
  }, []);

  const nextTrack = useCallback(() => {
    if (playlist.length === 0 || !currentTrack) return;
    const idx = playlist.findIndex((t) => t.id === currentTrack.id);
    const nextIdx = (idx + 1) % playlist.length;
    playTrack(playlist[nextIdx]);
  }, [playlist, currentTrack, playTrack]);

  const prevTrack = useCallback(() => {
    if (playlist.length === 0 || !currentTrack) return;
    if (audioRef.current && audioRef.current.currentTime > 3) {
      seek(0);
      return;
    }
    const idx = playlist.findIndex((t) => t.id === currentTrack.id);
    const prevIdx = (idx - 1 + playlist.length) % playlist.length;
    playTrack(playlist[prevIdx]);
  }, [playlist, currentTrack, playTrack, seek]);

  const setVolume = useCallback((vol: number) => {
    const clamped = Math.max(0, Math.min(1, vol));
    setVolumeState(clamped);
    if (clamped > 0 && isMuted) {
      setIsMuted(false);
    }
    if (typeof window !== 'undefined') {
      localStorage.setItem('let_timer_player_volume', String(clamped));
    }
  }, [isMuted]);

  const toggleMute = useCallback(() => {
    setIsMuted((prev) => !prev);
  }, []);

  const clearPlayer = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = '';
    }
    setCurrentTrack(null);
    setPlaylist([]);
    setMediaListDetail(null);
    loadedListIdRef.current = null;
    setCurrentImageIndex(0);
    setIsPlaying(false);
    setCurrentTime(0);
    setDuration(0);
  }, []);

  const loadMediaListForTask = useCallback(async (mediaListId: number | null | undefined) => {
    if (!mediaListId) {
      clearPlayer();
      return;
    }

    if (loadedListIdRef.current === mediaListId) return;

    try {
      const res = await apiFetch(`/api/v1/media-lists/${mediaListId}`);
      if (res.ok) {
        const data: MediaListDetail = await res.json();
        setMediaListDetail(data);
        loadedListIdRef.current = mediaListId;

        const musicTracks = data.items.filter((item) => item.media_type === 'music');
        setPlaylist(musicTracks);
        setCurrentImageIndex(0);

        // Pre-select first music track if none currently selected
        setCurrentTrack((prev) => {
          if (!prev && musicTracks.length > 0) {
            if (audioRef.current) {
              audioRef.current.src = musicTracks[0].url;
            }
            return musicTracks[0];
          }
          return prev;
        });
      } else {
        clearPlayer();
      }
    } catch (e) {
      console.error('Failed to load media list detail for task', e);
      clearPlayer();
    }
  }, [clearPlayer]);

  return (
    <AudioPlayerContext.Provider
      value={{
        currentTrack,
        playlist,
        isPlaying,
        currentTime,
        duration,
        volume,
        isMuted,
        mediaListDetail,
        currentImageIndex,
        setCurrentImageIndex,
        playTrack,
        togglePlay,
        pause,
        resume,
        seek,
        nextTrack,
        prevTrack,
        setVolume,
        toggleMute,
        loadMediaListForTask,
        clearPlayer,
      }}
    >
      {children}
      {/* Global persistent audio element (never unmounts across route navigation) */}
      <audio ref={audioRef} preload="auto" style={{ display: 'none' }} />
    </AudioPlayerContext.Provider>
  );
};

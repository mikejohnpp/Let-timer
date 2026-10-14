import React, { useState, useRef, useEffect } from 'react';
import {
  Music,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  ChevronUp,
  ChevronDown,
  BookOpen,
  ExternalLink,
  Image as ImageIcon,
  ListMusic,
  Check,
} from 'lucide-react';
import { useAudioPlayer } from '../context/AudioPlayerContext';
import { SoundWave } from './SoundWave';
import { motion, AnimatePresence } from 'motion/react';
import { useMotionTokens, useMotionTransition } from '../motion';
import { TimerDigits } from './ui';

function formatTime(secs: number): string {
  if (isNaN(secs) || secs < 0) return '0:00';
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

interface FloatingPlayerIslandProps {
  className?: string;
  hasBackgroundPhoto?: boolean;
  layoutScope?: string;
}

export const FloatingPlayerIsland: React.FC<FloatingPlayerIslandProps> = ({
  className = '',
  hasBackgroundPhoto = false,
  layoutScope,
}) => {
  const {
    currentTrack,
    playlist,
    isPlaying,
    currentTime,
    duration,
    volume,
    isMuted,
    mediaListDetail,
    playTrack,
    togglePlay,
    seek,
    nextTrack,
    prevTrack,
    setVolume,
    toggleMute,
    currentImageIndex,
    setCurrentImageIndex,
  } = useAudioPlayer();

  const [isExpanded, setIsExpanded] = useState(false);
  const tokens = useMotionTokens();
  const isReduced = tokens.isReduced;
  const islandLayoutId = isReduced ? undefined : layoutScope ? `${layoutScope}-floating-island` : 'floating-island';
  const islandTransition = useMotionTransition('fast');
  const [activeTab, setActiveTab] = useState<'music' | 'background' | 'poetry'>('music');
  const [showQueue, setShowQueue] = useState(false);
  const queueTransition = useMotionTransition('base');
  const [scrubTime, setScrubTime] = useState<number | null>(null);
  const scrubTimeRef = useRef<number | null>(null);
  const isScrubbingRef = useRef<boolean>(false);
  const islandRef = useRef<HTMLDivElement | null>(null);
  const activeTrackRef = useRef<HTMLDivElement | null>(null);
  const hoverCloseTimerRef = useRef<number | null>(null);
  const mobileInactivityTimerRef = useRef<number | null>(null);

  const poems = mediaListDetail?.items.filter((i) => i.media_type === 'poem') || [];
  const tracks = playlist.length > 0 ? playlist : mediaListDetail?.items.filter((i) => i.media_type === 'music') || [];
  const focusImages = mediaListDetail?.items.filter((i) => i.media_type === 'image' && i.url) || [];

  // If active tab was background but current task has no images, fallback to music
  useEffect(() => {
    if (focusImages.length === 0 && activeTab === 'background') {
      setActiveTab('music');
    }
  }, [focusImages.length, activeTab]);

  useEffect(() => {
    if (activeTrackRef.current && isExpanded && activeTab === 'music' && showQueue) {
      activeTrackRef.current.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }, [currentTrack?.id, isExpanded, activeTab, showQueue]);

  const lastActiveTrackRef = useRef(currentTrack);
  if (currentTrack) {
    lastActiveTrackRef.current = currentTrack;
  }
  const track = currentTrack || lastActiveTrackRef.current;

  // Desktop hover detection helper
  const canHover = () => {
    return typeof window !== 'undefined' && window.matchMedia('(hover: hover) and (pointer: fine)').matches;
  };

  const handleMouseEnter = () => {
    if (hoverCloseTimerRef.current) {
      window.clearTimeout(hoverCloseTimerRef.current);
      hoverCloseTimerRef.current = null;
    }
    if (canHover()) {
      setIsExpanded(true);
    }
  };

  const handleMouseLeave = () => {
    if (canHover()) {
      if (hoverCloseTimerRef.current) window.clearTimeout(hoverCloseTimerRef.current);
      hoverCloseTimerRef.current = window.setTimeout(() => {
        setIsExpanded(false);
      }, 250);
    }
  };

  // Mobile / touch inactivity auto-close timer (after 4s of no interaction)
  const resetMobileInactivityTimer = () => {
    if (mobileInactivityTimerRef.current) {
      window.clearTimeout(mobileInactivityTimerRef.current);
      mobileInactivityTimerRef.current = null;
    }
    if (!canHover() && isExpanded) {
      mobileInactivityTimerRef.current = window.setTimeout(() => {
        setIsExpanded(false);
      }, 4000);
    }
  };

  useEffect(() => {
    if (!isExpanded) {
      if (mobileInactivityTimerRef.current) {
        window.clearTimeout(mobileInactivityTimerRef.current);
        mobileInactivityTimerRef.current = null;
      }
      return;
    }

    resetMobileInactivityTimer();

    const handleInteraction = () => {
      resetMobileInactivityTimer();
    };

    const islandEl = islandRef.current;
    if (islandEl) {
      islandEl.addEventListener('touchstart', handleInteraction, { passive: true });
      islandEl.addEventListener('touchmove', handleInteraction, { passive: true });
      islandEl.addEventListener('click', handleInteraction);
    }

    return () => {
      if (islandEl) {
        islandEl.removeEventListener('touchstart', handleInteraction);
        islandEl.removeEventListener('touchmove', handleInteraction);
        islandEl.removeEventListener('click', handleInteraction);
      }
      if (mobileInactivityTimerRef.current) {
        window.clearTimeout(mobileInactivityTimerRef.current);
        mobileInactivityTimerRef.current = null;
      }
    };
  }, [isExpanded]);

  // Close expanded card on click/tap outside
  useEffect(() => {
    if (!isExpanded) return;
    const handleClickOutside = (e: MouseEvent | TouchEvent) => {
      if (islandRef.current && !islandRef.current.contains(e.target as Node)) {
        setIsExpanded(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('touchstart', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('touchstart', handleClickOutside);
    };
  }, [isExpanded]);

  // If there's no track, no tracks, no poems, and no focus images, nothing to show
  if (!track && tracks.length === 0 && poems.length === 0 && focusImages.length === 0) {
    return null;
  }

  const displayTime = scrubTime !== null ? scrubTime : currentTime;
  const progressPercent = duration > 0 ? Math.min(100, (displayTime / duration) * 100) : 0;

  return (
    <div
      ref={islandRef}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      className={`z-40 select-none ${className}`}
    >
      {/* ─── EXPANDED FLOATING ISLAND CARD ─────────────────────────────────── */}
      <AnimatePresence mode="wait" initial={false}>
        {isExpanded ? (
          <motion.div
            key="card"
            layoutId={islandLayoutId}
            initial={{ opacity: 0, scale: isReduced ? 1 : 0.96 }} // motion-allow: island expansion morph (A26)
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: isReduced ? 1 : 0.96 }} // motion-allow: island expansion morph (A26)
            transition={islandTransition}
            className={`origin-bottom w-[92vw] max-w-md rounded-2xl p-4 sm:p-5 shadow-2xl border flex flex-col gap-4 max-h-[80vh] overflow-hidden ${
              hasBackgroundPhoto
                ? 'bg-[var(--surface)]/95 backdrop-blur-2xl border-[var(--border-strong)]/40 text-[var(--text)]'
                : 'bg-[var(--surface)]/95 backdrop-blur-2xl border-[var(--border)] shadow-xl text-[var(--text)]'
            }`}
          >
          {/* Header Bar */}
          <div className="flex items-center justify-between pb-2 border-b border-[var(--border)]/40">
            <div className="flex items-center gap-1.5">
              {tracks.length > 0 && (
                <button
                  type="button"
                  onClick={() => setActiveTab('music')}
                  className={`px-3 py-1 rounded-full text-xs font-semibold transition cursor-pointer ${
                    activeTab === 'music'
                      ? 'bg-[var(--accent)] text-[var(--on-accent)] shadow-xs'
                      : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
                  }`}
                >
                  <span className="flex items-center gap-1.5">
                    <Music className="w-3 h-3" />
                    <span>Music ({tracks.length})</span>
                  </span>
                </button>
              )}

              {focusImages.length > 0 && (
                <button
                  type="button"
                  onClick={() => setActiveTab('background')}
                  className={`px-3 py-1 rounded-full text-xs font-semibold transition cursor-pointer ${
                    activeTab === 'background'
                      ? 'bg-[var(--accent)] text-[var(--on-accent)] shadow-xs'
                      : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
                  }`}
                >
                  <span className="flex items-center gap-1.5">
                    <ImageIcon className="w-3 h-3" />
                    <span>Background ({focusImages.length})</span>
                  </span>
                </button>
              )}

              {poems.length > 0 && (
                <button
                  type="button"
                  onClick={() => setActiveTab('poetry')}
                  className={`px-3 py-1 rounded-full text-xs font-semibold transition cursor-pointer ${
                    activeTab === 'poetry'
                      ? 'bg-[var(--accent)] text-[var(--on-accent)] shadow-xs'
                      : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]'
                  }`}
                >
                  <span className="flex items-center gap-1.5">
                    <BookOpen className="w-3 h-3" />
                    <span>Poetry ({poems.length})</span>
                  </span>
                </button>
              )}
            </div>

            <button
              type="button"
              onClick={() => setIsExpanded(false)}
              className="p-1 rounded-full hover:bg-[var(--surface-hover)] text-[var(--muted)] hover:text-[var(--text)] transition cursor-pointer"
              aria-label="Collapse player"
              title="Collapse"
            >
              <ChevronDown className="w-4 h-4" />
            </button>
          </div>

          {/* Tab 1: Music Player */}
          {activeTab === 'music' && (
            <div className="flex flex-col gap-3.5">
              {/* Now Playing Header */}
              {track ? (
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)]/50 flex items-center justify-center shrink-0 text-[var(--accent-text)]">
                    <SoundWave isPlaying={isPlaying} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-semibold text-[var(--text)] truncate">
                      {track.name}
                    </div>
                    <div className="text-xs text-[var(--muted)] truncate mt-0.5">
                      {track.description || mediaListDetail?.list.name || 'Focus Audio'}
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-xs text-[var(--muted)] text-center py-2">
                  Select a track from the playlist below
                </div>
              )}

              {/* Interactive Scrub Bar */}
              <div className="flex flex-col gap-1">
                <div className="relative flex items-center group">
                  <input
                    type="range"
                    min={0}
                    max={duration || 100}
                    step={0.5}
                    value={displayTime}
                    disabled={!currentTrack || duration <= 0}
                    onPointerDown={(e) => {
                      isScrubbingRef.current = true;
                      try {
                        e.currentTarget.setPointerCapture(e.pointerId);
                      } catch {}
                      const val = parseFloat(e.currentTarget.value);
                      scrubTimeRef.current = val;
                      setScrubTime(val);
                    }}
                    onPointerUp={(e) => {
                      try {
                        if (e.currentTarget.hasPointerCapture(e.pointerId)) {
                          e.currentTarget.releasePointerCapture(e.pointerId);
                        }
                      } catch {}
                      if (isScrubbingRef.current) {
                        isScrubbingRef.current = false;
                        if (scrubTimeRef.current !== null) {
                          seek(scrubTimeRef.current);
                        }
                        setScrubTime(null);
                      }
                    }}
                    onPointerCancel={() => {
                      isScrubbingRef.current = false;
                      setScrubTime(null);
                    }}
                    onChange={(e) => {
                      const val = parseFloat(e.target.value);
                      scrubTimeRef.current = val;
                      setScrubTime(val);
                      if (!isScrubbingRef.current) {
                        seek(val);
                      }
                    }}
                    className="w-full h-1.5 bg-[var(--border)]/60 rounded-full appearance-none cursor-pointer outline-none accent-[var(--accent)]"
                    style={{
                      background: `linear-gradient(to right, var(--accent) 0%, var(--accent) ${progressPercent}%, var(--border) ${progressPercent}%, var(--border) 100%)`,
                    }}
                    aria-label="Seek track"
                  />
                </div>
                <div className="flex justify-between items-center px-0.5">
                  <TimerDigits digits={formatTime(displayTime)} mode="idle" size="inline" />
                  <TimerDigits digits={formatTime(duration)} mode="idle" size="inline" />
                </div>
              </div>

              {/* Main Playback Controls */}
              <div className="flex items-center justify-between pt-1">
                {/* Volume cluster */}
                <div className="flex items-center gap-1.5 w-24">
                  <button
                    type="button"
                    onClick={toggleMute}
                    className="p-1 rounded text-[var(--muted)] hover:text-[var(--text)] transition cursor-pointer"
                    aria-label={isMuted ? 'Unmute' : 'Mute'}
                  >
                    {isMuted || volume === 0 ? (
                      <VolumeX className="w-3.5 h-3.5" />
                    ) : (
                      <Volume2 className="w-3.5 h-3.5" />
                    )}
                  </button>
                  <input
                    type="range"
                    min={0}
                    max={1}
                    step={0.05}
                    value={isMuted ? 0 : volume}
                    onChange={(e) => setVolume(parseFloat(e.target.value))}
                    className="w-16 h-1 bg-[var(--border)] rounded-full appearance-none cursor-pointer outline-none accent-[var(--accent)]"
                    aria-label="Volume"
                  />
                </div>

                {/* Center play buttons */}
                <div className="flex items-center gap-3">
                  <button
                    type="button"
                    onClick={prevTrack}
                    disabled={tracks.length <= 1}
                    className="p-1.5 rounded-full text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition disabled:opacity-40 cursor-pointer"
                    aria-label="Previous track"
                  >
                    <SkipBack className="w-4 h-4" />
                  </button>

                  <button
                    type="button"
                    onClick={togglePlay}
                    className="w-10 h-10 rounded-full bg-[var(--accent)] text-[var(--on-accent)] flex items-center justify-center shadow-lg active:scale-[0.98] transition-[background-color,color,transform] duration-[var(--dur-fast)] active:duration-[var(--dur-instant)] cursor-pointer"
                    aria-label={isPlaying ? 'Pause' : 'Play'}
                  >
                    {isPlaying ? (
                      <Pause className="w-4 h-4 fill-current" />
                    ) : (
                      <Play className="w-4 h-4 fill-current ml-0.5" />
                    )}
                  </button>

                  <button
                    type="button"
                    onClick={nextTrack}
                    disabled={tracks.length <= 1}
                    className="p-1.5 rounded-full text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition disabled:opacity-40 cursor-pointer"
                    aria-label="Next track"
                  >
                    <SkipForward className="w-4 h-4" />
                  </button>
                </div>

                {/* Queue toggle button */}
                <div className="w-24 flex justify-end">
                  {tracks.length > 0 && (
                    <button
                      type="button"
                      onClick={() => setShowQueue(!showQueue)}
                      className={`flex items-center gap-1 px-2 py-1 rounded-md text-xs tabular-nums transition cursor-pointer ${
                        showQueue
                          ? 'bg-[var(--surface-hover)] text-[var(--accent-text)] font-semibold'
                          : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]/60'
                      }`}
                      aria-label={showQueue ? 'Hide queue' : 'Show queue'}
                      title={showQueue ? 'Hide queue' : 'Show queue'}
                    >
                      <ListMusic className="w-3.5 h-3.5" />
                      <span>
                        {currentTrack ? tracks.findIndex((t) => t.id === currentTrack.id) + 1 : 1}/{tracks.length}
                      </span>
                      {showQueue ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
                    </button>
                  )}
                </div>
              </div>

              {/* Collapsible Playlist Queue */}
              <AnimatePresence>
                {showQueue && tracks.length > 0 && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: 'auto' }}
                    exit={{ opacity: 0, height: 0 }}
                    transition={queueTransition}
                    className="mt-1 pt-2 border-t border-[var(--border)]/30 max-h-36 overflow-y-auto divide-y divide-[var(--border)]/20 text-xs"
                  >
                    {tracks.map((t) => {
                      const isSelected = currentTrack?.id === t.id;
                      return (
                        <div
                          key={t.id}
                          ref={isSelected ? activeTrackRef : undefined}
                          onClick={() => playTrack(t, tracks)}
                          className={`py-2 px-2.5 rounded-lg flex items-center justify-between gap-2 cursor-pointer transition ${
                            isSelected
                              ? 'bg-[var(--surface-hover)] font-semibold text-[var(--accent-text)]'
                              : 'text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)]/50'
                          }`}
                        >
                          <div className="flex items-center gap-2 min-w-0">
                            {isSelected && isPlaying ? (
                              <SoundWave isPlaying={true} className="shrink-0" />
                            ) : (
                              <Music className="w-3.5 h-3.5 shrink-0 opacity-70" />
                            )}
                            <span className="truncate">{t.name}</span>
                          </div>
                          {t.description && (
                            <span className="text-xs text-[var(--muted)] truncate max-w-[100px]">
                              {t.description}
                            </span>
                          )}
                        </div>
                      );
                    })}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )}

          {/* Tab 2: Task Focus Background Selector */}
          {activeTab === 'background' && focusImages.length > 0 && (
            <div className="flex flex-col gap-2.5 py-1 px-1">
              <div className="flex items-center justify-between text-xs text-[var(--muted)] px-1">
                <span>Task focus background</span>
                <span className="text-xs tabular-nums">
                  {currentImageIndex + 1}/{focusImages.length}
                </span>
              </div>

              <div className="grid grid-cols-3 gap-2 max-h-56 overflow-y-auto pr-0.5">
                {focusImages.map((img, idx) => {
                  const isSelected = currentImageIndex === idx;
                  return (
                    <button
                      key={img.id || idx}
                      type="button"
                      onClick={() => setCurrentImageIndex(idx)}
                      className={`group relative aspect-video rounded-lg overflow-hidden border transition-all cursor-pointer text-left focus:outline-none ${
                        isSelected
                          ? 'border-[var(--accent)] ring-2 ring-[var(--accent)] shadow-md'
                          : 'border-[var(--border)] hover:border-[var(--text)]/40 hover:scale-[1.02]'
                      }`}
                    >
                      <img
                        src={img.url}
                        alt={img.name}
                        className="w-full h-full object-cover transition-transform duration-[var(--dur-medium)] ease-[var(--ease-out)] group-hover:scale-105"
                        loading="lazy"
                      />
                      <div className="absolute inset-0 bg-black/25 group-hover:bg-black/10 transition-colors" />
                      {isSelected && (
                        <div className="absolute top-1.5 right-1.5 w-5 h-5 rounded-full bg-[var(--accent)] text-[var(--on-accent)] flex items-center justify-center shadow-md">
                          <Check className="w-3 h-3 stroke-[3]" />
                        </div>
                      )}
                      <div className="absolute bottom-0 inset-x-0 p-1 bg-gradient-to-t from-black/80 via-black/40 to-transparent text-xs text-white truncate px-1.5">
                        {img.name}
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Tab 2: Poetry Reader */}
          {activeTab === 'poetry' && poems.length > 0 && (
            <div className="flex flex-col gap-4 max-h-64 overflow-y-auto py-1 px-1 text-center">
              {poems.map((poem) => (
                <article key={poem.id} className="border-b border-[var(--border)]/20 pb-3 last:border-b-0">
                  <h4 className="font-serif text-sm font-semibold leading-tight text-[var(--text)]">{poem.name}</h4>
                  {poem.description && (
                    <p className="text-xs text-[var(--muted)] italic mb-2">{poem.description}</p>
                  )}
                  {poem.content ? (
                    <div className="font-serif text-xs leading-relaxed whitespace-pre-line text-[var(--text)]/90 tracking-wide px-2">
                      {poem.content}
                    </div>
                  ) : (
                    <a
                      href={poem.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex items-center gap-1 text-xs text-[var(--accent-text)] hover:underline mt-1"
                    >
                      <span>Read original poem</span>
                      <ExternalLink className="w-3 h-3" />
                    </a>
                  )}
                </article>
              ))}
            </div>
          )}
        </motion.div>
      ) : (
        /* ─── COLLAPSED FLOATING ISLAND PILL ─────────────────────────────────── */
        <motion.div
          key="pill"
          layoutId={islandLayoutId}
          initial={{ opacity: 0, scale: isReduced ? 1 : 0.96 }} // motion-allow: island expansion morph (A26)
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: isReduced ? 1 : 0.96 }} // motion-allow: island expansion morph (A26)
          transition={islandTransition}
          role="region"
          aria-label={track ? `Floating audio player: ${track.name}` : 'Floating audio player'}
          className={`origin-bottom shrink-0 whitespace-nowrap flex items-center gap-2.5 px-3.5 py-1.5 rounded-full border select-none shadow-xl backdrop-blur-xl ${
            hasBackgroundPhoto
              ? 'bg-[var(--surface)]/90 border-[var(--border-strong)]/40 text-[var(--text)] shadow-black/40'
              : 'bg-[var(--surface)]/90 border-[var(--border-strong)]/30 text-[var(--text)]'
          }`}
        >
          {/* Sound wave / Music indicator */}
          <div
            onClick={() => setIsExpanded(true)}
            className="flex items-center gap-2 cursor-pointer min-w-0 h-6"
            title="Expand player"
          >
            <div className="w-6 h-6 grid place-items-center shrink-0">
              {isPlaying ? (
                <SoundWave isPlaying={true} />
              ) : (
                <Music className="w-3.5 h-3.5 text-[var(--muted)]" />
              )}
            </div>

            {/* Track Name */}
            <span className="text-xs font-semibold truncate max-w-[120px] sm:max-w-[200px] text-[var(--text)] leading-none">
              {track ? track.name : 'Focus media'}
            </span>

            {/* Time */}
            {duration > 0 && (
              <TimerDigits
                digits={formatTime(currentTime)}
                mode="idle"
                size="inline"
                className="hidden sm:inline leading-none"
              />
            )}
          </div>

          {/* Quick Play/Pause Button */}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              togglePlay();
            }}
            className="w-6 h-6 rounded-full bg-[var(--accent)] text-[var(--on-accent)] flex items-center justify-center active:scale-[0.98] transition-[background-color,color,transform] duration-[var(--dur-fast)] active:duration-[var(--dur-instant)] cursor-pointer shrink-0"
            aria-label={isPlaying ? 'Pause music' : 'Play music'}
            title={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? (
              <Pause className="w-2.5 h-2.5 fill-current" />
            ) : (
              <Play className="w-2.5 h-2.5 fill-current" />
            )}
          </button>

          {/* Expand Toggle */}
          <button
            type="button"
            onClick={() => setIsExpanded(true)}
            className="w-6 h-6 grid place-items-center text-[var(--muted)] hover:text-[var(--text)] transition cursor-pointer shrink-0"
            aria-label="Expand player"
            title="Expand player"
          >
            <ChevronUp className="w-3.5 h-3.5" />
          </button>
        </motion.div>
      )}
      </AnimatePresence>
    </div>
  );
};

import React, { useState, useEffect, useRef } from 'react';
import {
  Music,
  BookOpen,
  Volume2,
  ExternalLink,
  Play,
  Pause,
  Check,
  Plus,
  Compass,
  X,
  ChevronUp,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import type { components } from '../api/schema';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { getStoredToken } from '../api/client';
import { Button, useToast } from '../components/ui';

type MediaListDetail = components['schemas']['MediaListDetailDto'];
type MediaItem = components['schemas']['MediaDto'];

interface FocusViewProps {
  snapshot: TimerSnapshot | null;
  receivedAt: number;
  onRefreshTimer: () => void;
  onOpenStartPicker: () => void;
}

export const FocusView: React.FC<FocusViewProps> = ({
  snapshot,
  receivedAt,
  onRefreshTimer,
  onOpenStartPicker,
}) => {
  const [now, setNow] = useState<number>(Date.now());
  const [mediaListDetail, setMediaListDetail] = useState<MediaListDetail | null>(null);
  const [currentAudio, setCurrentAudio] = useState<MediaItem | null>(null);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [panelOpen, setPanelOpen] = useState(false);
  const [panelTab, setPanelTab] = useState<'music' | 'poetry'>('music');
  const [currentImageIndex, setCurrentImageIndex] = useState(0);

  const { showToast } = useToast();
  const hideTimerRef = useRef<number | null>(null);

  // Live ticking
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const timerInfo: FormattedTimer = computeTimerDisplay(snapshot, receivedAt, now);

  // Load media list attached to task
  useEffect(() => {
    if (!snapshot || !snapshot.task.media_list_id) {
      setMediaListDetail(null);
      return;
    }

    async function loadMediaList() {
      try {
        const token = getStoredToken();
        const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
        if (token) headers['Authorization'] = `Bearer ${token}`;

        const res = await fetch(`/api/v1/media-lists/${snapshot?.task.media_list_id}`, { headers });
        if (res.ok) {
          const data: MediaListDetail = await res.json();
          setMediaListDetail(data);
          const firstAudio = data.items.find((m) => m.media_type === 'music');
          if (firstAudio) setCurrentAudio(firstAudio);
        }
      } catch (e) {
        console.error('Failed to load media list detail', e);
      }
    }

    loadMediaList();
  }, [snapshot?.task.media_list_id]);

  // Activity detection for auto-hiding controls after 3s
  const resetHideTimer = () => {
    setControlsVisible(true);
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(() => {
      if (!panelOpen) {
        setControlsVisible(false);
      }
    }, 3000);
  };

  useEffect(() => {
    const handleActivity = () => resetHideTimer();
    window.addEventListener('mousemove', handleActivity);
    window.addEventListener('keydown', handleActivity);
    window.addEventListener('touchstart', handleActivity);
    resetHideTimer();

    return () => {
      window.removeEventListener('mousemove', handleActivity);
      window.removeEventListener('keydown', handleActivity);
      window.removeEventListener('touchstart', handleActivity);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
  }, [panelOpen]);

  // Filter items
  const images = mediaListDetail?.items.filter((i) => i.media_type === 'image' && i.url) || [];
  const tracks = mediaListDetail?.items.filter((i) => i.media_type === 'music') || [];
  const poems = mediaListDetail?.items.filter((i) => i.media_type === 'poem') || [];

  // Crossfade between images every 90s if > 1
  useEffect(() => {
    if (images.length <= 1) return;
    const interval = setInterval(() => {
      setCurrentImageIndex((prev) => (prev + 1) % images.length);
    }, 90000);
    return () => clearInterval(interval);
  }, [images.length]);

  if (!snapshot || snapshot.state === 'idle') {
    return (
      <div className="min-h-[calc(100vh-var(--header-h))] flex flex-col items-center justify-center p-6 text-center max-w-lg mx-auto">
        <Compass className="w-16 h-16 text-[var(--muted)] opacity-50 mb-4 animate-spin-slow" />
        <h2 className="text-xl font-bold text-[var(--text)]">Focus mode is idle</h2>
        <p className="text-xs text-[var(--muted)] mt-2 mb-6">
          Start a task to enter full focus with music, artwork, and poetry.
        </p>
        <Button variant="primary" size="md" onClick={onOpenStartPicker}>
          Choose a task to start
        </Button>
      </div>
    );
  }

  const isRunning = snapshot.state === 'running';
  const isPaused = snapshot.state === 'paused';
  const isOvertime = timerInfo.mode === 'overtime';
  const hasImageBg = images.length > 0;
  const appWallpaper = localStorage.getItem('let_timer_wallpaper');
  const effectiveBg = hasImageBg ? (images[currentImageIndex]?.url || null) : appWallpaper;
  const hasAnyBg = Boolean(effectiveBg);

  const handleLifecycle = async (action: 'pause' | 'resume' | 'done') => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks/${snapshot.task.id}/${action}`, {
        method: 'POST',
        headers,
      });
      if (res.ok) onRefreshTimer();
      else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Action failed', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Action failed', 'danger');
    }
  };

  const handleAddFiveMinutes = async () => {
    const currentEst = snapshot.task.estimated_mins || Math.ceil(snapshot.elapsed_secs / 60);
    const newEst = currentEst + 5;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks/${snapshot.task.id}`, {
        method: 'PATCH',
        headers,
        body: JSON.stringify({ estimated_mins: newEst }),
      });

      if (res.ok) {
        showToast(`Extended estimate to ${newEst}m`, 'success');
        onRefreshTimer();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to extend estimate', 'danger');
    }
  };

  return (
    <div className="relative min-h-screen w-full overflow-hidden flex flex-col justify-between p-6 sm:p-12 pt-14 sm:pt-16 select-none">
      {/* Layer 0: Background Photo */}
      {hasAnyBg && effectiveBg && (
        <div
          className="absolute inset-0 bg-cover bg-center transition-all duration-1000 z-0"
          style={{ backgroundImage: `url(${effectiveBg})` }}
        />
      )}

      {/* Layer 1: 65% Black Overlay (when image is present) */}
      {hasAnyBg && (
        <div className="absolute inset-0 bg-[var(--imm-overlay)] z-1 pointer-events-none" />
      )}

      {/* Layer 2: Top Space & Workspace label */}
      <div
        className={`relative z-2 text-center transition-opacity duration-200 ${
          controlsVisible ? 'opacity-100' : 'opacity-0'
        }`}
      >
        <span
          className={`text-xs uppercase tracking-widest font-semibold ${
            hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
          }`}
        >
          {snapshot.workspace_name}
        </span>
      </div>

      {/* Layer 2: Main Timer Center Stage */}
      <div className="relative z-2 my-auto text-center flex flex-col items-center">
        {/* Giant Timer Digits */}
        <div
          className={`font-mono font-light tabular-nums tracking-tighter text-6xl sm:text-8xl md:text-9xl transition-colors ${
            isOvertime
              ? 'text-[var(--over-text)]'
              : isPaused
              ? hasAnyBg
                ? 'text-[var(--imm-muted)]'
                : 'text-[var(--muted)]'
              : hasAnyBg
              ? 'text-[var(--imm-text)]'
              : 'text-[var(--accent-text)]'
          }`}
          style={hasAnyBg ? { textShadow: '0 2px 10px rgba(0,0,0,0.5)' } : undefined}
        >
          {timerInfo.display}
        </div>

        {/* Task Name */}
        <h1
          className={`font-serif text-2xl sm:text-3xl md:text-4xl font-bold mt-4 max-w-2xl px-4 ${
            hasAnyBg ? 'text-[var(--imm-text)]' : 'text-[var(--text)]'
          }`}
        >
          {snapshot.task.name}
        </h1>

        {snapshot.task.description && (
          <p
            className={`text-xs sm:text-sm mt-2 max-w-lg truncate ${
              hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
            }`}
          >
            {snapshot.task.description}
          </p>
        )}

        {/* Action Controls (Fade out when idle) */}
        <div
          className={`flex items-center justify-center gap-3 mt-8 transition-opacity duration-200 ${
            controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
          }`}
        >
          {isOvertime && (
            <button
              onClick={handleAddFiveMinutes}
              className="flex items-center gap-1.5 px-4 py-2.5 rounded-full bg-[var(--timer-over)] text-[var(--on-over)] font-bold text-xs shadow-md hover:opacity-90 transition cursor-pointer"
            >
              <Plus className="w-3.5 h-3.5 stroke-[3]" />
              <span>+5 min</span>
            </button>
          )}

          {isRunning ? (
            <button
              onClick={() => handleLifecycle('pause')}
              className={`w-12 h-12 rounded-full border flex items-center justify-center transition cursor-pointer ${
                hasAnyBg
                  ? 'border-[var(--imm-muted)] text-[var(--imm-text)] hover:bg-white/10'
                  : 'border-[var(--border-strong)] text-[var(--text)] hover:bg-[var(--surface-hover)]'
              }`}
              aria-label="Pause timer"
            >
              <Pause className="w-5 h-5 fill-current" />
            </button>
          ) : (
            <button
              onClick={() => handleLifecycle('resume')}
              className="w-12 h-12 rounded-full bg-[var(--accent)] text-[var(--on-accent)] flex items-center justify-center shadow-md hover:bg-[var(--accent-hover)] transition cursor-pointer"
              aria-label="Resume timer"
            >
              <Play className="w-5 h-5 fill-current ml-0.5" />
            </button>
          )}

          <button
            onClick={() => handleLifecycle('done')}
            className="w-12 h-12 rounded-full bg-[var(--success)] text-white flex items-center justify-center shadow-md hover:opacity-90 transition cursor-pointer"
            aria-label="Complete task"
          >
            <Check className="w-5 h-5 stroke-[3]" />
          </button>
        </div>
      </div>

      {/* Layer 3: Bottom corner controls & hints */}
      <div
        className={`relative z-3 flex items-center justify-between gap-4 transition-opacity duration-200 ${
          controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
      >
        {/* Music chip (if tracks exist) */}
        {tracks.length > 0 || poems.length > 0 ? (
          <button
            onClick={() => setPanelOpen(true)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-full border text-xs backdrop-blur-sm transition cursor-pointer ${
              hasAnyBg
                ? 'bg-black/40 border-white/20 text-[var(--imm-text)] hover:bg-black/60'
                : 'bg-[var(--surface)] border-[var(--border)] text-[var(--text)] hover:bg-[var(--surface-hover)]'
            }`}
          >
            {tracks.length > 0 ? <Music className="w-3.5 h-3.5" /> : <BookOpen className="w-3.5 h-3.5" />}
            <span className="font-medium truncate max-w-[160px]">
              {currentAudio ? currentAudio.name : 'Focus media'}
            </span>
            <ChevronUp className="w-3.5 h-3.5 opacity-60" />
          </button>
        ) : (
          <div />
        )}

        <div
          className={`text-[11px] ${
            hasAnyBg ? 'text-[var(--imm-muted)]' : 'text-[var(--muted)]'
          }`}
        >
          Space: pause • Tap to wake
        </div>
      </div>

      {/* Layer 4: Media Panel Sheet (Music & Poems) */}
      {panelOpen && (
        <div className="fixed inset-x-0 bottom-0 z-50 max-h-[60vh] bg-[var(--surface)] border-t border-[var(--border)] shadow-2xl p-5 overflow-y-auto animate-in slide-in-from-bottom duration-200">
          <div className="max-w-2xl mx-auto flex flex-col gap-4">
            <div className="flex items-center justify-between border-b border-[var(--border)] pb-3">
              <div className="flex items-center gap-2">
                {tracks.length > 0 && (
                  <button
                    onClick={() => setPanelTab('music')}
                    className={`px-3 py-1 rounded-[var(--radius-control)] text-xs font-semibold cursor-pointer ${
                      panelTab === 'music'
                        ? 'bg-[var(--accent)] text-[var(--on-accent)]'
                        : 'text-[var(--muted)] hover:text-[var(--text)]'
                    }`}
                  >
                    Music ({tracks.length})
                  </button>
                )}
                {poems.length > 0 && (
                  <button
                    onClick={() => setPanelTab('poetry')}
                    className={`px-3 py-1 rounded-[var(--radius-control)] text-xs font-semibold cursor-pointer ${
                      panelTab === 'poetry'
                        ? 'bg-[var(--accent)] text-[var(--on-accent)]'
                        : 'text-[var(--muted)] hover:text-[var(--text)]'
                    }`}
                  >
                    Poetry ({poems.length})
                  </button>
                )}
              </div>

              <button
                onClick={() => setPanelOpen(false)}
                className="p-1 rounded text-[var(--muted)] hover:text-[var(--text)] cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Music Tab */}
            {panelTab === 'music' && tracks.length > 0 && (
              <div className="flex flex-col gap-4">
                {currentAudio && (
                  <div>
                    <div className="text-xs font-semibold text-[var(--muted)] mb-1">
                      Now playing: {currentAudio.name}
                    </div>
                    <audio
                      key={currentAudio.url}
                      controls
                      autoPlay
                      className="w-full h-9 rounded outline-none"
                      src={currentAudio.url}
                    >
                      Your browser does not support audio.
                    </audio>
                  </div>
                )}

                <div className="divide-y divide-[var(--border)] text-xs">
                  {tracks.map((t) => (
                    <div
                      key={t.id}
                      onClick={() => setCurrentAudio(t)}
                      className={`py-2 px-2 flex items-center justify-between rounded cursor-pointer transition ${
                        currentAudio?.id === t.id
                          ? 'bg-[var(--surface-hover)] font-bold text-[var(--accent-text)]'
                          : 'text-[var(--muted)] hover:text-[var(--text)]'
                      }`}
                    >
                      <div className="flex items-center gap-2">
                        <Volume2 className="w-3.5 h-3.5" />
                        <span>{t.name}</span>
                      </div>
                      {t.description && <span>{t.description}</span>}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Poetry Tab */}
            {panelTab === 'poetry' && poems.length > 0 && (
              <div className="flex flex-col gap-6 py-2">
                {poems.map((p) => (
                  <article key={p.id} className="text-center max-w-md mx-auto">
                    <h3 className="font-serif text-base font-bold text-[var(--text)]">{p.name}</h3>
                    {p.description && (
                      <p className="text-xs text-[var(--muted)] italic mb-3">{p.description}</p>
                    )}
                    {p.content ? (
                      <div className="font-serif text-xs sm:text-sm leading-relaxed whitespace-pre-line text-[var(--text)]/90 tracking-wide">
                        {p.content}
                      </div>
                    ) : (
                      <a
                        href={p.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center gap-1 text-xs text-[var(--accent-text)] hover:underline"
                      >
                        <span>Read original text</span>
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    )}
                  </article>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

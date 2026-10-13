import React, { useState, useEffect } from 'react';
import {
  Compass,
  Music,
  Image as ImageIcon,
  BookOpen,
  Volume2,
  ExternalLink,
  Play,
  Pause,
  Check,
} from 'lucide-react';
import type { TimerSnapshot } from '../hooks/useEventStream';
import type { components } from '../api/schema';
import { computeTimerDisplay, type FormattedTimer } from '../utils/timer';
import { getStoredToken } from '../api/client';

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
  const [activeImage, setActiveImage] = useState<string | null>(null);
  const [currentAudio, setCurrentAudio] = useState<MediaItem | null>(null);

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
          // Set first audio item if exists
          const firstAudio = data.items.find((m) => m.media_type === 'music');
          if (firstAudio) setCurrentAudio(firstAudio);
        }
      } catch (e) {
        console.error('Failed to load media list detail', e);
      }
    }

    loadMediaList();
  }, [snapshot?.task.media_list_id]);

  if (!snapshot || snapshot.state === 'idle') {
    return (
      <div className="max-w-4xl mx-auto px-4 py-20 text-center">
        <Compass className="w-16 h-16 text-[var(--muted)] mx-auto mb-4 opacity-40 animate-spin-slow" />
        <h2 className="text-2xl font-bold text-[var(--text)]">Focus Mode is Idle</h2>
        <p className="text-sm text-[var(--muted)] mt-2 mb-6">
          Start a task to enter full focus mode with ambient music, artwork, and poetry.
        </p>
        <button
          onClick={onOpenStartPicker}
          className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-5 py-2.5 rounded-xl text-sm font-semibold shadow-md transition"
        >
          Choose a Task to Start
        </button>
      </div>
    );
  }

  const isRunning = snapshot.state === 'running';
  const isOvertime = timerInfo.mode === 'overtime';

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
    } catch (e) {
      console.error(`Failed to ${action} task`, e);
    }
  };

  const images = mediaListDetail?.items.filter((i) => i.media_type === 'image') || [];
  const tracks = mediaListDetail?.items.filter((i) => i.media_type === 'music') || [];
  const poems = mediaListDetail?.items.filter((i) => i.media_type === 'poem') || [];

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 py-12 flex flex-col items-center">
      {/* Big Hero Focus Timer */}
      <div className="text-center mb-8">
        <div
          className={`text-6xl sm:text-7xl md:text-8xl font-black tabular-nums tracking-tighter ${
            isOvertime ? 'text-[var(--timer-over)]' : isRunning ? 'text-[var(--timer-run)]' : 'text-amber-500'
          }`}
        >
          {timerInfo.display}
        </div>

        <h1 className="text-2xl sm:text-3xl font-bold mt-4 text-[var(--text)]">
          {snapshot.task.name}
        </h1>

        {snapshot.task.description && (
          <p className="text-sm text-[var(--muted)] mt-2 max-w-lg mx-auto">
            {snapshot.task.description}
          </p>
        )}

        {/* Quick controls */}
        <div className="flex items-center justify-center gap-3 mt-6">
          {isRunning ? (
            <button
              onClick={() => handleLifecycle('pause')}
              className="flex items-center gap-2 px-5 py-2.5 rounded-2xl bg-[var(--surface)] border border-[var(--border)] hover:bg-[var(--surface-hover)] text-sm font-semibold shadow-sm transition"
            >
              <Pause className="w-4 h-4 fill-current" />
              <span>Pause</span>
            </button>
          ) : (
            <button
              onClick={() => handleLifecycle('resume')}
              className="flex items-center gap-2 px-5 py-2.5 rounded-2xl bg-[var(--timer-run)] text-white hover:opacity-90 text-sm font-semibold shadow-md transition"
            >
              <Play className="w-4 h-4 fill-current" />
              <span>Resume</span>
            </button>
          )}

          <button
            onClick={() => handleLifecycle('done')}
            className="flex items-center gap-2 px-5 py-2.5 rounded-2xl bg-[var(--success)] text-white hover:opacity-90 text-sm font-semibold shadow-md transition"
          >
            <Check className="w-4 h-4 stroke-[3]" />
            <span>Complete</span>
          </button>
        </div>
      </div>

      {/* Focus Media Section */}
      {mediaListDetail && (
        <div className="w-full mt-6 flex flex-col gap-8">
          {/* Audio Player if music tracks are present */}
          {tracks.length > 0 && (
            <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
              <div className="flex items-center gap-2 text-sm font-bold mb-3 text-[var(--text)]">
                <Music className="w-4 h-4 text-[var(--accent)]" />
                <span>Focus Audio Playlist</span>
              </div>

              {currentAudio && (
                <div className="mb-4">
                  <div className="text-xs font-semibold text-[var(--muted)] mb-1">
                    Playing: {currentAudio.name}
                  </div>
                  <audio
                    key={currentAudio.url}
                    controls
                    className="w-full h-10 rounded-lg outline-none"
                    src={currentAudio.url}
                  >
                    Your browser does not support the audio element.
                  </audio>
                </div>
              )}

              {/* Tracks Queue */}
              <div className="divide-y divide-[var(--border)] text-xs">
                {tracks.map((t) => (
                  <div
                    key={t.id}
                    onClick={() => setCurrentAudio(t)}
                    className={`py-2 px-3 flex items-center justify-between rounded-lg cursor-pointer transition ${
                      currentAudio?.id === t.id
                        ? 'bg-[var(--surface-hover)] font-semibold text-[var(--accent)]'
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

          {/* Poems Reading Column */}
          {poems.length > 0 && (
            <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-8 shadow-sm">
              <div className="flex items-center gap-2 text-sm font-bold mb-6 text-[var(--text)] border-b border-[var(--border)] pb-3">
                <BookOpen className="w-4 h-4 text-[var(--accent)]" />
                <span>Focus Poetry</span>
              </div>

              <div className="flex flex-col gap-8">
                {poems.map((poem) => (
                  <article key={poem.id} className="max-w-md mx-auto text-center">
                    <h3 className="font-serif text-lg font-bold text-[var(--text)] mb-2">
                      {poem.name}
                    </h3>
                    {poem.description && (
                      <p className="text-xs text-[var(--muted)] italic mb-4">
                        {poem.description}
                      </p>
                    )}
                    {poem.content ? (
                      <div className="font-serif text-sm leading-relaxed whitespace-pre-line text-[var(--text)]/90 tracking-wide">
                        {poem.content}
                      </div>
                    ) : (
                      <a
                        href={poem.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center gap-1 text-xs text-[var(--accent)] hover:underline"
                      >
                        <span>Read original text</span>
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    )}
                  </article>
                ))}
              </div>
            </div>
          )}

          {/* Images Gallery */}
          {images.length > 0 && (
            <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
              <div className="flex items-center gap-2 text-sm font-bold mb-4 text-[var(--text)]">
                <ImageIcon className="w-4 h-4 text-[var(--accent)]" />
                <span>Ambient Artwork</span>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                {images.map((img) => (
                  <div
                    key={img.id}
                    onClick={() => setActiveImage(img.url)}
                    className="aspect-video rounded-xl overflow-hidden cursor-pointer bg-[var(--surface-hover)] border border-[var(--border)] group relative"
                  >
                    <img
                      src={img.url}
                      alt={img.name}
                      referrerPolicy="no-referrer"
                      className="w-full h-full object-cover group-hover:scale-105 transition duration-300"
                    />
                    <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition flex items-end p-2">
                      <span className="text-white text-xs font-medium truncate">{img.name}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Lightbox Modal */}
      {activeImage && (
        <div
          onClick={() => setActiveImage(null)}
          className="fixed inset-0 z-50 bg-black/90 flex items-center justify-center p-4 cursor-pointer"
        >
          <img
            src={activeImage}
            alt="Focus artwork"
            referrerPolicy="no-referrer"
            className="max-w-full max-h-[90vh] rounded-xl shadow-2xl object-contain"
          />
        </div>
      )}
    </div>
  );
};

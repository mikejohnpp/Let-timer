import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { AudioPlayerProvider, useAudioPlayer } from '../context/AudioPlayerContext';
import { MotionProvider } from '../motion';
import { ToastProvider, ConfirmProvider } from '../components/ui';

// Mock client
vi.mock('../api/client', () => ({
  apiFetch: vi.fn(async (url: string) => {
    if (url === '/api/v1/media-lists/1') {
      return {
        ok: true,
        json: async () => ({
          list: { id: 1, name: 'Deep Focus' },
          items: [
            { id: 10, name: 'Ambient Track 1', url: '/track1.mp3', media_type: 'music' },
            { id: 11, name: 'Ambient Track 2', url: '/track2.mp3', media_type: 'music' },
            { id: 20, name: 'Calm River', url: '/river.jpg', media_type: 'image' },
          ],
        }),
      };
    }
    if (url === '/api/v1/auth/me') {
      return { ok: true, json: async () => ({}) };
    }
    return { ok: true, json: async () => ({}) };
  }),
  getStoredToken: () => null,
  getStoredWorkspace: () => '1',
}));

// Test harness simulating AppContent's task synchronization effect
interface TaskSyncHarnessProps {
  snapshotState: 'idle' | 'running' | 'paused';
  currentTaskId?: number;
  currentMediaListId?: number | null;
}

const TaskSyncHarness: React.FC<TaskSyncHarnessProps> = ({
  snapshotState,
  currentTaskId,
  currentMediaListId,
}) => {
  const {
    currentTrack,
    playlist,
    isPlaying,
    currentImageIndex,
    loadMediaListForTask,
    clearPlayer,
    togglePlay,
    pause,
    setCurrentImageIndex,
  } = useAudioPlayer();

  const prevSnapshotStateRef = React.useRef<string>(snapshotState);
  const prevTaskIdRef = React.useRef<number | undefined>(undefined);

  React.useEffect(() => {
    const prevState = prevSnapshotStateRef.current;
    const currentState = snapshotState;
    const prevTaskId = prevTaskIdRef.current;

    prevSnapshotStateRef.current = currentState;
    prevTaskIdRef.current = currentTaskId;

    if (prevState && prevState !== 'idle' && currentState === 'idle') {
      clearPlayer();
      return;
    }

    if ((prevState === 'idle' || !prevState) && currentState === 'running') {
      pause();
    }

    // Fixed bug: only sync when task is NOT idle
    if (currentState !== 'idle' && (prevTaskId !== currentTaskId || prevState === 'idle')) {
      if (currentMediaListId) {
        loadMediaListForTask(currentMediaListId);
      } else {
        clearPlayer();
      }
    }
  }, [snapshotState, currentTaskId, currentMediaListId, pause, clearPlayer, loadMediaListForTask]);

  return (
    <div>
      <div data-testid="current-track">{currentTrack ? currentTrack.name : 'none'}</div>
      <div data-testid="playlist-count">{playlist.length}</div>
      <div data-testid="is-playing">{isPlaying ? 'playing' : 'paused'}</div>
      <div data-testid="current-image-index">
        {currentImageIndex === null ? 'null' : String(currentImageIndex)}
      </div>
      <button type="button" onClick={() => loadMediaListForTask(1)}>
        Load Media List 1
      </button>
      <button type="button" onClick={togglePlay}>
        Toggle Play
      </button>
      <button type="button" onClick={() => setCurrentImageIndex(null)}>
        Set None BG
      </button>
    </div>
  );
};

describe('Idle Ambient Playback and Background None option', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('keeps audio playlist and track loaded during idle without being wiped out by clearPlayer', async () => {
    const { rerender } = render(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <AudioPlayerProvider>
              <TaskSyncHarness snapshotState="idle" currentTaskId={undefined} currentMediaListId={null} />
            </AudioPlayerProvider>
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );

    expect(screen.getByTestId('current-track')).toHaveTextContent('none');
    expect(screen.getByTestId('playlist-count')).toHaveTextContent('0');

    // Load media list 1 while idle
    fireEvent.click(screen.getByRole('button', { name: 'Load Media List 1' }));

    await waitFor(() => {
      expect(screen.getByTestId('current-track')).toHaveTextContent('Ambient Track 1');
      expect(screen.getByTestId('playlist-count')).toHaveTextContent('2');
      expect(screen.getByTestId('current-image-index')).toHaveTextContent('0');
    });

    // Simulate idle snapshot refresh / re-render
    rerender(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <AudioPlayerProvider>
              <TaskSyncHarness snapshotState="idle" currentTaskId={undefined} currentMediaListId={null} />
            </AudioPlayerProvider>
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );

    // Audio MUST still be intact!
    expect(screen.getByTestId('current-track')).toHaveTextContent('Ambient Track 1');
    expect(screen.getByTestId('playlist-count')).toHaveTextContent('2');

    // Set background to None
    fireEvent.click(screen.getByRole('button', { name: 'Set None BG' }));
    expect(screen.getByTestId('current-image-index')).toHaveTextContent('null');
  });

  it('clears player when a running task transitions to idle', async () => {
    const { rerender } = render(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <AudioPlayerProvider>
              <TaskSyncHarness snapshotState="running" currentTaskId={99} currentMediaListId={1} />
            </AudioPlayerProvider>
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );

    await waitFor(() => {
      expect(screen.getByTestId('current-track')).toHaveTextContent('Ambient Track 1');
    });

    // Task finishes -> state becomes idle
    rerender(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <AudioPlayerProvider>
              <TaskSyncHarness snapshotState="idle" currentTaskId={undefined} currentMediaListId={null} />
            </AudioPlayerProvider>
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );

    await waitFor(() => {
      expect(screen.getByTestId('current-track')).toHaveTextContent('none');
      expect(screen.getByTestId('playlist-count')).toHaveTextContent('0');
    });
  });
});

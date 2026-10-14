import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { FocusView } from '../views/FocusView';
import { AudioPlayerContext, type AudioPlayerContextType } from '../context/AudioPlayerContext';
import { MotionProvider } from '../motion';
import { ToastProvider, ConfirmProvider } from '../components/ui';

vi.mock('../api/client', () => ({
  apiFetch: vi.fn(async (url: string) => {
    if (url === '/api/v1/media-lists') {
      return {
        ok: true,
        json: async () => [
          { id: 1, name: 'Deep Focus' },
          { id: 2, name: 'Rainy Night' },
        ],
      };
    }
    if (url === '/api/v1/workspaces/current') {
      return {
        ok: true,
        json: async () => ({ id: 1, name: 'Default' }),
      };
    }
    return { ok: true, json: async () => ({}) };
  }),
  getStoredToken: () => null,
  getStoredWorkspace: () => '1',
}));

describe('P1: Focus Playlist Popover picker', () => {
  const mockLoadMediaList = vi.fn();
  const mockClearPlayer = vi.fn();

  const mockAudioContext: Partial<AudioPlayerContextType> = {
    currentTrack: null,
    playlist: [],
    isPlaying: false,
    currentTime: 0,
    duration: 0,
    volume: 1,
    isMuted: false,
    mediaListDetail: null,
    currentImageIndex: 0,
    loadMediaListForTask: mockLoadMediaList,
    clearPlayer: mockClearPlayer,
    playTrack: vi.fn(),
    togglePlay: vi.fn(),
    seek: vi.fn(),
    nextTrack: vi.fn(),
    prevTrack: vi.fn(),
    setVolume: vi.fn(),
    toggleMute: vi.fn(),
    pause: vi.fn(),
    setCurrentImageIndex: vi.fn(),
  };

  const renderComponent = () => {
    return render(
      <MotionProvider>
        <ToastProvider>
          <ConfirmProvider>
            <AudioPlayerContext.Provider value={mockAudioContext as AudioPlayerContextType}>
              <FocusView
                snapshot={null}
                receivedAt={Date.now()}
                onRefreshTimer={vi.fn()}
                onOpenStartPicker={vi.fn()}
              />
            </AudioPlayerContext.Provider>
          </ConfirmProvider>
        </ToastProvider>
      </MotionProvider>
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders trigger button and does NOT render any native select element', async () => {
    const { container } = renderComponent();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /select focus playlist/i })).toBeInTheDocument();
    });

    const selectElements = container.querySelectorAll('select');
    expect(selectElements.length).toBe(0);
  });

  it('opens popover listbox and selects a playlist', async () => {
    renderComponent();

    const trigger = await screen.findByRole('button', { name: /select focus playlist/i });
    expect(trigger).toHaveTextContent(/choose focus playlist/i);

    fireEvent.click(trigger);

    // Verify popover listbox appears
    const listbox = await screen.findByRole('listbox', { name: /focus playlists/i });
    expect(listbox).toBeInTheDocument();

    const options = screen.getAllByRole('option');
    expect(options.length).toBe(2);
    expect(options[0]).toHaveTextContent('Deep Focus');
    expect(options[1]).toHaveTextContent('Rainy Night');

    // Click on "Deep Focus"
    fireEvent.click(options[0]);

    // Verify loadMediaListForTask is called with playlist id
    expect(mockLoadMediaList).toHaveBeenCalledWith(1);

    // Verify trigger label updates to selected playlist
    await waitFor(() => {
      expect(trigger).toHaveTextContent('Deep Focus');
    });

    // Re-open and check that "None" option exists and selected option has aria-selected=true
    fireEvent.click(trigger);
    const updatedOptions = await screen.findAllByRole('option');
    expect(updatedOptions.length).toBe(3); // None, Deep Focus, Rainy Night
    expect(updatedOptions[0]).toHaveTextContent(/none/i);
    expect(updatedOptions[1]).toHaveAttribute('aria-selected', 'true');

    // Click None
    fireEvent.click(updatedOptions[0]);
    expect(mockClearPlayer).toHaveBeenCalled();
  });

  it('disables trigger when no playlists are available', async () => {
    const { apiFetch } = await import('../api/client');
    vi.mocked(apiFetch).mockImplementationOnce(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
      if (url === '/api/v1/media-lists') {
        return { ok: true, json: async () => [] } as Response;
      }
      return { ok: true, json: async () => ({}) } as Response;
    });

    renderComponent();

    const trigger = await screen.findByRole('button', { name: /select focus playlist/i });
    expect(trigger).toBeDisabled();
    expect(trigger).toHaveTextContent(/no playlists available/i);
  });
});

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { FloatingPlayerIsland } from '../components/FloatingPlayerIsland';
import { MotionProvider, invalidateMotionTokenCache } from '../motion';
import { AudioPlayerContext, type AudioPlayerContextType } from '../context/AudioPlayerContext';

const mockAudioContext: AudioPlayerContextType = {
  currentTrack: {
    id: 1,
    name: 'Ambient Rain',
    url: '/rain.mp3',
    media_type: 'music',
    created_at: '',
    updated_at: '',
  },
  playlist: [
    {
      id: 1,
      name: 'Ambient Rain',
      url: '/rain.mp3',
      media_type: 'music',
      created_at: '',
      updated_at: '',
    },
  ],
  mediaListDetail: null,
  currentImageIndex: 0,
  setCurrentImageIndex: vi.fn(),
  isPlaying: true,
  currentTime: 30,
  duration: 120,
  volume: 0.8,
  isMuted: false,
  playTrack: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
  togglePlay: vi.fn(),
  nextTrack: vi.fn(),
  prevTrack: vi.fn(),
  seek: vi.fn(),
  setVolume: vi.fn(),
  toggleMute: vi.fn(),
  loadMediaListForTask: vi.fn(),
  clearPlayer: vi.fn(),
};

describe('T4: FloatingPlayerIsland single Motion engine', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    invalidateMotionTokenCache();
    document.documentElement.removeAttribute('data-motion');
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }));
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    document.documentElement.removeAttribute('data-motion');
    invalidateMotionTokenCache();
  });

  it('renders pill without CSS animation class and expands to card without .island-card', async () => {
    render(
      <MotionProvider>
        <AudioPlayerContext.Provider value={mockAudioContext}>
          <FloatingPlayerIsland />
        </AudioPlayerContext.Provider>
      </MotionProvider>
    );

    // Initial collapsed pill
    const trackName = screen.getByText('Ambient Rain');
    expect(trackName).toBeTruthy();
    const pill = trackName.closest('div[class*="rounded-full"]');
    expect(pill).toBeTruthy();
    expect(pill?.classList.contains('island-pill')).toBe(false);

    // Expand
    const expandButton = screen.getByRole('button', { name: 'Expand player' });
    fireEvent.click(expandButton);

    // Wait for card to enter
    await waitFor(() => {
      expect(screen.getByText('Music (1)')).toBeTruthy();
    });

    const card = screen.getByText('Music (1)').closest('div[class*="rounded-2xl"]');
    expect(card).toBeTruthy();
    expect(card?.classList.contains('island-card')).toBe(false);
  });

  it('in reduced motion mode, renders without island classes', () => {
    document.documentElement.setAttribute('data-motion', 'reduced');
    invalidateMotionTokenCache();

    render(
      <MotionProvider>
        <AudioPlayerContext.Provider value={mockAudioContext}>
          <FloatingPlayerIsland />
        </AudioPlayerContext.Provider>
      </MotionProvider>
    );

    const trackName = screen.getByText('Ambient Rain');
    expect(trackName).toBeTruthy();
    expect(document.querySelector('.island-pill')).toBeNull();
    expect(document.querySelector('.island-card')).toBeNull();
  });

  it('unifies in-pill rhythm: fixed icon box, leading-none typography, symmetric play button', () => {
    const { container } = render(
      <MotionProvider>
        <AudioPlayerContext.Provider value={mockAudioContext}>
          <FloatingPlayerIsland layoutScope="focus" />
        </AudioPlayerContext.Provider>
      </MotionProvider>
    );

    const trackName = screen.getByText('Ambient Rain');
    const textGroup = trackName.closest('div');
    expect(textGroup?.classList.contains('items-baseline')).toBe(true);

    const playPauseBtn = screen.getByRole('button', { name: 'Pause music' });
    expect(playPauseBtn.classList.contains('w-6')).toBe(true);
    expect(playPauseBtn.classList.contains('h-6')).toBe(true);

    // Icon container has fixed w-6 h-6 grid place-items-center
    const iconContainer = container.querySelector('.w-6.h-6.grid.place-items-center');
    expect(iconContainer).toBeTruthy();

    // Play icon when paused does not have ml-0.5
    const pausedContext = { ...mockAudioContext, isPlaying: false };
    render(
      <MotionProvider>
        <AudioPlayerContext.Provider value={pausedContext}>
          <FloatingPlayerIsland />
        </AudioPlayerContext.Provider>
      </MotionProvider>
    );
    const playBtn = screen.getByRole('button', { name: 'Play music' });
    const playSvg = playBtn.querySelector('svg');
    expect(playSvg?.classList.contains('ml-0.5')).toBe(false);
  });
});

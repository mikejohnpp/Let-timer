import React from 'react';

interface SoundWaveProps {
  isPlaying: boolean;
  className?: string;
}

export const SoundWave: React.FC<SoundWaveProps> = ({ isPlaying, className = '' }) => {
  return (
    <div
      className={`flex items-end gap-[2px] h-3.5 w-3.5 px-0.5 select-none ${className}`}
      aria-hidden="true"
    >
      <span
        className={`w-[2px] rounded-full bg-[var(--accent-text)] transition-all duration-300 ${
          isPlaying ? 'h-3 animate-pulse' : 'h-1 opacity-60'
        }`}
      />
      <span
        className={`w-[2px] rounded-full bg-[var(--accent-text)] transition-all duration-300 ${
          isPlaying ? 'h-3.5 animate-bounce' : 'h-2 opacity-60'
        }`}
        style={{ animationDuration: '0.6s' }}
      />
      <span
        className={`w-[2px] rounded-full bg-[var(--accent-text)] transition-all duration-300 ${
          isPlaying ? 'h-2.5 animate-pulse' : 'h-1.5 opacity-60'
        }`}
        style={{ animationDuration: '0.9s' }}
      />
    </div>
  );
};

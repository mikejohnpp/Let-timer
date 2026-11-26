import React from 'react';

interface SoundWaveProps {
  isPlaying: boolean;
  className?: string;
}

export const SoundWave: React.FC<SoundWaveProps> = ({ isPlaying, className = '' }) => {
  return (
    <div
      className={`flex items-end gap-[2px] h-3.5 w-3.5 px-0.5 select-none ${
        isPlaying ? 'eq-playing' : ''
      } ${className}`}
      aria-hidden="true"
    >
      <span className="w-[2px] h-3 rounded-full bg-[var(--accent-text)] eq-bar-1 origin-bottom" />
      <span className="w-[2px] h-3.5 rounded-full bg-[var(--accent-text)] eq-bar-2 origin-bottom" />
      <span className="w-[2px] h-3 rounded-full bg-[var(--accent-text)] eq-bar-3 origin-bottom" />
    </div>
  );
};

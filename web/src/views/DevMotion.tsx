import React, { useState, useEffect } from 'react';
import { Play, Pause, Sparkles } from 'lucide-react';
import { motion, LayoutGroup } from 'motion/react';
import { readMotionTokens } from '../motion/tokens';
import { useMotionMode, type MotionMode } from '../motion/useMotionMode';
import { useMotionTokens, useMotionTransition } from '../motion';
import { animate } from '../motion/animate';
import { Button, SegmentedControl, useToast } from '../components/ui';

export const DevMotion: React.FC = () => {
  const { motionMode, setMotionMode } = useMotionMode();
  const motionTokens = useMotionTokens();
  const isReduced = motionTokens.isReduced;
  const itemTransition = useMotionTransition('base');
  const [tokens, setTokens] = useState(() => readMotionTokens(true));
  const [isBreathingActive, setIsBreathingActive] = useState(false);
  const [ringPulseCount, setRingPulseCount] = useState(0);
  const [flipItems, setFlipItems] = useState([
    { id: '1', title: 'Deep Work Session', group: 'running' },
    { id: '2', title: 'Review Pull Request', group: 'pending' },
    { id: '3', title: 'Write Spec Documentation', group: 'pending' },
    { id: '4', title: 'Team Sync Call', group: 'completed' },
  ]);
  const [progressBarFill, setProgressBarFill] = useState(0.4);

  const { showToast } = useToast();

  useEffect(() => {
    const handleUpdate = () => {
      setTokens(readMotionTokens(true));
    };
    window.addEventListener('let-timer-motion-changed', handleUpdate);
    return () => window.removeEventListener('let-timer-motion-changed', handleUpdate);
  }, []);

  const triggerRingPulse = () => {
    setRingPulseCount((c) => c + 1);
  };

  const triggerFlipMove = (id: string, targetGroup: string) => {
    setFlipItems((prev) =>
      prev.map((item) => (item.id === id ? { ...item, group: targetGroup } : item))
    );
  };

  const handleTestAnimate = (e: React.MouseEvent<HTMLButtonElement>) => {
    const btn = e.currentTarget;
    animate(
      btn,
      [
        { transform: 'scale(1)', opacity: 1 },
        { transform: 'scale(0.92)', opacity: 0.8 },
        { transform: 'scale(1)', opacity: 1 },
      ],
      { duration: tokens.durBase, easing: tokens.easeOut }
    );
  };

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 py-8 pb-32 flex flex-col gap-8">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-[var(--border)]/30 pb-4">
        <div>
          <div className="flex items-center gap-2">
            <Sparkles className="w-5 h-5 text-[var(--accent-text)]" />
            <h1 className="text-2xl sm:text-3xl font-serif font-bold text-[var(--text)] tracking-tight">
              Motion Gallery
            </h1>
          </div>
          <p className="text-xs text-[var(--muted)] mt-1">
            Reference testing sandbox for calm, minimal, purposeful motion (M0–M5)
          </p>
        </div>

        <div className="flex items-center gap-2">
          <SegmentedControl
            value={motionMode}
            onValueChange={(v) => setMotionMode(v as MotionMode)}
            items={[
              { value: 'system', label: 'System' },
              { value: 'reduced', label: 'Reduced' },
              { value: 'full', label: 'Full' },
            ]}
          />
        </div>
      </div>

      {/* Section 1: Live Motion Tokens */}
      <section className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-xs flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold text-[var(--text)] uppercase tracking-wider">Active Motion Tokens</h2>
          <span className="text-xs font-mono text-[var(--muted)]">
            Mode: <strong className="text-[var(--accent-text)]">{tokens.isOff ? 'off' : tokens.isReduced ? 'reduced' : 'full'}</strong>
          </span>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--dur-instant</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.durInstant}ms</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--dur-fast</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.durFast}ms</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--dur-base</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.durBase}ms</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--dur-slow</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.durSlow}ms</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--travel-s</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.travelS}px</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--travel-m</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.travelM}px</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">--exit-ratio</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.exitRatio}</div>
          </div>
          <div className="p-3 bg-[var(--surface-hover)] rounded-xl">
            <div className="text-xs text-[var(--muted)] uppercase font-semibold">isReduced</div>
            <div className="font-mono text-sm font-bold text-[var(--text)] mt-1">{tokens.isReduced ? 'true' : 'false'}</div>
          </div>
        </div>
      </section>

      {/* Section 2: Layer 1 (Feedback) Micro-interactions */}
      <section className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-xs flex flex-col gap-4">
        <h2 className="text-sm font-bold text-[var(--text)] uppercase tracking-wider">Layer 1: Input Feedback</h2>

        <div className="flex flex-wrap items-center gap-3">
          <Button variant="primary" size="sm" onClick={handleTestAnimate}>
            Scale Tap (A01)
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => showToast('Feedback confirmed (A07)', 'success')}
          >
            Trigger Toast (A07)
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setProgressBarFill((p) => (p >= 0.9 ? 0.2 : p + 0.25))}
          >
            Advance Progress ({Math.round(progressBarFill * 100)}%)
          </Button>
        </div>

        {/* Progress bar demonstration (A15) */}
        <div className="mt-2 flex flex-col gap-1.5">
          <div className="text-xs text-[var(--muted)] font-medium">Daily Progress Bar via scaleX (A15)</div>
          <div className="w-full h-1.5 bg-[var(--border)] rounded-full overflow-hidden">
            <div
              className="h-full bg-[var(--accent-text)] origin-left transition-transform duration-300 ease-out"
              style={{ transform: `scaleX(${progressBarFill})` }}
            />
          </div>
        </div>
      </section>

      {/* Section 3: Layer 2 (Presence) Breathing & Ring Pulses */}
      <section className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-xs flex flex-col gap-4">
        <h2 className="text-sm font-bold text-[var(--text)] uppercase tracking-wider">Layer 2: Presence & Ambience</h2>

        <div className="grid sm:grid-cols-2 gap-4">
          {/* Running Dot Breathing (A12) */}
          <div className="p-4 bg-[var(--surface-hover)] rounded-xl flex items-center justify-between">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Running Dot Breathing (A12)</div>
              <div className="text-xs text-[var(--muted)] mt-0.5">4s period, max 7 iterations then still</div>
            </div>
            <div className="flex items-center gap-3">
              <div className="w-4 h-4 rounded-full border border-[var(--accent-text)] flex items-center justify-center">
                <span className={`w-1.5 h-1.5 rounded-full bg-[var(--accent-text)] ${isBreathingActive ? 'dot-running' : ''}`} />
              </div>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs px-2"
                onClick={() => setIsBreathingActive((a) => !a)}
              >
                {isBreathingActive ? <Pause className="w-3 h-3" /> : <Play className="w-3 h-3" />}
              </Button>
            </div>
          </div>

          {/* Time's Up Ring Pulse (A14) */}
          <div className="p-4 bg-[var(--surface-hover)] rounded-xl flex items-center justify-between">
            <div>
              <div className="text-xs font-semibold text-[var(--text)]">Time's Up Pulse (A14)</div>
              <div className="text-xs text-[var(--muted)] mt-0.5">Two soft expanding rings</div>
            </div>
            <div className="flex items-center gap-3">
              <div className="relative w-6 h-6 flex items-center justify-center">
                <div className="w-3 h-3 rounded-full bg-[var(--timer-over)]" />
                {ringPulseCount > 0 && (
                  <span
                    key={ringPulseCount}
                    className="absolute inset-0 rounded-full border-2 border-[var(--timer-over)] ring-pulse"
                  />
                )}
              </div>
              <Button variant="ghost" size="sm" className="h-7 text-xs px-2" onClick={triggerRingPulse}>
                Pulse
              </Button>
            </div>
          </div>
        </div>
      </section>

      {/* Section 4: Layer 3 (Continuity) Group Transitions */}
      <section className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-xs flex flex-col gap-4">
        <h2 className="text-sm font-bold text-[var(--text)] uppercase tracking-wider">Layer 3: Spatial Continuity (A17/A18)</h2>

        <LayoutGroup id="dev-motion-items">
          <div className="flex flex-col gap-2">
            {flipItems.map((item) => (
              <motion.div
                key={item.id}
                layout={isReduced ? false : 'position'}
                layoutId={`dev-item-${item.id}`}
                transition={itemTransition}
                className="px-3 py-2 bg-[var(--surface-hover)] rounded-lg flex items-center justify-between text-xs select-none"
              >
                <div className="flex items-center gap-2">
                  <span
                    className={`w-2 h-2 rounded-full ${
                      item.group === 'running'
                        ? 'bg-[var(--accent-text)]'
                        : item.group === 'completed'
                        ? 'bg-[var(--success)]'
                        : 'border border-[var(--muted)]'
                    }`}
                  />
                  <span className={item.group === 'completed' ? 'line-through text-[var(--muted)]' : 'text-[var(--text)] font-medium'}>
                    {item.title}
                  </span>
                  <span className="text-xs text-[var(--muted)] px-1.5 py-0.5 rounded bg-[var(--surface)]">
                    {item.group}
                  </span>
                </div>

                <div className="flex items-center gap-1">
                  {item.group !== 'running' && (
                    <button
                      onClick={() => triggerFlipMove(item.id, 'running')}
                      className="px-2 py-1 rounded text-xs bg-[var(--surface)] hover:text-[var(--accent-text)] cursor-pointer"
                    >
                      Run
                    </button>
                  )}
                  {item.group !== 'completed' && (
                    <button
                      onClick={() => triggerFlipMove(item.id, 'completed')}
                      className="px-2 py-1 rounded text-xs bg-[var(--surface)] hover:text-[var(--success-text)] cursor-pointer"
                    >
                      Done
                    </button>
                  )}
                  {item.group === 'completed' && (
                    <button
                      onClick={() => triggerFlipMove(item.id, 'pending')}
                      className="px-2 py-1 rounded text-xs bg-[var(--surface)] hover:text-[var(--text)] cursor-pointer"
                    >
                      Reset
                    </button>
                  )}
                </div>
              </motion.div>
            ))}
          </div>
        </LayoutGroup>
      </section>
    </div>
  );
};

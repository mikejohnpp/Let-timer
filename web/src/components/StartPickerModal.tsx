import React, { useState, useEffect } from 'react';
import { Play, ArrowRight, AlertTriangle } from 'lucide-react';
import type { components } from '../api/schema';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { getStoredToken } from '../api/client';
import {
  Modal,
  Checkbox,
  Banner,
  PriorityMarker,
  Skeleton,
} from './ui';

type Task = components['schemas']['TaskDto'];

interface StartPickerModalProps {
  isOpen: boolean;
  onClose: () => void;
  snapshot: TimerSnapshot | null;
  onStarted: () => void;
}

export const StartPickerModal: React.FC<StartPickerModalProps> = ({
  isOpen,
  onClose,
  snapshot,
  onStarted,
}) => {
  const [candidates, setCandidates] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [switchConfirmed, setSwitchConfirmed] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const hasRunningTask = snapshot && (snapshot.state === 'running' || snapshot.state === 'paused');

  useEffect(() => {
    if (!isOpen) return;

    async function loadCandidates() {
      setIsLoading(true);
      setError(null);
      try {
        const token = getStoredToken();
        const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
        if (token) headers['Authorization'] = `Bearer ${token}`;

        const res = await fetch('/api/v1/tasks/start-candidates', { headers });
        if (res.ok) {
          const data: Task[] = await res.json();
          setCandidates(data);
        } else {
          setError('Failed to load candidate tasks');
        }
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : 'Network error while loading candidates');
      } finally {
        setIsLoading(false);
      }
    }

    loadCandidates();
  }, [isOpen]);

  const handleStartTask = async (taskId: number) => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const url =
        hasRunningTask && switchConfirmed
          ? `/api/v1/tasks/${taskId}/start?switch=true`
          : `/api/v1/tasks/${taskId}/start`;

      const res = await fetch(url, {
        method: 'POST',
        headers,
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        setError(err.message || `Failed to start task #${taskId}`);
      } else {
        onStarted();
        onClose();
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Network error while starting task');
    }
  };

  return (
    <Modal open={isOpen} onOpenChange={(open) => !open && onClose()} title="Start which task?">
      {/* Running task conflict banner */}
      {hasRunningTask && (
        <Banner tone="warning">
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-1.5 font-medium">
              <AlertTriangle className="w-3.5 h-3.5 text-[var(--warning)]" />
              <span>"{snapshot.task.name}" is currently {snapshot.state}.</span>
            </div>
            <Checkbox
              checked={switchConfirmed}
              onCheckedChange={(c) => setSwitchConfirmed(!!c)}
              label="Pause and switch to this new task"
            />
          </div>
        </Banner>
      )}

      {error && <Banner tone="danger">{error}</Banner>}

      {/* Candidates List */}
      <div className="flex flex-col gap-2 -mx-1">
        {isLoading ? (
          <div className="flex flex-col gap-2">
            <Skeleton height="52px" />
            <Skeleton height="52px" />
            <Skeleton height="52px" />
          </div>
        ) : candidates.length === 0 ? (
          <div className="py-8 text-center text-xs text-[var(--muted)]">
            No pending or paused tasks to start.
          </div>
        ) : (
          candidates.map((t) => (
            <div
              key={t.id}
              onClick={() => handleStartTask(t.id)}
              className="p-3 rounded-[var(--radius-control)] border border-[var(--border)] hover:border-[var(--accent-text)] hover:bg-[var(--surface-hover)] transition flex items-center justify-between gap-3 cursor-pointer select-none group"
            >
              <div className="flex items-center gap-2.5 min-w-0">
                <span className="w-6 h-6 rounded-full bg-[var(--surface-hover)] group-hover:bg-[var(--accent)] group-hover:text-[var(--on-accent)] text-[var(--muted)] flex items-center justify-center text-xs transition shrink-0">
                  <Play className="w-3 h-3 fill-current ml-0.5" />
                </span>

                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-xs sm:text-sm text-[var(--text)] truncate">
                      {t.name}
                    </span>
                    <PriorityMarker priority={t.priority} showLabel />
                  </div>
                  <div className="text-[11px] text-[var(--muted)] flex items-center gap-2 mt-0.5">
                    {t.scheduled_on && <span>Scheduled {t.scheduled_on}</span>}
                    {t.estimated_mins && <span>• {t.estimated_mins} min</span>}
                    {t.status === 'paused' && <span className="text-[var(--warning-text)]">• Paused</span>}
                  </div>
                </div>
              </div>

              <ArrowRight className="w-4 h-4 text-[var(--muted)] group-hover:text-[var(--accent-text)] group-hover:translate-x-0.5 transition shrink-0" />
            </div>
          ))
        )}
      </div>
    </Modal>
  );
};

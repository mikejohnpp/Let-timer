import React, { useState, useEffect } from 'react';
import { Play, X, AlertTriangle, ArrowRight } from 'lucide-react';
import type { components } from '../api/schema';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { getStoredToken } from '../api/client';

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
      } catch (e) {
        setError('Network error while loading candidates');
      } finally {
        setIsLoading(false);
      }
    }

    loadCandidates();
  }, [isOpen]);

  if (!isOpen) return null;

  const handleStartTask = async (taskId: number) => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const url = hasRunningTask && switchConfirmed
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
    } catch (e) {
      setError('Network error while starting task');
    }
  };

  const priorityLabels = ['not-yet', 'immediate', 'urgent'];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[80vh]">
        {/* Header */}
        <div className="p-4 border-b border-[var(--border)] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Play className="w-5 h-5 text-[var(--accent)]" />
            <h2 className="text-lg font-bold">Start which task?</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Conflict switch banner */}
        {hasRunningTask && (
          <div className="p-3 bg-amber-500/10 border-b border-amber-500/20 text-xs flex items-center justify-between gap-2 text-amber-500">
            <div className="flex items-center gap-2">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>
                Task <strong>#{snapshot.task.id} {snapshot.task.name}</strong> is currently {snapshot.state}.
              </span>
            </div>
            <label className="flex items-center gap-1.5 cursor-pointer font-medium select-none shrink-0">
              <input
                type="checkbox"
                checked={switchConfirmed}
                onChange={(e) => setSwitchConfirmed(e.target.checked)}
                className="rounded accent-[var(--accent)] cursor-pointer"
              />
              <span>Pause & switch</span>
            </label>
          </div>
        )}

        {error && (
          <div className="p-3 bg-red-500/10 text-red-500 text-xs border-b border-red-500/20">
            {error}
          </div>
        )}

        {/* Candidates list */}
        <div className="p-3 overflow-y-auto divide-y divide-[var(--border)] flex-1">
          {isLoading ? (
            <div className="py-8 text-center text-sm text-[var(--muted)]">Loading candidates…</div>
          ) : candidates.length === 0 ? (
            <div className="py-8 text-center text-sm text-[var(--muted)]">
              No pending or paused tasks found to start.
            </div>
          ) : (
            candidates.map((t) => (
              <div
                key={t.id}
                onClick={() => handleStartTask(t.id)}
                className="p-3 flex items-center justify-between gap-3 hover:bg-[var(--surface-hover)] rounded-xl cursor-pointer transition group"
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm group-hover:text-[var(--accent)] transition truncate">
                      {t.name}
                    </span>
                    <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
                      t.priority === 2 ? 'bg-red-500/20 text-red-500' :
                      t.priority === 1 ? 'bg-amber-500/20 text-amber-500' :
                      'bg-[var(--surface-hover)] text-[var(--muted)]'
                    }`}>
                      {priorityLabels[t.priority] || 'normal'}
                    </span>
                  </div>
                  <div className="text-xs text-[var(--muted)] flex items-center gap-2 mt-0.5">
                    {t.scheduled_on && <span>Scheduled: {t.scheduled_on}</span>}
                    {t.estimated_mins && <span>• {t.estimated_mins} mins</span>}
                    {t.status === 'paused' && <span className="text-amber-500 font-medium">• Paused</span>}
                  </div>
                </div>

                <button
                  type="button"
                  className="p-2 rounded-lg bg-[var(--surface-hover)] group-hover:bg-[var(--accent)] group-hover:text-white transition shrink-0"
                >
                  <ArrowRight className="w-4 h-4" />
                </button>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

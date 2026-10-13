import React, { useState, useEffect, useCallback } from 'react';
import {
  ChevronLeft,
  ChevronRight,
  Plus,
  Play,
  Pause,
  Check,
  Clock,
  Filter,
} from 'lucide-react';
import type { components } from '../api/schema';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { TaskDrawer } from '../components/TaskDrawer';
import { getStoredToken } from '../api/client';

type Task = components['schemas']['TaskDto'];

interface TodayViewProps {
  snapshot: TimerSnapshot | null;
  onRefreshTimer: () => void;
  onOpenStartPicker: () => void;
}

export const TodayView: React.FC<TodayViewProps> = ({
  snapshot,
  onRefreshTimer,
}) => {
  const [selectedDate, setSelectedDate] = useState<string>(() => {
    return new Date().toISOString().slice(0, 10);
  });
  const [tasks, setTasks] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [quickAddText, setQuickAddText] = useState('');
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [filterPriority, setFilterPriority] = useState<string>('all');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [taskToEdit, setTaskToEdit] = useState<Task | null>(null);

  const loadTasks = useCallback(async () => {
    setIsLoading(true);
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks?date=${selectedDate}`, { headers });
      if (res.ok) {
        const data: Task[] = await res.json();
        setTasks(data);
      }
    } catch (e) {
      console.error('Failed to load tasks', e);
    } finally {
      setIsLoading(false);
    }
  }, [selectedDate]);

  useEffect(() => {
    loadTasks();
    const handleWsChange = () => loadTasks();
    window.addEventListener('let-timer-workspace-changed', handleWsChange);
    return () => window.removeEventListener('let-timer-workspace-changed', handleWsChange);
  }, [loadTasks]);

  const handleDateShift = (days: number) => {
    const current = new Date(selectedDate);
    current.setDate(current.getDate() + days);
    setSelectedDate(current.toISOString().slice(0, 10));
  };

  const handleToday = () => {
    setSelectedDate(new Date().toISOString().slice(0, 10));
  };

  const handleQuickAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = quickAddText.trim();
    if (!trimmed) return;

    // Quick regex parsing for "25m" or "50m"
    let estMins: number | null = 25;
    let name = trimmed;
    const match = trimmed.match(/(.*)\s+(\d+)m$/i);
    if (match) {
      name = match[1].trim();
      estMins = parseInt(match[2], 10);
    }

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/tasks', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          name,
          estimated_mins: estMins,
          scheduled_on: selectedDate,
          priority: 0,
        }),
      });

      if (res.ok) {
        setQuickAddText('');
        loadTasks();
      }
    } catch (e) {
      console.error('Quick add failed', e);
    }
  };

  const handleStartTask = async (task: Task) => {
    const isOtherRunning = snapshot && (snapshot.state === 'running' || snapshot.state === 'paused');
    const url = isOtherRunning
      ? `/api/v1/tasks/${task.id}/start?switch=true`
      : `/api/v1/tasks/${task.id}/start`;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(url, { method: 'POST', headers });
      if (res.ok) {
        onRefreshTimer();
        loadTasks();
      } else {
        const err = await res.json().catch(() => ({}));
        alert(err.message || 'Failed to start task');
      }
    } catch (e) {
      console.error('Failed to start task', e);
    }
  };

  const handleQuickLifecycle = async (taskId: number, action: 'pause' | 'resume' | 'done') => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks/${taskId}/${action}`, {
        method: 'POST',
        headers,
      });

      if (res.ok) {
        onRefreshTimer();
        loadTasks();
      }
    } catch (e) {
      console.error(`Failed to ${action} task`, e);
    }
  };

  // Filter tasks
  const filteredTasks = tasks.filter((t) => {
    if (filterStatus !== 'all' && t.status !== filterStatus) return false;
    if (filterPriority !== 'all' && t.priority.toString() !== filterPriority) return false;
    return true;
  });

  const priorityLabels = ['not-yet', 'immediate', 'urgent'];

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
      {/* Date Header & Filters */}
      <div className="flex flex-wrap items-center justify-between gap-4 mb-6">
        {/* Date Pager */}
        <div className="flex items-center gap-2 bg-[var(--surface)] border border-[var(--border)] p-1.5 rounded-xl shadow-sm">
          <button
            onClick={() => handleDateShift(-1)}
            className="p-1.5 rounded-lg text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition"
            aria-label="Previous day"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>

          <span className="font-semibold text-sm px-2 tabular-nums">
            {new Date(selectedDate + 'T00:00:00').toLocaleDateString(undefined, {
              weekday: 'short',
              day: 'numeric',
              month: 'short',
              year: 'numeric',
            })}
          </span>

          <button
            onClick={() => handleDateShift(1)}
            className="p-1.5 rounded-lg text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition"
            aria-label="Next day"
          >
            <ChevronRight className="w-4 h-4" />
          </button>

          <button
            onClick={handleToday}
            className="ml-1 px-2.5 py-1 text-xs font-medium rounded-lg bg-[var(--surface-hover)] text-[var(--text)] hover:bg-[var(--border)] transition"
          >
            Today
          </button>
        </div>

        {/* Filters & Add button */}
        <div className="flex items-center gap-2 ml-auto">
          <div className="flex items-center gap-1.5 bg-[var(--surface)] border border-[var(--border)] px-2.5 py-1.5 rounded-xl text-xs">
            <Filter className="w-3.5 h-3.5 text-[var(--muted)]" />
            <select
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
              className="bg-transparent text-[var(--text)] outline-none cursor-pointer"
            >
              <option value="all" className="bg-[var(--surface)]">All Status</option>
              <option value="pending" className="bg-[var(--surface)]">Pending</option>
              <option value="in-progress" className="bg-[var(--surface)]">Running</option>
              <option value="paused" className="bg-[var(--surface)]">Paused</option>
              <option value="completed" className="bg-[var(--surface)]">Completed</option>
              <option value="cancelled" className="bg-[var(--surface)]">Cancelled</option>
            </select>
          </div>

          <div className="flex items-center gap-1.5 bg-[var(--surface)] border border-[var(--border)] px-2.5 py-1.5 rounded-xl text-xs">
            <select
              value={filterPriority}
              onChange={(e) => setFilterPriority(e.target.value)}
              className="bg-transparent text-[var(--text)] outline-none cursor-pointer"
            >
              <option value="all" className="bg-[var(--surface)]">All Priority</option>
              <option value="2" className="bg-[var(--surface)]">Urgent</option>
              <option value="1" className="bg-[var(--surface)]">Immediate</option>
              <option value="0" className="bg-[var(--surface)]">Not-yet</option>
            </select>
          </div>

          <button
            onClick={() => {
              setTaskToEdit(null);
              setDrawerOpen(true);
            }}
            className="flex items-center gap-1.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-3 py-1.5 rounded-xl text-sm font-semibold shadow-sm transition"
          >
            <Plus className="w-4 h-4" />
            <span>New Task</span>
          </button>
        </div>
      </div>

      {/* Quick Add Form */}
      <form onSubmit={handleQuickAdd} className="mb-6">
        <div className="flex items-center gap-2 bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-2 shadow-sm focus-within:border-[var(--accent)] transition">
          <Plus className="w-5 h-5 text-[var(--muted)] ml-2" />
          <input
            type="text"
            value={quickAddText}
            onChange={(e) => setQuickAddText(e.target.value)}
            placeholder="Quick add: e.g. 'Write essay 45m' (press Enter)"
            className="w-full bg-transparent text-sm text-[var(--text)] outline-none placeholder-[var(--muted)]"
          />
          <button
            type="submit"
            disabled={!quickAddText.trim()}
            className="px-4 py-1.5 rounded-xl bg-[var(--surface-hover)] hover:bg-[var(--accent)] hover:text-white text-xs font-semibold text-[var(--text)] transition disabled:opacity-30"
          >
            Add
          </button>
        </div>
      </form>

      {/* Tasks List */}
      {isLoading ? (
        <div className="py-12 text-center text-[var(--muted)] text-sm">Loading tasks…</div>
      ) : filteredTasks.length === 0 ? (
        <div className="py-16 text-center border-2 border-dashed border-[var(--border)] rounded-2xl p-8 bg-[var(--surface)]/50">
          <Clock className="w-10 h-10 text-[var(--muted)] mx-auto mb-3 opacity-50" />
          <h3 className="text-base font-semibold text-[var(--text)]">No tasks scheduled for this day</h3>
          <p className="text-xs text-[var(--muted)] mt-1">Use the quick add bar above or click "New Task" to plan your schedule.</p>
        </div>
      ) : (
        <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl shadow-sm divide-y divide-[var(--border)] overflow-hidden">
          {filteredTasks.map((t) => {
            const isTaskRunning = snapshot && snapshot.state === 'running' && snapshot.task.id === t.id;
            const isTaskPaused = snapshot && snapshot.state === 'paused' && snapshot.task.id === t.id;

            return (
              <div
                key={t.id}
                className="p-4 flex items-center justify-between gap-4 hover:bg-[var(--surface-hover)] transition group"
              >
                {/* Left info & clickable title */}
                <div
                  className="flex items-center gap-3.5 min-w-0 cursor-pointer flex-1"
                  onClick={() => {
                    setTaskToEdit(t);
                    setDrawerOpen(true);
                  }}
                >
                  {/* Status Indicator Icon */}
                  <div className="shrink-0">
                    {t.status === 'completed' ? (
                      <span className="w-6 h-6 rounded-full bg-[var(--success)]/20 text-[var(--success)] flex items-center justify-center text-xs font-bold">
                        ✓
                      </span>
                    ) : isTaskRunning ? (
                      <span className="w-6 h-6 rounded-full bg-[var(--timer-run)]/20 text-[var(--timer-run)] flex items-center justify-center text-xs animate-pulse">
                        ⏳
                      </span>
                    ) : isTaskPaused ? (
                      <span className="w-6 h-6 rounded-full bg-amber-500/20 text-amber-500 flex items-center justify-center text-xs">
                        ⏸
                      </span>
                    ) : (
                      <span className="w-6 h-6 rounded-full border border-[var(--border)] text-[var(--muted)] flex items-center justify-center text-xs">
                        ○
                      </span>
                    )}
                  </div>

                  {/* Title and details */}
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className={`text-sm font-semibold truncate ${
                        t.status === 'completed' ? 'line-through text-[var(--muted)]' : 'text-[var(--text)]'
                      }`}>
                        {t.name}
                      </span>

                      {/* Priority pill */}
                      <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
                        t.priority === 2 ? 'bg-red-500/10 text-red-500 border border-red-500/20' :
                        t.priority === 1 ? 'bg-amber-500/10 text-amber-500 border border-amber-500/20' :
                        'bg-[var(--surface-hover)] text-[var(--muted)]'
                      }`}>
                        {priorityLabels[t.priority]}
                      </span>
                    </div>

                    <div className="flex items-center gap-2 text-xs text-[var(--muted)] mt-0.5">
                      {t.estimated_mins && <span>{t.estimated_mins}m est</span>}
                      {t.description && <span>• {t.description}</span>}
                      <span className="capitalize">• {t.status}</span>
                    </div>
                  </div>
                </div>

                {/* Right Lifecycle Actions */}
                <div className="flex items-center gap-1.5 shrink-0">
                  {t.status !== 'completed' && t.status !== 'cancelled' && (
                    <>
                      {isTaskRunning ? (
                        <button
                          onClick={() => handleQuickLifecycle(t.id, 'pause')}
                          className="p-2 rounded-xl bg-[var(--surface-hover)] hover:bg-[var(--border)] text-[var(--text)] transition"
                          title="Pause task"
                        >
                          <Pause className="w-4 h-4 fill-current" />
                        </button>
                      ) : isTaskPaused ? (
                        <button
                          onClick={() => handleQuickLifecycle(t.id, 'resume')}
                          className="p-2 rounded-xl bg-[var(--timer-run)] hover:opacity-90 text-white transition shadow-sm"
                          title="Resume task"
                        >
                          <Play className="w-4 h-4 fill-current" />
                        </button>
                      ) : (
                        <button
                          onClick={() => handleStartTask(t)}
                          className="p-2 rounded-xl bg-[var(--surface-hover)] group-hover:bg-[var(--accent)] group-hover:text-white text-[var(--muted)] transition"
                          title="Start task"
                        >
                          <Play className="w-4 h-4 fill-current" />
                        </button>
                      )}

                      <button
                        onClick={() => handleQuickLifecycle(t.id, 'done')}
                        className="p-2 rounded-xl bg-[var(--surface-hover)] hover:bg-[var(--success)] hover:text-white text-[var(--muted)] transition"
                        title="Mark done"
                      >
                        <Check className="w-4 h-4 stroke-[3]" />
                      </button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Task Drawer */}
      <TaskDrawer
        isOpen={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        taskToEdit={taskToEdit}
        onSaved={loadTasks}
        defaultDate={selectedDate}
      />
    </div>
  );
};

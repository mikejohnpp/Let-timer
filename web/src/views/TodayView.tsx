import React, { useState, useEffect, useCallback } from 'react';
import {
  ChevronLeft,
  ChevronRight,
  Plus,
  Play,
  Pause,
  Check,
  Filter,
  MoreHorizontal,
  Square,
  RotateCcw,
  XCircle,
  Trash2,
  Calendar,
  RotateCw,
} from 'lucide-react';
import type { components } from '../api/schema';
import type { TimerSnapshot } from '../hooks/useEventStream';
import { TaskDrawer } from '../components/TaskDrawer';
import { getStoredToken } from '../api/client';
import { toLocalISODate } from '../utils/timer';
import {
  StatusGlyph,
  PriorityMarker,
  Button,
  IconButton,
  Popover,
  PopoverTrigger,
  PopoverContent,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  EmptyState,
  Skeleton,
  useToast,
  useConfirm,
} from '../components/ui';

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
  const [selectedDate, setSelectedDate] = useState<string>(() => toLocalISODate());
  const [tasks, setTasks] = useState<Task[]>([]);
  const [overdueTasks, setOverdueTasks] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [quickAddText, setQuickAddText] = useState('');
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [filterPriority, setFilterPriority] = useState<string>('all');
  const [doneCollapsed, setDoneCollapsed] = useState(true);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [taskToEdit, setTaskToEdit] = useState<Task | null>(null);

  const { showToast } = useToast();
  const confirm = useConfirm();

  const isToday = selectedDate === toLocalISODate();

  const loadTasks = useCallback(async () => {
    setIsLoading(true);
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const [resDate, resOverdue] = await Promise.all([
        fetch(`/api/v1/tasks?date=${selectedDate}`, { headers }),
        isToday ? fetch(`/api/v1/tasks?date=${selectedDate}&overdue=true`, { headers }) : Promise.resolve(null),
      ]);

      if (resDate.ok) {
        const data: Task[] = await resDate.json();
        setTasks(data);
      }
      if (resOverdue && resOverdue.ok) {
        const over: Task[] = await resOverdue.json();
        setOverdueTasks(over);
      } else {
        setOverdueTasks([]);
      }
    } catch (e) {
      console.error('Failed to load tasks', e);
      showToast('Failed to load tasks', 'danger');
    } finally {
      setIsLoading(false);
    }
  }, [selectedDate, isToday, showToast]);

  useEffect(() => {
    loadTasks();
    const handleWsChange = () => loadTasks();
    const handleNewTask = () => {
      setTaskToEdit(null);
      setDrawerOpen(true);
    };
    window.addEventListener('let-timer-workspace-changed', handleWsChange);
    window.addEventListener('let-timer-new-task', handleNewTask);
    return () => {
      window.removeEventListener('let-timer-workspace-changed', handleWsChange);
      window.removeEventListener('let-timer-new-task', handleNewTask);
    };
  }, [loadTasks]);

  const handleDateShift = (days: number) => {
    const parts = selectedDate.split('-').map(Number);
    const current = new Date(parts[0], parts[1] - 1, parts[2]);
    current.setDate(current.getDate() + days);
    setSelectedDate(toLocalISODate(current));
  };

  const handleToday = () => {
    setSelectedDate(toLocalISODate());
  };

  // Quick add parser
  const parseQuickAdd = (raw: string) => {
    const trimmed = raw.trim();
    if (!trimmed) return null;

    let estMins: number | null = 25;
    const defaultStored = localStorage.getItem('let_timer_default_estimate');
    if (defaultStored === 'none') estMins = null;
    else if (defaultStored && !isNaN(Number(defaultStored))) estMins = Number(defaultStored);

    let name = trimmed;
    const match = trimmed.match(/(.*?)\s+(\d+h\d+|\d+h|\d+m)$/i);
    if (match) {
      name = match[1].trim();
      const durStr = match[2].toLowerCase();
      if (durStr === '0m') {
        estMins = null;
      } else if (durStr.includes('h')) {
        const [hPart, mPart] = durStr.split('h');
        const h = parseInt(hPart, 10) || 0;
        const m = parseInt(mPart, 10) || 0;
        estMins = h * 60 + m;
      } else {
        estMins = parseInt(durStr.replace('m', ''), 10);
      }
    }

    return { name, estMins };
  };

  const quickAddParsed = parseQuickAdd(quickAddText);

  const handleQuickAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!quickAddParsed || !quickAddParsed.name) return;

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
          name: quickAddParsed.name,
          estimated_mins: quickAddParsed.estMins,
          scheduled_on: selectedDate,
          priority: 0,
        }),
      });

      if (res.ok) {
        setQuickAddText('');
        showToast(`Added "${quickAddParsed.name}"`, 'success');
        loadTasks();
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to add task', 'danger');
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Quick add failed';
      showToast(msg, 'danger');
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
        showToast(err.message || 'Failed to start task', 'danger');
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Failed to start task';
      showToast(msg, 'danger');
    }
  };

  const handleTaskAction = async (task: Task, action: 'pause' | 'resume' | 'done' | 'stop' | 'restart' | 'cancel' | 'reopen' | 'delete' | 'mark_done_force' | 'move_to_today') => {
    if (action === 'cancel') {
      const ok = await confirm({
        title: `Cancel task "${task.name}"?`,
        tone: 'danger',
        confirmLabel: 'Cancel task',
      });
      if (!ok) return;
    } else if (action === 'delete') {
      const ok = await confirm({
        title: `Delete task "${task.name}"?`,
        body: 'This action cannot be undone.',
        tone: 'danger',
        confirmLabel: 'Delete',
      });
      if (!ok) return;
    } else if (action === 'restart') {
      const ok = await confirm({
        title: `Restart "${task.name}"?`,
        body: 'Restart starts a new run and resets elapsed time.',
        confirmLabel: 'Restart',
      });
      if (!ok) return;
    }

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      let url = `/api/v1/tasks/${task.id}/${action}`;
      let method = 'POST';

      if (action === 'delete') {
        url = `/api/v1/tasks/${task.id}`;
        method = 'DELETE';
      } else if (action === 'mark_done_force') {
        url = `/api/v1/tasks/${task.id}/done?force=true`;
      } else if (action === 'move_to_today') {
        url = `/api/v1/tasks/${task.id}`;
        method = 'PATCH';
        headers['Content-Type'] = 'application/json';
      }

      const res = await fetch(url, {
        method,
        headers,
        body: action === 'move_to_today' ? JSON.stringify({ scheduled_on: toLocalISODate() }) : undefined,
      });

      if (res.ok) {
        onRefreshTimer();
        loadTasks();
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Action failed', 'danger');
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Action failed';
      showToast(msg, 'danger');
    }
  };

  // Filter tasks
  const applyFilter = (list: Task[]) =>
    list.filter((t) => {
      if (filterStatus !== 'all' && t.status !== filterStatus) return false;
      if (filterPriority !== 'all' && t.priority.toString() !== filterPriority) return false;
      return true;
    });

  const allVisible = applyFilter(tasks);
  const runningTasks = allVisible.filter((t) => t.status === 'in-progress');
  const upNextTasks = allVisible.filter((t) => t.status === 'pending' || t.status === 'paused');
  const doneTasks = allVisible.filter((t) => t.status === 'completed' || t.status === 'cancelled');

  const visibleOverdue = applyFilter(overdueTasks);

  const nonCancelled = tasks.filter((t) => t.status !== 'cancelled');
  const completedCount = tasks.filter((t) => t.status === 'completed').length;
  const plannedMins = nonCancelled.reduce((sum, t) => sum + (t.estimated_mins || 0), 0);
  const plannedHours = Math.floor(plannedMins / 60);
  const plannedRemMins = plannedMins % 60;
  const plannedFormatted = plannedHours > 0 ? `${plannedHours}h ${plannedRemMins}m` : `${plannedRemMins}m`;

  const progressPercent = nonCancelled.length > 0 ? Math.round((completedCount / nonCancelled.length) * 100) : 0;

  const renderTaskRow = (t: Task, isOverdueItem: boolean = false) => {
    const isRunning = t.status === 'in-progress';
    const isPaused = t.status === 'paused';
    const isPending = t.status === 'pending';
    const isCompleted = t.status === 'completed';
    const isCancelled = t.status === 'cancelled';

    return (
      <div
        key={t.id}
        className="min-h-[44px] px-3 py-2 flex items-center justify-between gap-3 hover:bg-[var(--surface-hover)] transition group rounded-[var(--radius-control)] select-none"
      >
        {/* Left: status glyph, title, description, priority */}
        <div
          className="flex items-center gap-3 min-w-0 flex-1 cursor-pointer"
          onClick={() => {
            setTaskToEdit(t);
            setDrawerOpen(true);
          }}
        >
          <StatusGlyph status={t.status as 'pending' | 'in-progress' | 'paused' | 'completed' | 'cancelled'} />

          <div className="min-w-0 flex flex-col justify-center">
            <div className="flex items-center gap-2">
              <span
                className={`text-xs sm:text-sm font-medium truncate ${
                  isCompleted || isCancelled ? 'line-through text-[var(--muted)]' : 'text-[var(--text)]'
                }`}
              >
                {t.name}
              </span>
              <PriorityMarker priority={t.priority} />
            </div>

            <div className="flex items-center gap-2 text-[11px] text-[var(--muted)] mt-0.5">
              {isOverdueItem && t.scheduled_on && (
                <span className="text-[var(--warning-text)] font-medium">from {t.scheduled_on}</span>
              )}
              <span>{t.estimated_mins ? `${t.estimated_mins}m est` : 'no est'}</span>
              {t.description && <span className="truncate max-w-xs">• {t.description}</span>}
            </div>
          </div>
        </div>

        {/* Right: Actions (Progressive disclosure: visible on hover or when running/paused) */}
        <div
          className={`flex items-center gap-1 shrink-0 ${
            isRunning || isPaused ? 'opacity-100' : 'opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity'
          }`}
        >
          {/* Status specific actions */}
          {isPending && (
            <IconButton
              variant="ghost"
              size="sm"
              onClick={() => handleStartTask(t)}
              aria-label="Start task"
              title="Start task"
            >
              <Play className="w-3.5 h-3.5 fill-current text-[var(--accent-text)]" />
            </IconButton>
          )}

          {isRunning && (
            <>
              <IconButton
                variant="ghost"
                size="sm"
                onClick={() => handleTaskAction(t, 'pause')}
                aria-label="Pause task"
                title="Pause task"
              >
                <Pause className="w-3.5 h-3.5 fill-current text-[var(--muted)]" />
              </IconButton>
              <IconButton
                variant="ghost"
                size="sm"
                onClick={() => handleTaskAction(t, 'done')}
                aria-label="Mark done"
                title="Mark done"
              >
                <Check className="w-4 h-4 stroke-[3] text-[var(--success-text)]" />
              </IconButton>
            </>
          )}

          {isPaused && (
            <>
              <IconButton
                variant="ghost"
                size="sm"
                onClick={() => handleTaskAction(t, 'resume')}
                aria-label="Resume task"
                title="Resume task"
              >
                <Play className="w-3.5 h-3.5 fill-current text-[var(--accent-text)]" />
              </IconButton>
              <IconButton
                variant="ghost"
                size="sm"
                onClick={() => handleTaskAction(t, 'done')}
                aria-label="Mark done"
                title="Mark done"
              >
                <Check className="w-4 h-4 stroke-[3] text-[var(--success-text)]" />
              </IconButton>
            </>
          )}

          {/* Row Dropdown menu */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <IconButton variant="ghost" size="sm" aria-label="Task options">
                <MoreHorizontal className="w-3.5 h-3.5 text-[var(--muted)]" />
              </IconButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {isOverdueItem && (
                <DropdownMenuItem onClick={() => handleTaskAction(t, 'move_to_today')}>
                  <Calendar className="w-3.5 h-3.5" />
                  <span>Move to today</span>
                </DropdownMenuItem>
              )}

              {isPending && (
                <DropdownMenuItem onClick={() => handleTaskAction(t, 'mark_done_force')}>
                  <Check className="w-3.5 h-3.5" />
                  <span>Mark done without running</span>
                </DropdownMenuItem>
              )}

              {(isRunning || isPaused) && (
                <>
                  <DropdownMenuItem onClick={() => handleTaskAction(t, 'stop')}>
                    <Square className="w-3.5 h-3.5" />
                    <span>Stop</span>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleTaskAction(t, 'restart')}>
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span>Restart</span>
                  </DropdownMenuItem>
                </>
              )}

              {(isCompleted || isCancelled) && (
                <DropdownMenuItem onClick={() => handleTaskAction(t, 'reopen')}>
                  <RotateCw className="w-3.5 h-3.5" />
                  <span>Reopen task</span>
                </DropdownMenuItem>
              )}

              {!isCompleted && !isCancelled && (
                <DropdownMenuItem destructive onClick={() => handleTaskAction(t, 'cancel')}>
                  <XCircle className="w-3.5 h-3.5" />
                  <span>Cancel task</span>
                </DropdownMenuItem>
              )}

              <DropdownMenuSeparator />
              <DropdownMenuItem destructive onClick={() => handleTaskAction(t, 'delete')}>
                <Trash2 className="w-3.5 h-3.5" />
                <span>Delete</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    );
  };

  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 py-8 pb-28">
      {/* Title & Date Bar */}
      <div className="flex items-baseline justify-between gap-4 mb-3">
        <div>
          <h1 className="text-2xl sm:text-3xl font-serif font-bold text-[var(--text)] tracking-tight">
            {isToday
              ? 'Today'
              : new Date(selectedDate + 'T00:00:00').toLocaleDateString(undefined, {
                  weekday: 'long',
                  day: 'numeric',
                  month: 'short',
                })}
          </h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            {completedCount} of {nonCancelled.length} done • {plannedFormatted} planned
          </p>
        </div>

        {/* Minimal Date Navigator */}
        <div className="flex items-center gap-1 text-xs">
          <IconButton variant="ghost" size="sm" onClick={() => handleDateShift(-1)} aria-label="Previous day" className="h-7 w-7 p-0">
            <ChevronLeft className="w-4 h-4 text-[var(--muted)]" />
          </IconButton>
          <button
            onClick={handleToday}
            className="font-mono text-xs font-medium px-2 py-0.5 rounded text-[var(--muted)] hover:text-[var(--text)] transition tabular-nums"
            title="Click to jump to today"
          >
            {new Date(selectedDate + 'T00:00:00').toLocaleDateString(undefined, {
              day: 'numeric',
              month: 'short',
            })}
          </button>
          <IconButton variant="ghost" size="sm" onClick={() => handleDateShift(1)} aria-label="Next day" className="h-7 w-7 p-0">
            <ChevronRight className="w-4 h-4 text-[var(--muted)]" />
          </IconButton>
        </div>
      </div>

      {/* Hairline Progress Line */}
      <div className="mb-6 h-1 w-full bg-[var(--border)] rounded-full overflow-hidden">
        <div
          className="h-full bg-[var(--accent-text)] transition-all duration-300 rounded-full"
          style={{ width: `${progressPercent}%` }}
        />
      </div>

      {/* Zen Quick Add Row */}
      <form onSubmit={handleQuickAdd} className="mb-6">
        <div className="relative border-b border-[var(--border)] focus-within:border-[var(--accent-text)] transition-colors pb-1.5 flex items-center gap-2">
          <Plus className="w-4 h-4 text-[var(--muted)] shrink-0" />
          <input
            type="text"
            value={quickAddText}
            onChange={(e) => setQuickAddText(e.target.value)}
            placeholder='Add a task... (e.g. "Write essay 45m", press Enter)'
            className="flex-1 bg-transparent text-sm text-[var(--text)] outline-none placeholder-[var(--muted)]"
          />
          {quickAddText.trim() && (
            <Button type="submit" variant="ghost" size="sm" className="h-6 text-xs px-2 text-[var(--accent-text)]">
              Add
            </Button>
          )}
        </div>
        {quickAddParsed && quickAddParsed.name && (
          <div className="text-[11px] text-[var(--muted)] pt-1 pl-6">
            Will add: <strong className="text-[var(--text)]">{quickAddParsed.name}</strong> •{' '}
            {quickAddParsed.estMins ? `${quickAddParsed.estMins} min` : 'no estimate'}
          </div>
        )}
      </form>

      {/* Minimal Toolbar */}
      <div className="flex items-center justify-between gap-2 mb-4 text-xs">
        <Popover>
          <PopoverTrigger asChild>
            <button className="flex items-center gap-1.5 text-[var(--muted)] hover:text-[var(--text)] transition cursor-pointer font-medium py-1">
              <Filter className="w-3.5 h-3.5" />
              <span>Filter</span>
              {(filterStatus !== 'all' || filterPriority !== 'all') && (
                <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)]" />
              )}
            </button>
          </PopoverTrigger>
          <PopoverContent align="start" className="w-64 flex flex-col gap-3">
            <div>
              <label className="text-xs font-semibold text-[var(--text)] mb-1.5 block">Status</label>
              <div className="grid grid-cols-2 gap-1 text-xs">
                {['all', 'pending', 'in-progress', 'paused', 'completed', 'cancelled'].map((st) => (
                  <button
                    key={st}
                    onClick={() => setFilterStatus(st)}
                    className={`px-2 py-1 rounded text-left capitalize cursor-pointer ${
                      filterStatus === st
                        ? 'bg-[var(--accent)] text-[var(--on-accent)] font-medium'
                        : 'text-[var(--muted)] hover:bg-[var(--surface-hover)]'
                    }`}
                  >
                    {st === 'in-progress' ? 'Running' : st}
                  </button>
                ))}
              </div>
            </div>
            <div className="border-t border-[var(--border)] pt-2">
              <label className="text-xs font-semibold text-[var(--text)] mb-1.5 block">Priority</label>
              <div className="grid grid-cols-2 gap-1 text-xs">
                {[
                  { id: 'all', label: 'All' },
                  { id: '2', label: 'Urgent' },
                  { id: '1', label: 'Immediate' },
                  { id: '0', label: 'Not-yet' },
                ].map((pr) => (
                  <button
                    key={pr.id}
                    onClick={() => setFilterPriority(pr.id)}
                    className={`px-2 py-1 rounded text-left cursor-pointer ${
                      filterPriority === pr.id
                        ? 'bg-[var(--accent)] text-[var(--on-accent)] font-medium'
                        : 'text-[var(--muted)] hover:bg-[var(--surface-hover)]'
                    }`}
                  >
                    {pr.label}
                  </button>
                ))}
              </div>
            </div>
            {(filterStatus !== 'all' || filterPriority !== 'all') && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setFilterStatus('all');
                  setFilterPriority('all');
                }}
                className="mt-1"
              >
                Reset filters
              </Button>
            )}
          </PopoverContent>
        </Popover>

        <button
          onClick={() => {
            setTaskToEdit(null);
            setDrawerOpen(true);
          }}
          className="flex items-center gap-1 text-[var(--muted)] hover:text-[var(--accent-text)] transition cursor-pointer font-medium"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>New task</span>
        </button>
      </div>

      {/* Zen Tasks Stream */}
      {isLoading ? (
        <div className="flex flex-col gap-2">
          <Skeleton height="40px" />
          <Skeleton height="40px" />
          <Skeleton height="40px" />
        </div>
      ) : allVisible.length === 0 && visibleOverdue.length === 0 ? (
        <EmptyState
          title="No tasks for this day"
          description="Use quick add above or create a task to plan your schedule."
          action={{
            label: 'New task',
            onClick: () => {
              setTaskToEdit(null);
              setDrawerOpen(true);
            },
          }}
        />
      ) : (
        <div className="flex flex-col gap-6">
          {/* Running Group */}
          {runningTasks.length > 0 && (
            <div>
              <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--accent-text)] mb-2 flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)] animate-ping" />
                <span>Now Running</span>
              </div>
              <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--accent)]/30 bg-[var(--accent)]/5 rounded-[var(--radius-control)]">
                {runningTasks.map((t) => renderTaskRow(t))}
              </div>
            </div>
          )}

          {/* Overdue Group */}
          {isToday && visibleOverdue.length > 0 && (
            <div>
              <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--warning-text)] mb-2">
                Overdue ({visibleOverdue.length})
              </div>
              <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--warning)]/30">
                {visibleOverdue.map((t) => renderTaskRow(t, true))}
              </div>
            </div>
          )}

          {/* Up Next Group */}
          {upNextTasks.length > 0 && (
            <div>
              <div className="text-[11px] font-bold uppercase tracking-wider text-[var(--muted)] mb-2">
                Tasks ({upNextTasks.length})
              </div>
              <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--border)]/60">
                {upNextTasks.map((t) => renderTaskRow(t))}
              </div>
            </div>
          )}

          {/* Done Group (collapsible) */}
          {doneTasks.length > 0 && (
            <div className="pt-2">
              <button
                onClick={() => setDoneCollapsed(!doneCollapsed)}
                className="text-[11px] font-bold uppercase tracking-wider text-[var(--muted)] mb-2 flex items-center gap-1.5 cursor-pointer select-none hover:text-[var(--text)] transition"
              >
                <span>Completed ({doneTasks.length})</span>
                <span className="text-[10px] opacity-75">{doneCollapsed ? '▼' : '▲'}</span>
              </button>
              {!doneCollapsed && (
                <div className="flex flex-col divide-y divide-[var(--border)]/30 border-y border-[var(--border)]/40 opacity-70">
                  {doneTasks.map((t) => renderTaskRow(t))}
                </div>
              )}
            </div>
          )}
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

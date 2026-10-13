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
  ProgressLine,
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

        {/* Right: Actions */}
        <div className="flex items-center gap-1 shrink-0">
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
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 pb-24">
      {/* Title & Date Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-4">
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
          <div className="mt-2 w-48 max-w-full">
            <ProgressLine value={progressPercent} />
          </div>
        </div>

        {/* Date Pager */}
        <div className="flex items-center gap-1.5 self-start sm:self-center bg-[var(--surface)] border border-[var(--border)] p-1 rounded-[var(--radius-control)] shadow-xs">
          <IconButton variant="ghost" size="sm" onClick={() => handleDateShift(-1)} aria-label="Previous day">
            <ChevronLeft className="w-4 h-4" />
          </IconButton>
          <span className="font-mono text-xs font-semibold px-2 tabular-nums">
            {new Date(selectedDate + 'T00:00:00').toLocaleDateString(undefined, {
              weekday: 'short',
              day: 'numeric',
              month: 'short',
              year: 'numeric',
            })}
          </span>
          <IconButton variant="ghost" size="sm" onClick={() => handleDateShift(1)} aria-label="Next day">
            <ChevronRight className="w-4 h-4" />
          </IconButton>
          <Button variant="secondary" size="sm" onClick={handleToday} className="ml-1 h-7 text-[11px] px-2">
            Today
          </Button>
        </div>
      </div>

      {/* Quick Add Row */}
      <form onSubmit={handleQuickAdd} className="mb-5">
        <div className="flex flex-col gap-1.5 bg-[var(--surface)] border border-[var(--border-strong)] rounded-[var(--radius-card)] p-2 shadow-xs transition focus-within:border-[var(--accent-text)] focus-within:ring-2 focus-within:ring-[var(--accent-text)]/20">
          <div className="flex items-center gap-2">
            <Plus className="w-4 h-4 text-[var(--muted)] ml-1" />
            <input
              type="text"
              value={quickAddText}
              onChange={(e) => setQuickAddText(e.target.value)}
              placeholder='Quick add: "Write essay 45m" (0m = no estimate, press Enter)'
              className="flex-1 bg-transparent text-xs sm:text-sm text-[var(--text)] outline-none placeholder-[var(--muted)]"
            />
            <Button type="submit" variant="ghost" size="sm" disabled={!quickAddText.trim()}>
              Add
            </Button>
          </div>
          {quickAddParsed && quickAddParsed.name && (
            <div className="text-[11px] text-[var(--muted)] pl-6 pb-0.5">
              Will add: <strong className="text-[var(--text)]">{quickAddParsed.name}</strong> •{' '}
              {quickAddParsed.estMins ? `${quickAddParsed.estMins} min` : 'no estimate'} • {selectedDate}
            </div>
          )}
        </div>
      </form>

      {/* Toolbar: Filter popover & New task button */}
      <div className="flex items-center justify-between gap-3 mb-4">
        <div className="flex items-center gap-2">
          <Popover>
            <PopoverTrigger asChild>
              <Button variant="secondary" size="sm" className="relative">
                <Filter className="w-3.5 h-3.5 text-[var(--muted)]" />
                <span>Filter</span>
                {(filterStatus !== 'all' || filterPriority !== 'all') && (
                  <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)]" />
                )}
              </Button>
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
                        filterStatus === st ? 'bg-[var(--accent)] text-[var(--on-accent)] font-medium' : 'text-[var(--muted)] hover:bg-[var(--surface-hover)]'
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
                        filterPriority === pr.id ? 'bg-[var(--accent)] text-[var(--on-accent)] font-medium' : 'text-[var(--muted)] hover:bg-[var(--surface-hover)]'
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
        </div>

        <Button
          variant="primary"
          size="sm"
          onClick={() => {
            setTaskToEdit(null);
            setDrawerOpen(true);
          }}
        >
          <Plus className="w-3.5 h-3.5 stroke-[3]" />
          <span>New task</span>
        </Button>
      </div>

      {/* Groups List */}
      {isLoading ? (
        <div className="flex flex-col gap-2">
          <Skeleton height="44px" />
          <Skeleton height="44px" />
          <Skeleton height="44px" />
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
              <h2 className="text-xs font-semibold text-[var(--accent-text)] mb-1.5 flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)] animate-ping" />
                <span>Running</span>
              </h2>
              <div className="bg-[var(--surface)] border border-[var(--accent)]/30 rounded-[var(--radius-card)] divide-y divide-[var(--border)] overflow-hidden shadow-xs">
                {runningTasks.map((t) => renderTaskRow(t))}
              </div>
            </div>
          )}

          {/* Overdue Group (only shown when viewing today and overdue tasks exist) */}
          {isToday && visibleOverdue.length > 0 && (
            <div>
              <h2 className="text-xs font-semibold text-[var(--warning-text)] mb-1.5 flex items-center gap-1">
                <span>Overdue ({visibleOverdue.length})</span>
              </h2>
              <div className="bg-[var(--surface)] border border-[var(--warning)]/30 rounded-[var(--radius-card)] divide-y divide-[var(--border)] overflow-hidden shadow-xs">
                {visibleOverdue.map((t) => renderTaskRow(t, true))}
              </div>
            </div>
          )}

          {/* Up Next Group */}
          {upNextTasks.length > 0 && (
            <div>
              <h2 className="text-xs font-semibold text-[var(--muted)] mb-1.5">Up next ({upNextTasks.length})</h2>
              <div className="bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-card)] divide-y divide-[var(--border)] overflow-hidden shadow-xs">
                {upNextTasks.map((t) => renderTaskRow(t))}
              </div>
            </div>
          )}

          {/* Done Group (collapsible) */}
          {doneTasks.length > 0 && (
            <div>
              <button
                onClick={() => setDoneCollapsed(!doneCollapsed)}
                className="text-xs font-semibold text-[var(--muted)] mb-1.5 flex items-center gap-1 cursor-pointer select-none hover:text-[var(--text)] transition"
              >
                <span>Done ({doneTasks.length})</span>
                <span className="text-[10px]">{doneCollapsed ? '▼' : '▲'}</span>
              </button>
              {!doneCollapsed && (
                <div className="bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-card)] divide-y divide-[var(--border)] overflow-hidden opacity-75 shadow-xs">
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

import React, { useState, useEffect } from 'react';
import { X, Trash2, Calendar, Clock, AlertCircle } from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredToken } from '../api/client';

type Task = components['schemas']['TaskDto'];
type MediaList = components['schemas']['MediaListDto'];

interface TaskDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  taskToEdit: Task | null;
  onSaved: () => void;
  defaultDate?: string;
}

export const TaskDrawer: React.FC<TaskDrawerProps> = ({
  isOpen,
  onClose,
  taskToEdit,
  onSaved,
  defaultDate,
}) => {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [estimatedMins, setEstimatedMins] = useState<string>('');
  const [scheduledOn, setScheduledOn] = useState<string>('');
  const [priority, setPriority] = useState<number>(0);
  const [mediaListId, setMediaListId] = useState<string>('');
  const [mediaLists, setMediaLists] = useState<MediaList[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;

    if (taskToEdit) {
      setName(taskToEdit.name);
      setDescription(taskToEdit.description || '');
      setEstimatedMins(taskToEdit.estimated_mins != null ? taskToEdit.estimated_mins.toString() : '');
      setScheduledOn(taskToEdit.scheduled_on || '');
      setPriority(taskToEdit.priority);
      setMediaListId(taskToEdit.media_list_id != null ? taskToEdit.media_list_id.toString() : '');
    } else {
      setName('');
      setDescription('');
      setEstimatedMins('25');
      setScheduledOn(defaultDate || new Date().toISOString().slice(0, 10));
      setPriority(0);
      setMediaListId('');
    }

    async function loadMediaLists() {
      try {
        const token = getStoredToken();
        const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
        if (token) headers['Authorization'] = `Bearer ${token}`;

        const res = await fetch('/api/v1/media-lists', { headers });
        if (res.ok) {
          const data: MediaList[] = await res.json();
          setMediaLists(data);
        }
      } catch (e) {
        console.error('Failed to load media lists', e);
      }
    }

    loadMediaLists();
  }, [isOpen, taskToEdit, defaultDate]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError('Task name is required');
      return;
    }

    setIsSubmitting(true);
    setError(null);

    const estMins = estimatedMins.trim() ? parseInt(estimatedMins.trim(), 10) : null;
    const mlId = mediaListId.trim() ? parseInt(mediaListId.trim(), 10) : null;
    const dateVal = scheduledOn.trim() || null;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      let res: Response;
      if (taskToEdit) {
        res = await fetch(`/api/v1/tasks/${taskToEdit.id}`, {
          method: 'PATCH',
          headers,
          body: JSON.stringify({
            name: name.trim(),
            description: description.trim() || null,
            estimated_mins: estMins,
            scheduled_on: dateVal,
            priority,
            media_list_id: mlId,
          }),
        });
      } else {
        res = await fetch('/api/v1/tasks', {
          method: 'POST',
          headers,
          body: JSON.stringify({
            name: name.trim(),
            description: description.trim() || null,
            estimated_mins: estMins,
            scheduled_on: dateVal,
            priority,
            media_list_id: mlId,
          }),
        });
      }

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Failed to save task');
      } else {
        onSaved();
        onClose();
      }
    } catch (e) {
      setError('Network error while saving task');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!taskToEdit) return;
    if (!confirm(`Delete task #${taskToEdit.id} "${taskToEdit.name}"?`)) return;

    setIsSubmitting(true);
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/tasks/${taskToEdit.id}`, {
        method: 'DELETE',
        headers,
      });

      if (res.ok) {
        onSaved();
        onClose();
      } else {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Failed to delete task');
      }
    } catch (e) {
      setError('Network error while deleting task');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-end bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
      <div className="bg-[var(--surface)] border-l border-[var(--border)] w-full max-w-md h-full shadow-2xl flex flex-col overflow-y-auto">
        {/* Drawer Header */}
        <div className="p-4 border-b border-[var(--border)] flex items-center justify-between">
          <h2 className="text-lg font-bold">
            {taskToEdit ? `Edit Task #${taskToEdit.id}` : 'Create Task'}
          </h2>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {error && (
          <div className="p-3 bg-red-500/10 text-red-500 text-xs border-b border-red-500/20 flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-5 flex-1 flex flex-col gap-4">
          <div>
            <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-1.5">
              Task Name *
            </label>
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Read chapter 3"
              className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-[var(--text)] text-sm focus:border-[var(--accent)] outline-none transition"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-1.5 flex items-center gap-1">
                <Clock className="w-3.5 h-3.5" />
                <span>Estimate (mins)</span>
              </label>
              <input
                type="number"
                min="1"
                value={estimatedMins}
                onChange={(e) => setEstimatedMins(e.target.value)}
                placeholder="None"
                className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-[var(--text)] text-sm focus:border-[var(--accent)] outline-none transition"
              />
              <span className="text-[10px] text-[var(--muted)]">Leave empty for no estimate</span>
            </div>

            <div>
              <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-1.5 flex items-center gap-1">
                <Calendar className="w-3.5 h-3.5" />
                <span>Scheduled Date</span>
              </label>
              <input
                type="date"
                value={scheduledOn}
                onChange={(e) => setScheduledOn(e.target.value)}
                className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-[var(--text)] text-sm focus:border-[var(--accent)] outline-none transition"
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-1.5">
              Priority
            </label>
            <div className="grid grid-cols-3 gap-2">
              {[
                { val: 0, label: 'Not-yet' },
                { val: 1, label: 'Immediate' },
                { val: 2, label: 'Urgent' },
              ].map((p) => (
                <button
                  type="button"
                  key={p.val}
                  onClick={() => setPriority(p.val)}
                  className={`py-2 text-xs font-medium rounded-xl border transition ${
                    priority === p.val
                      ? 'border-[var(--accent)] bg-[var(--accent)] text-white shadow-sm'
                      : 'border-[var(--border)] bg-[var(--surface-hover)] text-[var(--muted)] hover:text-[var(--text)]'
                  }`}
                >
                  {p.label}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-1.5">
              Focus Media List
            </label>
            <select
              value={mediaListId}
              onChange={(e) => setMediaListId(e.target.value)}
              className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-[var(--text)] text-sm focus:border-[var(--accent)] outline-none transition"
            >
              <option value="">None</option>
              {mediaLists.map((ml) => (
                <option key={ml.id} value={ml.id.toString()}>
                  {ml.name}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-1.5">
              Description / Notes
            </label>
            <textarea
              rows={4}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Add details, links, or notes…"
              className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-[var(--text)] text-sm focus:border-[var(--accent)] outline-none transition resize-none"
            />
          </div>

          {/* Form Actions */}
          <div className="mt-auto pt-4 border-t border-[var(--border)] flex items-center justify-between gap-3">
            {taskToEdit ? (
              <button
                type="button"
                onClick={handleDelete}
                disabled={isSubmitting}
                className="flex items-center gap-1.5 text-red-500 hover:text-red-400 p-2 text-sm font-medium rounded-xl hover:bg-red-500/10 transition"
              >
                <Trash2 className="w-4 h-4" />
                <span>Delete</span>
              </button>
            ) : <div />}

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-sm font-medium text-[var(--muted)] hover:text-[var(--text)] rounded-xl transition"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-5 py-2 text-sm font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white rounded-xl shadow-sm transition disabled:opacity-50"
              >
                {isSubmitting ? 'Saving…' : 'Save'}
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};

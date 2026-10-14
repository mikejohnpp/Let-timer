import React, { useState, useEffect } from 'react';
import { Trash2 } from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredWorkspace, apiFetch } from '../api/client';
import { toLocalISODate } from '../utils/timer';
import {
  Drawer,
  Button,
  FormField,
  Input,
  Textarea,
  Select,
  SegmentedControl,
  Banner,
  useToast,
  useConfirm,
} from './ui';

type Task = components['schemas']['TaskDto'];
type MediaList = components['schemas']['MediaListDto'];

interface TaskDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  taskToEdit: Task | null;
  onSaved: (task?: Task) => void;
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

  const { showToast } = useToast();
  const confirm = useConfirm();

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
      const defaultStored = localStorage.getItem('let_timer_default_estimate');
      const defEst = defaultStored === 'none' ? '' : defaultStored && !isNaN(Number(defaultStored)) ? defaultStored : '25';
      setEstimatedMins(defEst);
      setScheduledOn(defaultDate || toLocalISODate());
      setPriority(0);
      setMediaListId('');
    }

    async function loadMediaLists() {
      try {
        const res = await apiFetch('/api/v1/media-lists');
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
      let res: globalThis.Response;
      if (taskToEdit) {
        res = await apiFetch(`/api/v1/tasks/${taskToEdit.id}`, {
          method: 'PATCH',
          headers: {
            'Content-Type': 'application/json',
          },
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
        const currentWsId = getStoredWorkspace();
        res = await apiFetch('/api/v1/tasks', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            name: name.trim(),
            description: description.trim() || null,
            estimated_mins: estMins,
            scheduled_on: dateVal,
            priority,
            media_list_id: mlId,
            workspace_id: currentWsId ? Number(currentWsId) : undefined,
          }),
        });
      }

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Failed to save task');
      } else {
        const savedTask: Task | null = await res.json().catch(() => null);
        showToast(taskToEdit ? 'Task updated' : 'Task created', 'success');
        onSaved(savedTask ?? undefined);
        onClose();
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Network error while saving task');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!taskToEdit) return;
    const ok = await confirm({
      title: `Delete task #${taskToEdit.id} "${taskToEdit.name}"?`,
      body: 'This task and its sessions will be deleted permanently.',
      tone: 'danger',
      confirmLabel: 'Delete task',
    });
    if (!ok) return;

    setIsSubmitting(true);
    try {
      const res = await apiFetch(`/api/v1/tasks/${taskToEdit.id}`, {
        method: 'DELETE',
      });

      if (res.ok) {
        showToast('Task deleted', 'success');
        onSaved();
        onClose();
      } else {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Failed to delete task');
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Network error while deleting task');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Drawer
      open={isOpen}
      onOpenChange={(open) => !open && onClose()}
      title={taskToEdit ? `Edit task #${taskToEdit.id}` : 'Create task'}
    >
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        {error && <Banner tone="danger">{error}</Banner>}

        <FormField label="Name" required>
          {(id) => (
            <Input
              id={id}
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Read chapter 3"
            />
          )}
        </FormField>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <FormField label="Estimate (min)" helperText="Leave empty for no estimate">
            {(id) => (
              <Input
                id={id}
                type="number"
                min="1"
                value={estimatedMins}
                onChange={(e) => setEstimatedMins(e.target.value)}
                placeholder="25"
              />
            )}
          </FormField>

          <FormField label="Scheduled date">
            {(id) => (
              <Input
                id={id}
                type="date"
                value={scheduledOn}
                onChange={(e) => setScheduledOn(e.target.value)}
              />
            )}
          </FormField>
        </div>

        <FormField label="Priority">
          {() => (
            <SegmentedControl
              value={priority.toString()}
              onValueChange={(val) => setPriority(parseInt(val, 10))}
              items={[
                { value: '0', label: 'Not-yet' },
                { value: '1', label: 'Immediate' },
                { value: '2', label: 'Urgent' },
              ]}
            />
          )}
        </FormField>

        <FormField label="Focus media list" helperText="Used as the background in Focus mode">
          {(id) => (
            <Select
              id={id}
              value={mediaListId}
              onChange={(e) => setMediaListId(e.target.value)}
            >
              <option value="">None</option>
              {mediaLists.map((ml) => (
                <option key={ml.id} value={ml.id.toString()}>
                  {ml.name}
                </option>
              ))}
            </Select>
          )}
        </FormField>

        <FormField label="Notes">
          {(id) => (
            <Textarea
              id={id}
              rows={4}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional notes or subtasks..."
            />
          )}
        </FormField>

        <div className="pt-4 border-t border-[var(--border)] flex items-center justify-between gap-3">
          {taskToEdit ? (
            <Button
              type="button"
              variant="danger"
              size="sm"
              onClick={handleDelete}
              disabled={isSubmitting}
            >
              <Trash2 className="w-3.5 h-3.5" />
              <span>Delete</span>
            </Button>
          ) : (
            <div />
          )}

          <div className="flex items-center gap-2">
            <Button type="button" variant="ghost" size="sm" onClick={onClose} disabled={isSubmitting}>
              Cancel
            </Button>
            <Button type="submit" variant="primary" size="sm" loading={isSubmitting}>
              Save
            </Button>
          </div>
        </div>
      </form>
    </Drawer>
  );
};

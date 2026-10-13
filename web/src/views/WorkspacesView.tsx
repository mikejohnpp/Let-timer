import React, { useState, useEffect } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredWorkspace, setStoredWorkspace, apiFetch } from '../api/client';
import {
  Button,
  IconButton,
  Banner,
  FormField,
  Input,
  Skeleton,
  EmptyState,
  useToast,
  useConfirm,
} from '../components/ui';

type Workspace = components['schemas']['WorkspaceDto'];

export const WorkspacesView: React.FC = () => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWsId, setActiveWsId] = useState<string>(getStoredWorkspace() || '');
  const [newWsName, setNewWsName] = useState('');
  const [newWsDesc, setNewWsDesc] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { showToast } = useToast();
  const confirm = useConfirm();

  const loadWorkspaces = async () => {
    setIsLoading(true);
    try {
      const res = await apiFetch('/api/v1/workspaces');
      if (res.ok) {
        const data: Workspace[] = await res.json();
        setWorkspaces(data);
      }
    } catch (e) {
      console.error('Failed to load workspaces', e);
      showToast('Failed to load workspaces', 'danger');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadWorkspaces();
  }, []);

  const handleSelectWorkspace = async (id: number) => {
    const idStr = id.toString();
    setActiveWsId(idStr);
    setStoredWorkspace(idStr);
    try {
      await apiFetch('/api/v1/workspaces/switch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id }),
      });
    } catch (e) {
      console.error('Failed to sync switch with daemon', e);
    }
    window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
    showToast('Workspace switched', 'info');
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newWsName.trim()) return;

    setError(null);
    try {
      const res = await apiFetch('/api/v1/workspaces', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: newWsName.trim(),
          description: newWsDesc.trim() || null,
        }),
      });

      if (res.ok) {
        setNewWsName('');
        setNewWsDesc('');
        setIsCreating(false);
        showToast('Workspace created', 'success');
        loadWorkspaces();
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
      } else {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Failed to create workspace');
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Network error creating workspace');
    }
  };

  const handleDelete = async (id: number, name: string) => {
    const ok = await confirm({
      title: `Delete workspace "${name}"?`,
      body: 'All tasks within this workspace will be deleted permanently. You cannot delete the active workspace.',
      tone: 'danger',
      confirmLabel: 'Delete workspace',
    });
    if (!ok) return;

    try {
      const res = await apiFetch(`/api/v1/workspaces/${id}`, {
        method: 'DELETE',
      });

      if (res.ok) {
        showToast('Workspace deleted', 'success');
        loadWorkspaces();
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to delete workspace', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to delete workspace', 'danger');
    }
  };

  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 py-8 pb-28">
      {/* Header */}
      <div className="flex items-baseline justify-between mb-6">
        <div>
          <h1 className="text-2xl sm:text-3xl font-serif font-bold text-[var(--text)] tracking-tight">Workspaces</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Active workspace scopes your tasks and timer.
          </p>
        </div>

        <button
          onClick={() => setIsCreating(!isCreating)}
          className="flex items-center gap-1 text-xs text-[var(--muted)] hover:text-[var(--accent-text)] transition cursor-pointer font-medium"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{isCreating ? 'Cancel' : 'New workspace'}</span>
        </button>
      </div>

      {/* Inline Create Form */}
      {isCreating && (
        <div className="mb-6 p-4 border border-[var(--border)] rounded-[var(--radius-control)] bg-[var(--surface-hover)]/40">
          <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--muted)] mb-3">Add Workspace</h2>
          {error && <Banner tone="danger" className="mb-3">{error}</Banner>}

          <form onSubmit={handleCreate} className="flex flex-col gap-3 text-xs">
            <FormField label="Workspace name" required>
              {(id) => (
                <Input
                  id={id}
                  required
                  value={newWsName}
                  onChange={(e) => setNewWsName(e.target.value)}
                  placeholder="e.g. Study, Work, Personal"
                />
              )}
            </FormField>

            <FormField label="Description (optional)">
              {(id) => (
                <Input
                  id={id}
                  value={newWsDesc}
                  onChange={(e) => setNewWsDesc(e.target.value)}
                  placeholder="e.g. University exam prep"
                />
              )}
            </FormField>

            <div className="flex justify-end gap-2 mt-1">
              <Button type="button" variant="ghost" size="sm" onClick={() => setIsCreating(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" size="sm">
                Create
              </Button>
            </div>
          </form>
        </div>
      )}

      {/* Workspaces List */}
      {isLoading ? (
        <div className="flex flex-col gap-2">
          <Skeleton height="44px" />
          <Skeleton height="44px" />
        </div>
      ) : workspaces.length === 0 ? (
        <EmptyState
          title="Create your first workspace"
          description="Workspaces help you group and scope tasks into projects."
          action={{
            label: 'New workspace',
            onClick: () => setIsCreating(true),
          }}
        />
      ) : (
        <div className="flex flex-col divide-y divide-[var(--border)]/40 border-y border-[var(--border)]/60">
          {workspaces.map((w) => {
            const isActive = activeWsId === w.id.toString();

            return (
              <div
                key={w.id}
                className={`py-3 px-2 flex items-center justify-between gap-3 hover:bg-[var(--surface-hover)]/60 transition group cursor-pointer ${
                  isActive ? 'bg-[var(--accent)]/5' : ''
                }`}
                onClick={() => !isActive && handleSelectWorkspace(w.id)}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <span
                    className={`w-2.5 h-2.5 rounded-full shrink-0 transition ${
                      isActive ? 'bg-[var(--accent-text)] ring-4 ring-[var(--accent-text)]/20' : 'border border-[var(--muted)]'
                    }`}
                  />

                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className={`text-xs sm:text-sm font-semibold truncate ${isActive ? 'text-[var(--accent-text)]' : 'text-[var(--text)]'}`}>
                        {w.name}
                      </span>
                      {isActive && (
                        <span className="text-[10px] font-bold text-[var(--accent-text)] bg-[var(--accent)]/15 px-1.5 py-0.2 rounded-full">
                          Active
                        </span>
                      )}
                    </div>
                    {w.description && (
                      <p className="text-[11px] text-[var(--muted)] mt-0.5 truncate max-w-md">
                        {w.description}
                      </p>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity" onClick={(e) => e.stopPropagation()}>
                  {!isActive && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleSelectWorkspace(w.id)}
                      className="text-xs h-6 px-2 text-[var(--muted)] hover:text-[var(--text)]"
                    >
                      Switch
                    </Button>
                  )}

                  {!isActive && (
                    <IconButton
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 text-[var(--muted)] hover:text-[var(--danger-text)]"
                      onClick={() => handleDelete(w.id, w.name)}
                      aria-label={`Delete workspace ${w.name}`}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </IconButton>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

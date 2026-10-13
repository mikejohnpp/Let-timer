import React, { useState, useEffect } from 'react';
import { Folder, Plus, Trash2, Check } from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredToken, getStoredWorkspace, setStoredWorkspace } from '../api/client';
import {
  Button,
  IconButton,
  Badge,
  Banner,
  Card,
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
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/workspaces', { headers });
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

  const handleSelectWorkspace = (id: number) => {
    const idStr = id.toString();
    setActiveWsId(idStr);
    setStoredWorkspace(idStr);
    window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
    showToast('Workspace switched', 'info');
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newWsName.trim()) return;

    setError(null);
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/workspaces', {
        method: 'POST',
        headers,
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
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/workspaces/${id}`, {
        method: 'DELETE',
        headers,
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
    <div className="max-w-4xl mx-auto px-4 sm:px-6 py-6 pb-24">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text)] tracking-tight">Workspaces</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Active workspace scopes your tasks and timer.
          </p>
        </div>

        <Button
          variant="primary"
          size="sm"
          onClick={() => setIsCreating(!isCreating)}
        >
          <Plus className="w-3.5 h-3.5 stroke-[3]" />
          <span>New workspace</span>
        </Button>
      </div>

      {/* Inline Create Form */}
      {isCreating && (
        <Card className="mb-6">
          <h2 className="text-sm font-bold text-[var(--text)] mb-3">Add workspace</h2>
          {error && <Banner tone="danger" className="mb-3">{error}</Banner>}

          <form onSubmit={handleCreate} className="flex flex-col gap-3">
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

            <div className="flex justify-end gap-2 mt-2">
              <Button type="button" variant="ghost" size="sm" onClick={() => setIsCreating(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" size="sm">
                Create
              </Button>
            </div>
          </form>
        </Card>
      )}

      {/* Workspaces List */}
      {isLoading ? (
        <div className="flex flex-col gap-2">
          <Skeleton height="56px" />
          <Skeleton height="56px" />
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
        <div className="bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-card)] divide-y divide-[var(--border)] overflow-hidden shadow-xs">
          {workspaces.map((w) => {
            const isActive = activeWsId === w.id.toString();

            return (
              <div
                key={w.id}
                className="p-3.5 flex items-center justify-between gap-3 hover:bg-[var(--surface-hover)] transition group cursor-pointer"
                onClick={() => !isActive && handleSelectWorkspace(w.id)}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div
                    className={`w-9 h-9 rounded-[var(--radius-control)] flex items-center justify-center shrink-0 transition ${
                      isActive
                        ? 'bg-[var(--accent)] text-[var(--on-accent)]'
                        : 'bg-[var(--surface-hover)] text-[var(--muted)] group-hover:text-[var(--text)]'
                    }`}
                  >
                    <Folder className="w-4 h-4" />
                  </div>

                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-xs sm:text-sm text-[var(--text)] truncate">
                        {w.name}
                      </span>
                      {isActive && <Badge variant="accent">Active</Badge>}
                    </div>
                    {w.description && (
                      <p className="text-[11px] text-[var(--muted)] mt-0.5 truncate max-w-md">
                        {w.description}
                      </p>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0" onClick={(e) => e.stopPropagation()}>
                  {!isActive && (
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => handleSelectWorkspace(w.id)}
                      className="text-xs h-7"
                    >
                      <Check className="w-3 h-3" />
                      <span className="hidden sm:inline">Switch</span>
                    </Button>
                  )}

                  {!isActive && (
                    <IconButton
                      variant="danger"
                      size="sm"
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

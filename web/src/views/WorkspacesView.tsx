import React, { useState, useEffect } from 'react';
import { Folder, Plus, Trash2, Check } from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredToken, getStoredWorkspace, setStoredWorkspace } from '../api/client';

type Workspace = components['schemas']['WorkspaceDto'];

export const WorkspacesView: React.FC = () => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWsId, setActiveWsId] = useState<string>(getStoredWorkspace() || '');
  const [newWsName, setNewWsName] = useState('');
  const [newWsDesc, setNewWsDesc] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadWorkspaces = async () => {
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
        loadWorkspaces();
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
      } else {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Failed to create workspace');
      }
    } catch (e) {
      setError('Network error creating workspace');
    }
  };

  const handleDelete = async (id: number, name: string) => {
    if (!confirm(`Delete workspace "${name}" and all its tasks?`)) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/workspaces/${id}`, {
        method: 'DELETE',
        headers,
      });

      if (res.ok) {
        loadWorkspaces();
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
      } else {
        const err = await res.json().catch(() => ({}));
        alert(err.message || 'Failed to delete workspace');
      }
    } catch (e) {
      console.error('Delete workspace failed', e);
    }
  };

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 py-8">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text)]">Workspaces</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Group your tasks into dedicated contexts (e.g. Study, Work, Home).
          </p>
        </div>

        <button
          onClick={() => setIsCreating(true)}
          className="flex items-center gap-1.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-3.5 py-2 rounded-xl text-sm font-semibold shadow-sm transition"
        >
          <Plus className="w-4 h-4" />
          <span>New Workspace</span>
        </button>
      </div>

      {isCreating && (
        <form onSubmit={handleCreate} className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 mb-6 shadow-sm">
          <h2 className="text-sm font-bold mb-3">Add New Workspace</h2>
          {error && <div className="text-xs text-red-500 mb-3">{error}</div>}

          <div className="grid sm:grid-cols-2 gap-3 mb-4">
            <input
              type="text"
              required
              placeholder="Workspace name (e.g. Study)"
              value={newWsName}
              onChange={(e) => setNewWsName(e.target.value)}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
            <input
              type="text"
              placeholder="Description (optional)"
              value={newWsDesc}
              onChange={(e) => setNewWsDesc(e.target.value)}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
          </div>

          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setIsCreating(false)}
              className="px-3 py-1.5 text-xs font-medium text-[var(--muted)] hover:text-[var(--text)] rounded-lg"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-1.5 text-xs font-semibold bg-[var(--accent)] text-white rounded-lg shadow-sm"
            >
              Create
            </button>
          </div>
        </form>
      )}

      {/* Workspaces list */}
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl shadow-sm divide-y divide-[var(--border)] overflow-hidden">
        {workspaces.map((w) => {
          const isActive = activeWsId === w.id.toString();

          return (
            <div
              key={w.id}
              className="p-4 flex items-center justify-between gap-4 hover:bg-[var(--surface-hover)] transition"
            >
              <div
                onClick={() => handleSelectWorkspace(w.id)}
                className="flex items-center gap-3.5 flex-1 cursor-pointer min-w-0"
              >
                <div className={`p-2.5 rounded-xl ${isActive ? 'bg-[var(--accent)] text-white' : 'bg-[var(--surface-hover)] text-[var(--muted)]'}`}>
                  <Folder className="w-5 h-5" />
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm truncate text-[var(--text)]">{w.name}</span>
                    {isActive && (
                      <span className="text-[10px] bg-[var(--accent)]/10 text-[var(--accent)] border border-[var(--accent)]/20 px-1.5 py-0.5 rounded font-medium">
                        Active
                      </span>
                    )}
                  </div>
                  {w.description && <p className="text-xs text-[var(--muted)] mt-0.5">{w.description}</p>}
                </div>
              </div>

              <div className="flex items-center gap-2">
                {!isActive && (
                  <button
                    onClick={() => handleSelectWorkspace(w.id)}
                    className="p-2 rounded-xl bg-[var(--surface-hover)] hover:bg-[var(--border)] text-xs font-medium text-[var(--text)] transition flex items-center gap-1"
                  >
                    <Check className="w-3.5 h-3.5" />
                    <span>Switch</span>
                  </button>
                )}

                <button
                  onClick={() => handleDelete(w.id, w.name)}
                  className="p-2 text-[var(--muted)] hover:text-red-500 rounded-xl hover:bg-red-500/10 transition"
                  title="Delete workspace"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

import React, { useState, useEffect } from 'react';
import {
  ListMusic,
  Plus,
  Trash2,
  Image as ImageIcon,
  Music,
  BookOpen,
  X,
  ExternalLink,
} from 'lucide-react';
import type { components } from '../api/schema';
import { getStoredToken } from '../api/client';

type MediaList = components['schemas']['MediaListDto'];
type MediaItem = components['schemas']['MediaDto'];
type MediaListDetail = components['schemas']['MediaListDetailDto'];

export const MediaView: React.FC = () => {
  const [tab, setTab] = useState<'lists' | 'items'>('lists');
  const [lists, setLists] = useState<MediaList[]>([]);
  const [items, setItems] = useState<MediaItem[]>([]);
  const [selectedList, setSelectedList] = useState<MediaListDetail | null>(null);

  // New list form
  const [newListName, setNewListName] = useState('');
  const [newListDesc, setNewListDesc] = useState('');
  const [isCreatingList, setIsCreatingList] = useState(false);

  // New item form
  const [newItemName, setNewItemName] = useState('');
  const [newItemUrl, setNewItemUrl] = useState('');
  const [newItemType, setNewItemType] = useState<'image' | 'music' | 'poem'>('music');
  const [newItemDesc, setNewItemDesc] = useState('');
  const [newItemContent, setNewItemContent] = useState('');
  const [isCreatingItem, setIsCreatingItem] = useState(false);

  const loadData = async () => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const [resLists, resItems] = await Promise.all([
        fetch('/api/v1/media-lists', { headers }),
        fetch('/api/v1/medias', { headers }),
      ]);

      if (resLists.ok) setLists(await resLists.json());
      if (resItems.ok) setItems(await resItems.json());
    } catch (e) {
      console.error('Failed to load media data', e);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const loadListDetail = async (id: number) => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/media-lists/${id}`, { headers });
      if (res.ok) setSelectedList(await res.json());
    } catch (e) {
      console.error('Failed to load media list detail', e);
    }
  };

  const handleCreateList = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newListName.trim()) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/media-lists', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          name: newListName.trim(),
          description: newListDesc.trim() || null,
        }),
      });

      if (res.ok) {
        setNewListName('');
        setNewListDesc('');
        setIsCreatingList(false);
        loadData();
      }
    } catch (e) {
      console.error('Failed to create media list', e);
    }
  };

  const handleCreateItem = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newItemName.trim() || !newItemUrl.trim()) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/medias', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          name: newItemName.trim(),
          url: newItemUrl.trim(),
          media_type: newItemType,
          description: newItemDesc.trim() || null,
          content: newItemType === 'poem' ? (newItemContent.trim() || null) : null,
        }),
      });

      if (res.ok) {
        setNewItemName('');
        setNewItemUrl('');
        setNewItemDesc('');
        setNewItemContent('');
        setIsCreatingItem(false);
        loadData();
      }
    } catch (e) {
      console.error('Failed to create media item', e);
    }
  };

  const handleAddItemToList = async (listId: number, mediaId: number) => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Let-Timer-Protocol': '1',
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/media-lists/${listId}/medias`, {
        method: 'POST',
        headers,
        body: JSON.stringify({ media_id: mediaId }),
      });

      if (res.ok) loadListDetail(listId);
    } catch (e) {
      console.error('Failed to attach media to list', e);
    }
  };

  const handleRemoveItemFromList = async (listId: number, mediaId: number) => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/media-lists/${listId}/medias/${mediaId}`, {
        method: 'DELETE',
        headers,
      });

      if (res.ok) loadListDetail(listId);
    } catch (e) {
      console.error('Failed to detach media from list', e);
    }
  };

  const handleDeleteList = async (id: number) => {
    if (!confirm(`Delete media list #${id}?`)) return;
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/media-lists/${id}`, { method: 'DELETE', headers });
      if (res.ok) {
        setSelectedList(null);
        loadData();
      }
    } catch (e) {
      console.error('Failed to delete media list', e);
    }
  };

  const handleDeleteItem = async (id: number) => {
    if (!confirm(`Delete media item #${id}?`)) return;
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/medias/${id}`, { method: 'DELETE', headers });
      if (res.ok) loadData();
    } catch (e) {
      console.error('Failed to delete media item', e);
    }
  };

  return (
    <div className="max-w-5xl mx-auto px-4 sm:px-6 py-8">
      {/* Header and tab switcher */}
      <div className="flex flex-wrap items-center justify-between gap-4 mb-6">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text)]">Media Library</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Curate playlists, poems, and background imagery for your focus sessions.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center bg-[var(--surface)] border border-[var(--border)] p-1 rounded-xl text-xs font-semibold">
            <button
              onClick={() => {
                setTab('lists');
                setSelectedList(null);
              }}
              className={`px-3 py-1.5 rounded-lg transition ${
                tab === 'lists' ? 'bg-[var(--accent)] text-white' : 'text-[var(--muted)] hover:text-[var(--text)]'
              }`}
            >
              Media Lists
            </button>
            <button
              onClick={() => {
                setTab('items');
                setSelectedList(null);
              }}
              className={`px-3 py-1.5 rounded-lg transition ${
                tab === 'items' ? 'bg-[var(--accent)] text-white' : 'text-[var(--muted)] hover:text-[var(--text)]'
              }`}
            >
              All Items ({items.length})
            </button>
          </div>

          {tab === 'lists' ? (
            <button
              onClick={() => setIsCreatingList(true)}
              className="flex items-center gap-1.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-3 py-1.5 rounded-xl text-xs font-semibold shadow-sm transition"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>New List</span>
            </button>
          ) : (
            <button
              onClick={() => setIsCreatingItem(true)}
              className="flex items-center gap-1.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-3 py-1.5 rounded-xl text-xs font-semibold shadow-sm transition"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>New Item</span>
            </button>
          )}
        </div>
      </div>

      {/* New List Form */}
      {isCreatingList && (
        <form onSubmit={handleCreateList} className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 mb-6 shadow-sm">
          <h2 className="text-sm font-bold mb-3">Add New Media List</h2>
          <div className="grid sm:grid-cols-2 gap-3 mb-4">
            <input
              type="text"
              required
              placeholder="List name (e.g. Deep Coding Mix)"
              value={newListName}
              onChange={(e) => setNewListName(e.target.value)}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
            <input
              type="text"
              placeholder="Description (optional)"
              value={newListDesc}
              onChange={(e) => setNewListDesc(e.target.value)}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
          </div>
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setIsCreatingList(false)}
              className="px-3 py-1.5 text-xs text-[var(--muted)] rounded-lg"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-1.5 text-xs font-semibold bg-[var(--accent)] text-white rounded-lg shadow-sm"
            >
              Create List
            </button>
          </div>
        </form>
      )}

      {/* New Item Form */}
      {isCreatingItem && (
        <form onSubmit={handleCreateItem} className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 mb-6 shadow-sm">
          <h2 className="text-sm font-bold mb-3">Add New Media Item</h2>
          <div className="grid sm:grid-cols-3 gap-3 mb-3">
            <input
              type="text"
              required
              placeholder="Media title (e.g. Forest Birds)"
              value={newItemName}
              onChange={(e) => setNewItemName(e.target.value)}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
            <input
              type="text"
              required
              placeholder="Media URL or URI"
              value={newItemUrl}
              onChange={(e) => setNewItemUrl(e.target.value)}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
            <select
              value={newItemType}
              onChange={(e) => setNewItemType(e.target.value as 'image' | 'music' | 'poem')}
              className="px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            >
              <option value="music">Music / Audio</option>
              <option value="image">Image / Art</option>
              <option value="poem">Poem (Schema v4)</option>
            </select>
          </div>

          <div className="mb-3">
            <input
              type="text"
              placeholder="Description (optional)"
              value={newItemDesc}
              onChange={(e) => setNewItemDesc(e.target.value)}
              className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)]"
            />
          </div>

          {newItemType === 'poem' && (
            <div className="mb-4">
              <label className="block text-xs font-semibold text-[var(--muted)] mb-1">
                Poem Text Content
              </label>
              <textarea
                rows={4}
                placeholder="Enter inline poem verses here…"
                value={newItemContent}
                onChange={(e) => setNewItemContent(e.target.value)}
                className="w-full px-3 py-2 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-sm outline-none focus:border-[var(--accent)] font-serif"
              />
            </div>
          )}

          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setIsCreatingItem(false)}
              className="px-3 py-1.5 text-xs text-[var(--muted)] rounded-lg"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-1.5 text-xs font-semibold bg-[var(--accent)] text-white rounded-lg shadow-sm"
            >
              Create Item
            </button>
          </div>
        </form>
      )}

      {/* Content for Media Lists Tab */}
      {tab === 'lists' && (
        <div className="grid md:grid-cols-3 gap-6">
          {/* Lists Column */}
          <div className="md:col-span-1 flex flex-col gap-2">
            <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--muted)] mb-1">Playlists</h2>
            {lists.map((l) => (
              <div
                key={l.id}
                onClick={() => loadListDetail(l.id)}
                className={`p-3 rounded-xl border cursor-pointer transition flex items-center justify-between gap-2 ${
                  selectedList?.list.id === l.id
                    ? 'border-[var(--accent)] bg-[var(--surface-hover)] shadow-sm'
                    : 'border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)]'
                }`}
              >
                <div className="min-w-0">
                  <div className="font-semibold text-sm truncate text-[var(--text)]">{l.name}</div>
                  {l.description && <div className="text-xs text-[var(--muted)] truncate">{l.description}</div>}
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDeleteList(l.id);
                  }}
                  className="p-1 text-[var(--muted)] hover:text-red-500 rounded-lg"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            ))}
          </div>

          {/* Selected List Detail Column */}
          <div className="md:col-span-2">
            {selectedList ? (
              <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
                <div className="flex items-center justify-between pb-4 border-b border-[var(--border)] mb-4">
                  <div>
                    <h2 className="text-lg font-bold text-[var(--text)]">{selectedList.list.name}</h2>
                    {selectedList.list.description && (
                      <p className="text-xs text-[var(--muted)] mt-0.5">{selectedList.list.description}</p>
                    )}
                  </div>
                  <span className="text-xs text-[var(--muted)]">{selectedList.items.length} items</span>
                </div>

                {/* Items in this list */}
                <div className="divide-y divide-[var(--border)] mb-6">
                  {selectedList.items.length === 0 ? (
                    <div className="py-6 text-center text-xs text-[var(--muted)]">
                      No media items attached yet. Choose an item below to attach.
                    </div>
                  ) : (
                    selectedList.items.map((item) => (
                      <div key={item.id} className="py-2.5 flex items-center justify-between gap-3 text-xs">
                        <div className="flex items-center gap-2 min-w-0">
                          {item.media_type === 'music' && <Music className="w-4 h-4 text-sky-500 shrink-0" />}
                          {item.media_type === 'image' && <ImageIcon className="w-4 h-4 text-emerald-500 shrink-0" />}
                          {item.media_type === 'poem' && <BookOpen className="w-4 h-4 text-amber-500 shrink-0" />}
                          <span className="font-medium truncate text-[var(--text)]">{item.name}</span>
                        </div>
                        <button
                          onClick={() => handleRemoveItemFromList(selectedList.list.id, item.id)}
                          className="p-1 text-[var(--muted)] hover:text-red-500 rounded"
                          title="Remove from list"
                        >
                          <X className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    ))
                  )}
                </div>

                {/* Attach more items dropdown */}
                <div className="pt-4 border-t border-[var(--border)]">
                  <h3 className="text-xs font-semibold text-[var(--muted)] mb-2">Attach an Item to this List</h3>
                  <div className="flex items-center gap-2">
                    <select
                      id="attach-select"
                      className="flex-1 px-3 py-1.5 rounded-xl bg-[var(--surface-hover)] border border-[var(--border)] text-xs outline-none"
                    >
                      {items
                        .filter((i) => !selectedList.items.some((si) => si.id === i.id))
                        .map((i) => (
                          <option key={i.id} value={i.id}>
                            [{i.media_type}] {i.name}
                          </option>
                        ))}
                    </select>
                    <button
                      onClick={() => {
                        const sel = document.getElementById('attach-select') as HTMLSelectElement;
                        if (sel && sel.value) handleAddItemToList(selectedList.list.id, parseInt(sel.value, 10));
                      }}
                      className="px-3 py-1.5 rounded-xl bg-[var(--accent)] text-white text-xs font-semibold"
                    >
                      Attach
                    </button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="bg-[var(--surface)] border border-dashed border-[var(--border)] rounded-2xl p-12 text-center text-sm text-[var(--muted)]">
                <ListMusic className="w-8 h-8 mx-auto mb-2 opacity-50" />
                Select a media list on the left to view or edit its contents.
              </div>
            )}
          </div>
        </div>
      )}

      {/* Content for All Media Items Tab */}
      {tab === 'items' && (
        <div className="bg-[var(--surface)] border border-[var(--border)] rounded-2xl shadow-sm divide-y divide-[var(--border)] overflow-hidden">
          {items.map((item) => (
            <div key={item.id} className="p-4 flex items-center justify-between gap-4 hover:bg-[var(--surface-hover)] transition">
              <div className="flex items-center gap-3.5 min-w-0">
                <div className="p-2.5 rounded-xl bg-[var(--surface-hover)]">
                  {item.media_type === 'music' && <Music className="w-5 h-5 text-sky-500" />}
                  {item.media_type === 'image' && <ImageIcon className="w-5 h-5 text-emerald-500" />}
                  {item.media_type === 'poem' && <BookOpen className="w-5 h-5 text-amber-500" />}
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm truncate text-[var(--text)]">{item.name}</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--surface-hover)] text-[var(--muted)] uppercase font-medium">
                      {item.media_type}
                    </span>
                  </div>
                  <div className="text-xs text-[var(--muted)] mt-0.5 flex items-center gap-2 truncate">
                    <a
                      href={item.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="hover:underline flex items-center gap-1 text-[var(--accent)]"
                    >
                      <span className="truncate max-w-xs">{item.url}</span>
                      <ExternalLink className="w-3 h-3" />
                    </a>
                    {item.content && <span>• Has Poem Content</span>}
                  </div>
                </div>
              </div>

              <button
                onClick={() => handleDeleteItem(item.id)}
                className="p-2 text-[var(--muted)] hover:text-red-500 rounded-xl hover:bg-red-500/10 transition"
                title="Delete media item"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

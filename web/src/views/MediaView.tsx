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
import {
  Button,
  IconButton,
  SegmentedControl,
  Card,
  FormField,
  Input,
  Textarea,
  Select,
  EmptyState,
  Skeleton,
  useToast,
  useConfirm,
} from '../components/ui';

type MediaList = components['schemas']['MediaListDto'];
type MediaItem = components['schemas']['MediaDto'];
type MediaListDetail = components['schemas']['MediaListDetailDto'];

export const MediaView: React.FC = () => {
  const [tab, setTab] = useState<'lists' | 'items'>('lists');
  const [lists, setLists] = useState<MediaList[]>([]);
  const [items, setItems] = useState<MediaItem[]>([]);
  const [selectedList, setSelectedList] = useState<MediaListDetail | null>(null);
  const [isLoading, setIsLoading] = useState(false);

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

  const { showToast } = useToast();
  const confirm = useConfirm();

  const loadData = async () => {
    setIsLoading(true);
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
      showToast('Failed to load media library', 'danger');
    } finally {
      setIsLoading(false);
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
        showToast('Created media list', 'success');
        loadData();
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to create list', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Error creating list', 'danger');
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
          content: newItemType === 'poem' ? newItemContent.trim() || null : null,
        }),
      });

      if (res.ok) {
        setNewItemName('');
        setNewItemUrl('');
        setNewItemDesc('');
        setNewItemContent('');
        setIsCreatingItem(false);
        showToast('Created media item', 'success');
        loadData();
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to create item', 'danger');
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Error creating item', 'danger');
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

      if (res.ok) {
        showToast('Item attached to list', 'success');
        loadListDetail(listId);
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to attach item', 'danger');
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

      if (res.ok) {
        showToast('Item detached from list', 'info');
        loadListDetail(listId);
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to detach item', 'danger');
    }
  };

  const handleDeleteList = async (id: number, name: string) => {
    const ok = await confirm({
      title: `Delete list "${name}"?`,
      body: 'This will not delete the attached media items.',
      tone: 'danger',
      confirmLabel: 'Delete list',
    });
    if (!ok) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/media-lists/${id}`, { method: 'DELETE', headers });
      if (res.ok) {
        setSelectedList(null);
        showToast('List deleted', 'success');
        loadData();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to delete list', 'danger');
    }
  };

  const handleDeleteItem = async (id: number, name: string) => {
    const ok = await confirm({
      title: `Delete media item "${name}"?`,
      tone: 'danger',
      confirmLabel: 'Delete item',
    });
    if (!ok) return;

    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/v1/medias/${id}`, { method: 'DELETE', headers });
      if (res.ok) {
        showToast('Item deleted', 'success');
        loadData();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to delete item', 'danger');
    }
  };

  return (
    <div className="max-w-6xl mx-auto px-4 sm:px-6 py-6 pb-24">
      {/* Header and tab switcher */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text)] tracking-tight">Media library</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Curated music, images, and poems shown in Focus mode.
          </p>
        </div>

        <div className="flex items-center gap-2 self-start sm:self-center">
          <SegmentedControl
            value={tab}
            onValueChange={(v) => {
              setTab(v as 'lists' | 'items');
              setSelectedList(null);
            }}
            items={[
              { value: 'lists', label: 'Lists' },
              { value: 'items', label: `Items (${items.length})` },
            ]}
          />

          {tab === 'lists' ? (
            <Button variant="primary" size="sm" onClick={() => setIsCreatingList(true)}>
              <Plus className="w-3.5 h-3.5 stroke-[3]" />
              <span>New list</span>
            </Button>
          ) : (
            <Button variant="primary" size="sm" onClick={() => setIsCreatingItem(true)}>
              <Plus className="w-3.5 h-3.5 stroke-[3]" />
              <span>New item</span>
            </Button>
          )}
        </div>
      </div>

      {/* New List Form */}
      {isCreatingList && (
        <Card className="mb-6">
          <h2 className="text-sm font-bold text-[var(--text)] mb-3">Add new media list</h2>
          <form onSubmit={handleCreateList} className="flex flex-col gap-3">
            <div className="grid sm:grid-cols-2 gap-3">
              <FormField label="List name" required>
                {(id) => (
                  <Input
                    id={id}
                    required
                    placeholder="e.g. Deep coding mix"
                    value={newListName}
                    onChange={(e) => setNewListName(e.target.value)}
                  />
                )}
              </FormField>
              <FormField label="Description (optional)">
                {(id) => (
                  <Input
                    id={id}
                    placeholder="Optional description"
                    value={newListDesc}
                    onChange={(e) => setNewListDesc(e.target.value)}
                  />
                )}
              </FormField>
            </div>
            <div className="flex justify-end gap-2 mt-2">
              <Button type="button" variant="ghost" size="sm" onClick={() => setIsCreatingList(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" size="sm">
                Create list
              </Button>
            </div>
          </form>
        </Card>
      )}

      {/* New Item Form */}
      {isCreatingItem && (
        <Card className="mb-6">
          <h2 className="text-sm font-bold text-[var(--text)] mb-3">Add new media item</h2>
          <form onSubmit={handleCreateItem} className="flex flex-col gap-3">
            <div className="grid sm:grid-cols-3 gap-3">
              <FormField label="Title" required>
                {(id) => (
                  <Input
                    id={id}
                    required
                    placeholder="e.g. Forest birds"
                    value={newItemName}
                    onChange={(e) => setNewItemName(e.target.value)}
                  />
                )}
              </FormField>

              <FormField label="URL or URI" required>
                {(id) => (
                  <Input
                    id={id}
                    required
                    placeholder="https://... or data:..."
                    value={newItemUrl}
                    onChange={(e) => setNewItemUrl(e.target.value)}
                  />
                )}
              </FormField>

              <FormField label="Type">
                {(id) => (
                  <Select
                    id={id}
                    value={newItemType}
                    onChange={(e) => setNewItemType(e.target.value as 'image' | 'music' | 'poem')}
                  >
                    <option value="music">Music / Audio</option>
                    <option value="image">Image / Art</option>
                    <option value="poem">Poem (Schema v4)</option>
                  </Select>
                )}
              </FormField>
            </div>

            <FormField label="Description (optional)">
              {(id) => (
                <Input
                  id={id}
                  placeholder="Optional description"
                  value={newItemDesc}
                  onChange={(e) => setNewItemDesc(e.target.value)}
                />
              )}
            </FormField>

            {newItemType === 'poem' && (
              <FormField label="Inline poem verses (optional)">
                {(id) => (
                  <Textarea
                    id={id}
                    rows={4}
                    placeholder="Enter poem verses here…"
                    value={newItemContent}
                    onChange={(e) => setNewItemContent(e.target.value)}
                    className="font-serif"
                  />
                )}
              </FormField>
            )}

            <div className="flex justify-end gap-2 mt-2">
              <Button type="button" variant="ghost" size="sm" onClick={() => setIsCreatingItem(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" size="sm">
                Create item
              </Button>
            </div>
          </form>
        </Card>
      )}

      {/* Content for Media Lists Tab */}
      {tab === 'lists' && (
        <div className="grid md:grid-cols-3 gap-6">
          {/* Lists Column */}
          <div className="md:col-span-1 flex flex-col gap-2">
            <h2 className="text-xs font-semibold text-[var(--muted)] mb-1">Playlists</h2>
            {isLoading ? (
              <div className="flex flex-col gap-2">
                <Skeleton height="48px" />
                <Skeleton height="48px" />
              </div>
            ) : lists.length === 0 ? (
              <div className="text-xs text-[var(--muted)] py-4">No media lists yet.</div>
            ) : (
              lists.map((l) => (
                <div
                  key={l.id}
                  onClick={() => loadListDetail(l.id)}
                  className={`p-3 rounded-[var(--radius-control)] border cursor-pointer transition flex items-center justify-between gap-2 select-none ${
                    selectedList?.list.id === l.id
                      ? 'border-[var(--accent-text)] bg-[var(--surface-hover)] shadow-xs'
                      : 'border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)]'
                  }`}
                >
                  <div className="min-w-0">
                    <div className="font-semibold text-xs sm:text-sm truncate text-[var(--text)]">
                      {l.name}
                    </div>
                    {l.description && (
                      <div className="text-[11px] text-[var(--muted)] truncate">{l.description}</div>
                    )}
                  </div>
                  <IconButton
                    variant="danger"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteList(l.id, l.name);
                    }}
                    aria-label={`Delete list ${l.name}`}
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </IconButton>
                </div>
              ))
            )}
          </div>

          {/* Selected List Detail Column */}
          <div className="md:col-span-2">
            {selectedList ? (
              <Card>
                <div className="flex items-center justify-between pb-3 border-b border-[var(--border)] mb-3">
                  <div>
                    <h2 className="text-base font-bold text-[var(--text)]">{selectedList.list.name}</h2>
                    {selectedList.list.description && (
                      <p className="text-xs text-[var(--muted)] mt-0.5">{selectedList.list.description}</p>
                    )}
                  </div>
                  <span className="text-xs text-[var(--muted)]">{selectedList.items.length} items</span>
                </div>

                {/* Items in this list */}
                <div className="divide-y divide-[var(--border)] mb-4">
                  {selectedList.items.length === 0 ? (
                    <div className="py-6 text-center text-xs text-[var(--muted)]">
                      No media items attached yet. Choose an item below to attach.
                    </div>
                  ) : (
                    selectedList.items.map((item) => (
                      <div key={item.id} className="py-2 flex items-center justify-between gap-3 text-xs">
                        <div className="flex items-center gap-2.5 min-w-0">
                          {item.media_type === 'music' && (
                            <Music className="w-4 h-4 text-[var(--accent-text)] shrink-0" />
                          )}
                          {item.media_type === 'image' && (
                            <ImageIcon className="w-4 h-4 text-[var(--muted)] shrink-0" />
                          )}
                          {item.media_type === 'poem' && (
                            <BookOpen className="w-4 h-4 text-[var(--muted)] shrink-0" />
                          )}
                          <span className="font-medium truncate text-[var(--text)]">{item.name}</span>
                        </div>
                        <IconButton
                          variant="ghost"
                          size="sm"
                          onClick={() => handleRemoveItemFromList(selectedList.list.id, item.id)}
                          aria-label={`Detach ${item.name}`}
                        >
                          <X className="w-3.5 h-3.5 text-[var(--muted)] hover:text-[var(--danger-text)]" />
                        </IconButton>
                      </div>
                    ))
                  )}
                </div>

                {/* Attach more items dropdown */}
                <div className="pt-3 border-t border-[var(--border)]">
                  <h3 className="text-xs font-semibold text-[var(--muted)] mb-2">Attach an item to this list</h3>
                  <div className="flex items-center gap-2">
                    <select
                      id="attach-select"
                      className="flex-1 px-3 py-1.5 rounded-[var(--radius-control)] bg-[var(--surface-hover)] border border-[var(--border-strong)] text-xs outline-none text-[var(--text)]"
                    >
                      {items
                        .filter((i) => !selectedList.items.some((si) => si.id === i.id))
                        .map((i) => (
                          <option key={i.id} value={i.id}>
                            [{i.media_type}] {i.name}
                          </option>
                        ))}
                    </select>
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => {
                        const sel = document.getElementById('attach-select') as HTMLSelectElement;
                        if (sel && sel.value) handleAddItemToList(selectedList.list.id, parseInt(sel.value, 10));
                      }}
                    >
                      Attach
                    </Button>
                  </div>
                </div>
              </Card>
            ) : (
              <EmptyState
                icon={<ListMusic className="w-8 h-8 text-[var(--muted)]" />}
                title="Select a playlist"
                description="Choose a media list on the left to view and curate its audio, artwork, and poems."
              />
            )}
          </div>
        </div>
      )}

      {/* Content for All Media Items Tab */}
      {tab === 'items' && (
        <div>
          {isLoading ? (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              <Skeleton height="90px" />
              <Skeleton height="90px" />
              <Skeleton height="90px" />
            </div>
          ) : items.length === 0 ? (
            <EmptyState
              title="Add your first item"
              description="Images become Focus backgrounds, audio plays during sessions, and poems appear in the reader."
              action={{
                label: 'New item',
                onClick: () => setIsCreatingItem(true),
              }}
            />
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {items.map((item) => (
                <Card key={item.id} className="p-3.5 flex flex-col justify-between gap-3">
                  <div className="flex items-start gap-3 min-w-0">
                    <div className="p-2 rounded-[var(--radius-control)] bg-[var(--surface-hover)] text-[var(--muted)] shrink-0">
                      {item.media_type === 'music' && <Music className="w-4 h-4 text-[var(--accent-text)]" />}
                      {item.media_type === 'image' && <ImageIcon className="w-4 h-4" />}
                      {item.media_type === 'poem' && <BookOpen className="w-4 h-4" />}
                    </div>

                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-1.5">
                        <span className="font-semibold text-xs sm:text-sm truncate text-[var(--text)]">
                          {item.name}
                        </span>
                      </div>
                      <span className="text-[10px] text-[var(--muted)] capitalize block mt-0.5">
                        {item.media_type}
                      </span>
                      {item.description && (
                        <p className="text-[11px] text-[var(--muted)] truncate mt-1">{item.description}</p>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-[var(--border)] text-[11px]">
                    <a
                      href={item.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-[var(--accent-text)] hover:underline inline-flex items-center gap-1 truncate max-w-[180px]"
                    >
                      <span className="truncate">{item.url}</span>
                      <ExternalLink className="w-3 h-3 shrink-0" />
                    </a>

                    <IconButton
                      variant="danger"
                      size="sm"
                      onClick={() => handleDeleteItem(item.id, item.name)}
                      aria-label={`Delete ${item.name}`}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </IconButton>
                  </div>
                </Card>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

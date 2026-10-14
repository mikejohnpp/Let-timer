import React, { useState, useEffect, useRef } from 'react';
import {
  Plus,
  Trash2,
  Image as ImageIcon,
  BookOpen,
  X,
  ExternalLink,
  Upload,
  Play,
  Pause,
} from 'lucide-react';
import type { components } from '../api/schema';
import { apiFetch } from '../api/client';
import { useAudioPlayer } from '../context/AudioPlayerContext';
import { useAutoAnimate } from '@formkit/auto-animate/react';
import {
  Button,
  IconButton,
  SegmentedControl,
  FormField,
  Input,
  Textarea,
  Select,
  Skeleton,
  useToast,
  useConfirm,
} from '../components/ui';

type MediaList = components['schemas']['MediaListDto'];
type MediaItem = components['schemas']['MediaDto'];
type MediaListDetail = components['schemas']['MediaListDetailDto'];

function getDisplayHost(url: string) {
  if (url.startsWith('/api/v1/media/files/')) {
    const filename = url.split('/').pop() || 'file';
    return `local storage (${filename.substring(0, 10)}…)`;
  }
  try {
    return new URL(url).hostname.replace('www.', '');
  } catch {
    return url.length > 25 ? url.substring(0, 22) + '…' : url;
  }
}

export const MediaView: React.FC = () => {
  const { currentTrack, isPlaying, playTrack, togglePlay } = useAudioPlayer();
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
  const [newItemSource, setNewItemSource] = useState<'upload' | 'url'>('upload');
  const [selectedUploadFile, setSelectedUploadFile] = useState<File | null>(null);
  const [newItemName, setNewItemName] = useState('');
  const [newItemUrl, setNewItemUrl] = useState('');
  const [newItemType, setNewItemType] = useState<'image' | 'music' | 'poem'>('music');
  const [newItemDesc, setNewItemDesc] = useState('');
  const [newItemContent, setNewItemContent] = useState('');
  const [isCreatingItem, setIsCreatingItem] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const itemFileInputRef = useRef<HTMLInputElement | null>(null);

  const { showToast } = useToast();
  const confirm = useConfirm();

  // Animation tracking states
  const [newlyAddedListId, setNewlyAddedListId] = useState<number | null>(null);
  const [newlyAddedItemId, setNewlyAddedItemId] = useState<number | null>(null);
  const [newlyAttachedItemId, setNewlyAttachedItemId] = useState<number | null>(null);
  const isFirstLoadRef = useRef(true);

  // Animated list refs
  const [playlistsListRef] = useAutoAnimate<HTMLDivElement>({ duration: 200, easing: 'ease-out' });
  const [playlistItemsRef] = useAutoAnimate<HTMLDivElement>({ duration: 200, easing: 'ease-out' });
  const [libraryItemsRef] = useAutoAnimate<HTMLDivElement>({ duration: 200, easing: 'ease-out' });
  const [formsContainerRef] = useAutoAnimate<HTMLDivElement>({ duration: 200, easing: 'ease-out' });

  const loadData = async (showSkeleton = false) => {
    if (showSkeleton && isFirstLoadRef.current) {
      setIsLoading(true);
    }
    try {
      const [resLists, resItems] = await Promise.all([
        apiFetch('/api/v1/media-lists'),
        apiFetch('/api/v1/medias'),
      ]);

      if (resLists.ok) setLists(await resLists.json());
      if (resItems.ok) setItems(await resItems.json());
    } catch (e) {
      console.error('Failed to load media data', e);
      showToast('Failed to load media library', 'danger');
    } finally {
      setIsLoading(false);
      isFirstLoadRef.current = false;
    }
  };

  useEffect(() => {
    loadData(true);
  }, []);

  const loadListDetail = async (id: number) => {
    try {
      const res = await apiFetch(`/api/v1/media-lists/${id}`);
      if (res.ok) setSelectedList(await res.json());
    } catch (e) {
      console.error('Failed to load media list detail', e);
    }
  };

  const handleCreateList = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newListName.trim()) return;

    try {
      const res = await apiFetch('/api/v1/media-lists', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: newListName.trim(),
          description: newListDesc.trim() || null,
        }),
      });

      if (res.ok) {
        const created: MediaList = await res.json().catch(() => null);
        setNewListName('');
        setNewListDesc('');
        setIsCreatingList(false);
        if (created?.id) {
          setNewlyAddedListId(created.id);
          setTimeout(() => setNewlyAddedListId(null), 700);
        }
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

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    setSelectedUploadFile(file);
    if (!newItemName.trim()) {
      setNewItemName(file.name.replace(/\.[^/.]+$/, ''));
    }
    if (file.type.startsWith('audio/') || /\.(mp3|wav|ogg|flac|m4a|aac)$/i.test(file.name)) {
      setNewItemType('music');
    } else if (file.type.startsWith('image/') || /\.(jpg|jpeg|png|gif|webp|svg|avif)$/i.test(file.name)) {
      setNewItemType('image');
    }
  };

  const handleCreateItem = async (e: React.FormEvent) => {
    e.preventDefault();

    if (newItemSource === 'upload') {
      if (!selectedUploadFile) {
        showToast('Please select a file to upload', 'danger');
        return;
      }
      setIsUploading(true);
      try {
        const formData = new FormData();
        formData.append('file', selectedUploadFile);
        formData.append('name', newItemName.trim() || selectedUploadFile.name.replace(/\.[^/.]+$/, ''));
        formData.append('media_type', newItemType === 'poem' ? 'image' : newItemType);
        if (newItemDesc.trim()) {
          formData.append('description', newItemDesc.trim());
        }

        const res = await apiFetch('/api/v1/media/upload', {
          method: 'POST',
          body: formData,
        });

        if (res.ok) {
          const created: MediaItem = await res.json().catch(() => null);
          setNewItemName('');
          setNewItemUrl('');
          setNewItemDesc('');
          setNewItemContent('');
          setSelectedUploadFile(null);
          if (itemFileInputRef.current) itemFileInputRef.current.value = '';
          setIsCreatingItem(false);
          if (created?.id) {
            setNewlyAddedItemId(created.id);
            setTimeout(() => setNewlyAddedItemId(null), 700);
          }
          showToast('Media uploaded and saved to library', 'success');
          loadData();
        } else {
          const err = await res.json().catch(() => ({}));
          showToast(err.message || 'Failed to upload media', 'danger');
        }
      } catch (e: unknown) {
        showToast(e instanceof Error ? e.message : 'Error uploading file', 'danger');
      } finally {
        setIsUploading(false);
      }
      return;
    }

    if (!newItemName.trim() || !newItemUrl.trim()) return;

    try {
      const res = await apiFetch('/api/v1/medias', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: newItemName.trim(),
          url: newItemUrl.trim(),
          media_type: newItemType,
          description: newItemDesc.trim() || null,
          content: newItemType === 'poem' ? newItemContent.trim() || null : null,
        }),
      });

      if (res.ok) {
        const created: MediaItem = await res.json().catch(() => null);
        setNewItemName('');
        setNewItemUrl('');
        setNewItemDesc('');
        setNewItemContent('');
        setSelectedUploadFile(null);
        setIsCreatingItem(false);
        if (created?.id) {
          setNewlyAddedItemId(created.id);
          setTimeout(() => setNewlyAddedItemId(null), 700);
        }
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
    const itemToAttach = items.find((i) => i.id === mediaId);
    if (itemToAttach && selectedList && selectedList.list.id === listId) {
      setNewlyAttachedItemId(mediaId);
      setSelectedList({
        ...selectedList,
        items: [...selectedList.items, itemToAttach],
      });
      setTimeout(() => setNewlyAttachedItemId(null), 700);
    }

    try {
      const res = await apiFetch(`/api/v1/media-lists/${listId}/medias`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ media_id: mediaId }),
      });

      if (res.ok) {
        showToast('Item attached to list', 'success');
        loadListDetail(listId);
      } else {
        const err = await res.json().catch(() => ({}));
        showToast(err.message || 'Failed to attach item', 'danger');
        loadListDetail(listId);
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to attach item', 'danger');
      loadListDetail(listId);
    }
  };

  const handleRemoveItemFromList = async (listId: number, mediaId: number) => {
    setSelectedList((prev) =>
      prev ? { ...prev, items: prev.items.filter((i) => i.id !== mediaId) } : prev
    );

    try {
      const res = await apiFetch(`/api/v1/media-lists/${listId}/medias/${mediaId}`, {
        method: 'DELETE',
      });

      if (res.ok) {
        showToast('Item detached from list', 'info');
        loadListDetail(listId);
      } else {
        loadListDetail(listId);
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to detach item', 'danger');
      loadListDetail(listId);
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

    setLists((prev) => prev.filter((l) => l.id !== id));
    if (selectedList?.list.id === id) {
      setSelectedList(null);
    }

    try {
      const res = await apiFetch(`/api/v1/media-lists/${id}`, { method: 'DELETE' });
      if (res.ok) {
        showToast('List deleted', 'success');
        loadData();
      } else {
        loadData();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to delete list', 'danger');
      loadData();
    }
  };

  const handleDeleteItem = async (id: number, name: string) => {
    const ok = await confirm({
      title: `Delete media item "${name}"?`,
      tone: 'danger',
      confirmLabel: 'Delete item',
    });
    if (!ok) return;

    setItems((prev) => prev.filter((i) => i.id !== id));

    try {
      const res = await apiFetch(`/api/v1/medias/${id}`, { method: 'DELETE' });
      if (res.ok) {
        showToast('Item deleted', 'success');
        loadData();
      } else {
        loadData();
      }
    } catch (e: unknown) {
      showToast(e instanceof Error ? e.message : 'Failed to delete item', 'danger');
      loadData();
    }
  };

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 py-8 pb-28 flex flex-col gap-6">
      {/* Header and tab switcher */}
      <div className="flex flex-col sm:flex-row sm:items-baseline justify-between gap-4 mb-2">
        <div>
          <h1 className="text-2xl sm:text-3xl font-serif font-bold text-[var(--text)] tracking-tight">Media</h1>
          <p className="text-xs text-[var(--muted)] mt-1">
            Curated music, artwork, and poems for focus sessions
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
              { value: 'lists', label: 'Playlists' },
              { value: 'items', label: `Library (${items.length})` },
            ]}
          />

          {tab === 'lists' ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsCreatingList(true)}
              className="text-xs font-semibold text-[var(--accent-text)] h-8 px-2.5"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>New list</span>
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsCreatingItem(true)}
              className="text-xs font-semibold text-[var(--accent-text)] h-8 px-2.5"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>New item</span>
            </Button>
          )}
        </div>
      </div>

      {/* New Creation Forms with smooth expand/collapse */}
      <div ref={formsContainerRef}>
        {/* New List Form */}
        {isCreatingList && (
        <div className="backdrop-blur-md bg-[var(--surface)]/50 border border-[var(--border)]/40 rounded-xl p-4 sm:p-5 mb-2">
          <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--muted)] mb-3">Add new media list</h2>
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
        </div>
      )}

      {/* New Item Form */}
      {isCreatingItem && (
        <div className="backdrop-blur-md bg-[var(--surface)]/50 border border-[var(--border)]/40 rounded-xl p-4 sm:p-5 mb-2">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--muted)]">Add new media item</h2>
            <div className="flex items-center gap-1 bg-[var(--surface-hover)]/60 p-0.5 rounded-lg border border-[var(--border)]/40">
              <button
                type="button"
                onClick={() => setNewItemSource('upload')}
                className={`px-2.5 py-1 text-xs rounded-md font-medium transition cursor-pointer ${
                  newItemSource === 'upload'
                    ? 'bg-[var(--surface)] text-[var(--text)] shadow-xs'
                    : 'text-[var(--muted)] hover:text-[var(--text)]'
                }`}
              >
                Upload File
              </button>
              <button
                type="button"
                onClick={() => setNewItemSource('url')}
                className={`px-2.5 py-1 text-xs rounded-md font-medium transition cursor-pointer ${
                  newItemSource === 'url'
                    ? 'bg-[var(--surface)] text-[var(--text)] shadow-xs'
                    : 'text-[var(--muted)] hover:text-[var(--text)]'
                }`}
              >
                External URL
              </button>
            </div>
          </div>

          <form onSubmit={handleCreateItem} className="flex flex-col gap-3">
            {newItemSource === 'upload' ? (
              <div className="flex flex-col gap-3">
                <FormField label="File (Image or Audio)" required>
                  {() => (
                    <div>
                      <input
                        ref={itemFileInputRef}
                        type="file"
                        accept="image/*,audio/*,.mp3,.wav,.ogg,.flac,.m4a,.aac,.jpg,.jpeg,.png,.gif,.webp,.svg,.avif"
                        className="hidden"
                        onChange={handleFileChange}
                      />
                      <div
                        onClick={() => itemFileInputRef.current?.click()}
                        className="border border-dashed border-[var(--border)] hover:border-[var(--accent-text)] rounded-lg p-4 text-center cursor-pointer bg-[var(--surface-hover)]/30 transition flex flex-col items-center gap-1.5"
                      >
                        <Upload className="w-5 h-5 text-[var(--muted)]" />
                        {selectedUploadFile ? (
                          <div className="text-xs">
                            <span className="font-medium text-[var(--text)]">{selectedUploadFile.name}</span>{' '}
                            <span className="text-[var(--muted)]">
                              ({(selectedUploadFile.size / 1024 / 1024).toFixed(2)} MB)
                            </span>
                          </div>
                        ) : (
                          <div className="text-xs text-[var(--muted)]">
                            <span className="font-medium text-[var(--accent-text)]">Click to choose a file</span> or drag & drop (Image or Audio)
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </FormField>

                <div className="grid sm:grid-cols-2 gap-3">
                  <FormField label="Title (optional, defaults to filename)">
                    {(id) => (
                      <Input
                        id={id}
                        placeholder="e.g. Ambient Focus"
                        value={newItemName}
                        onChange={(e) => setNewItemName(e.target.value)}
                      />
                    )}
                  </FormField>

                  <FormField label="Type">
                    {(id) => (
                      <Select
                        id={id}
                        value={newItemType}
                        onChange={(e) => setNewItemType(e.target.value as 'image' | 'music')}
                      >
                        <option value="music">Music / Audio</option>
                        <option value="image">Image / Art</option>
                      </Select>
                    )}
                  </FormField>
                </div>
              </div>
            ) : (
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
            )}

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

            {newItemSource === 'url' && newItemType === 'poem' && (
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
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setIsCreatingItem(false);
                  setSelectedUploadFile(null);
                  if (itemFileInputRef.current) itemFileInputRef.current.value = '';
                }}
              >
                Cancel
              </Button>
              <Button type="submit" variant="primary" size="sm" disabled={isUploading}>
                {isUploading ? 'Uploading…' : newItemSource === 'upload' ? 'Upload file' : 'Create item'}
              </Button>
            </div>
          </form>
        </div>
      )}
      </div>

      {/* Content for Media Lists Tab */}
      {tab === 'lists' && (
        <div className="grid md:grid-cols-3 gap-6 animate-[fade-in_var(--dur-fast)_var(--ease-out)]">
          {/* Lists Column */}
          <div className="md:col-span-1 flex flex-col gap-2">
            <div className="text-[10px] font-semibold uppercase tracking-widest text-[var(--muted)] mb-1 px-1">
              Playlists
            </div>
            {isLoading && lists.length === 0 ? (
              <div className="flex flex-col gap-2">
                <Skeleton height="40px" />
                <Skeleton height="40px" />
              </div>
            ) : lists.length === 0 ? (
              <div className="text-xs text-[var(--muted)] py-4 px-1">No playlists yet.</div>
            ) : (
              <div ref={playlistsListRef} className="flex flex-col divide-y divide-[var(--border)]/20 border-y border-[var(--border)]/20">
                {lists.map((l) => {
                  const isSelected = selectedList?.list.id === l.id;
                  const isNewlyAdded = newlyAddedListId === l.id;

                  return (
                    <div
                      key={l.id}
                      onClick={() => loadListDetail(l.id)}
                      className={`p-2.5 rounded-lg cursor-pointer transition flex items-center justify-between gap-2 select-none group ${
                        isNewlyAdded ? 'task-row-add' : ''
                      } ${
                        isSelected
                          ? 'bg-[var(--accent)]/10 text-[var(--accent-text)] font-semibold'
                          : 'hover:bg-[var(--surface-hover)]/50 text-[var(--text)]'
                      }`}
                    >
                      <div className="min-w-0">
                        <div className="text-xs sm:text-sm truncate">
                          {l.name}
                        </div>
                        {l.description && (
                          <div className="text-[11px] text-[var(--muted)] truncate mt-0.5">{l.description}</div>
                        )}
                      </div>
                      <IconButton
                        variant="ghost"
                        size="sm"
                        className="opacity-0 group-hover:opacity-100 transition-opacity text-[var(--muted)] hover:text-[var(--danger-text)] h-6 w-6 p-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteList(l.id, l.name);
                        }}
                        aria-label={`Delete list ${l.name}`}
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </IconButton>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* Selected List Detail Column */}
          <div className="md:col-span-2">
            {selectedList ? (
              <div key={selectedList.list.id} className="backdrop-blur-md bg-[var(--surface)]/30 border border-[var(--border)]/30 rounded-xl p-4 sm:p-5 animate-[fade-in_var(--dur-fast)_var(--ease-out)]">
                <div className="flex items-center justify-between pb-3 border-b border-[var(--border)]/30 mb-3">
                  <div>
                    <h2 className="text-sm font-semibold text-[var(--text)]">{selectedList.list.name}</h2>
                    {selectedList.list.description && (
                      <p className="text-xs text-[var(--muted)] mt-0.5">{selectedList.list.description}</p>
                    )}
                  </div>
                  <span className="text-xs text-[var(--muted)]">{selectedList.items.length} items</span>
                </div>

                {/* Items in this list */}
                <div ref={playlistItemsRef} className="divide-y divide-[var(--border)]/20 mb-4">
                  {selectedList.items.length === 0 ? (
                    <div className="py-8 text-center text-xs text-[var(--muted)]">
                      No media items attached yet. Choose an item below to attach.
                    </div>
                  ) : (
                    selectedList.items.map((item) => (
                      <div
                        key={item.id}
                        className={`py-2 px-1.5 flex items-center justify-between gap-3 text-xs group rounded-md transition-colors ${
                          newlyAttachedItemId === item.id ? 'task-row-add' : ''
                        } ${
                          currentTrack?.id === item.id && isPlaying ? 'bg-[var(--accent)]/10 text-[var(--accent-text)]' : 'hover:bg-[var(--surface-hover)]/30'
                        }`}
                      >
                        <div className="flex items-center gap-2.5 min-w-0">
                          {item.media_type === 'music' && (
                            <button
                              type="button"
                              onClick={() => {
                                if (currentTrack?.id === item.id) {
                                  togglePlay();
                                } else {
                                  playTrack(
                                    item,
                                    selectedList.items.filter((i) => i.media_type === 'music')
                                  );
                                }
                              }}
                              className="w-5 h-5 rounded-full flex items-center justify-center bg-[var(--surface-hover)] hover:bg-[var(--accent)] hover:text-white text-[var(--accent-text)] transition cursor-pointer shrink-0"
                              title={currentTrack?.id === item.id && isPlaying ? 'Pause' : 'Play'}
                            >
                              {currentTrack?.id === item.id && isPlaying ? (
                                <Pause className="w-2.5 h-2.5" />
                              ) : (
                                <Play className="w-2.5 h-2.5 ml-0.5" />
                              )}
                            </button>
                          )}
                          {item.media_type === 'image' && (
                            <ImageIcon className="w-3.5 h-3.5 text-[var(--muted)] shrink-0" />
                          )}
                          {item.media_type === 'poem' && (
                            <BookOpen className="w-3.5 h-3.5 text-[var(--muted)] shrink-0" />
                          )}
                          <span className="font-medium truncate text-[var(--text)]">{item.name}</span>
                          {currentTrack?.id === item.id && isPlaying && (
                            <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)] dot-running shrink-0" />
                          )}
                        </div>
                        <IconButton
                          variant="ghost"
                          size="sm"
                          className="opacity-0 group-hover:opacity-100 transition-opacity text-[var(--muted)] hover:text-[var(--danger-text)] h-6 w-6 p-0"
                          onClick={() => handleRemoveItemFromList(selectedList.list.id, item.id)}
                          aria-label={`Detach ${item.name}`}
                        >
                          <X className="w-3.5 h-3.5" />
                        </IconButton>
                      </div>
                    ))
                  )}
                </div>

                {/* Attach more items dropdown */}
                <div className="pt-3 border-t border-[var(--border)]/30">
                  <div className="text-[11px] font-medium text-[var(--muted)] mb-2">Attach an item to this list</div>
                  <div className="flex items-center gap-2 min-w-0">
                    <select
                      id="attach-select"
                      className="flex-1 min-w-0 px-3 py-1.5 rounded-lg bg-[var(--surface-hover)]/70 border border-[var(--border)]/50 text-xs outline-none text-[var(--text)] truncate"
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
                      variant="ghost"
                      size="sm"
                      className="text-xs font-semibold text-[var(--accent-text)] h-7 px-2.5 shrink-0"
                      onClick={() => {
                        const sel = document.getElementById('attach-select') as HTMLSelectElement;
                        if (sel && sel.value) handleAddItemToList(selectedList.list.id, parseInt(sel.value, 10));
                      }}
                    >
                      Attach
                    </Button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="py-16 text-center text-xs text-[var(--muted)]">
                Select a playlist on the left to view and curate its items.
              </div>
            )}
          </div>
        </div>
      )}

      {/* Content for All Media Items Tab */}
      {tab === 'items' && (
        <div className="animate-[fade-in_var(--dur-fast)_var(--ease-out)]">
          {isLoading && items.length === 0 ? (
            <div className="flex flex-col gap-2">
              <Skeleton height="44px" />
              <Skeleton height="44px" />
              <Skeleton height="44px" />
            </div>
          ) : items.length === 0 ? (
            <div className="py-16 text-center text-xs text-[var(--muted)] flex flex-col items-center gap-3">
              <span>No media items in library yet.</span>
              <button
                onClick={() => setIsCreatingItem(true)}
                className="text-xs text-[var(--accent-text)] hover:underline font-medium cursor-pointer"
              >
                + Add your first item
              </button>
            </div>
          ) : (
            <div ref={libraryItemsRef} className="flex flex-col divide-y divide-[var(--border)]/20 border-y border-[var(--border)]/20">
              {items.map((item) => {
                const isNewlyAdded = newlyAddedItemId === item.id;

                return (
                  <div
                    key={item.id}
                    className={`py-3 px-2 flex items-center justify-between gap-4 rounded-lg hover:bg-[var(--surface-hover)]/40 transition group select-none ${
                      isNewlyAdded ? 'task-row-add' : ''
                    } ${
                      currentTrack?.id === item.id && isPlaying ? 'bg-[var(--accent)]/10' : ''
                    }`}
                  >
                  <div className="flex items-center gap-3 min-w-0">
                    <div className="w-7 h-7 rounded-md bg-[var(--surface-hover)]/70 flex items-center justify-center text-[var(--muted)] shrink-0">
                      {item.media_type === 'music' ? (
                        <button
                          type="button"
                          onClick={() => {
                            if (currentTrack?.id === item.id) {
                              togglePlay();
                            } else {
                              playTrack(
                                item,
                                items.filter((i) => i.media_type === 'music')
                              );
                            }
                          }}
                          className="w-full h-full rounded-md flex items-center justify-center text-[var(--accent-text)] hover:bg-[var(--accent)] hover:text-white transition cursor-pointer"
                          title={currentTrack?.id === item.id && isPlaying ? 'Pause' : 'Play'}
                        >
                          {currentTrack?.id === item.id && isPlaying ? (
                            <Pause className="w-3.5 h-3.5" />
                          ) : (
                            <Play className="w-3.5 h-3.5 ml-0.5" />
                          )}
                        </button>
                      ) : item.media_type === 'image' ? (
                        <ImageIcon className="w-3.5 h-3.5" />
                      ) : (
                        <BookOpen className="w-3.5 h-3.5" />
                      )}
                    </div>

                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-xs sm:text-sm text-[var(--text)] truncate">
                          {item.name}
                        </span>
                        {currentTrack?.id === item.id && isPlaying && (
                          <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)] dot-running shrink-0" />
                        )}
                        <span className="text-[10px] text-[var(--muted)] capitalize">
                          {item.media_type}
                        </span>
                      </div>
                      <div className="text-[11px] text-[var(--muted)] truncate mt-0.5 flex items-center gap-2">
                        {item.description && <span>{item.description} •</span>}
                        <span className="font-mono text-[10px] text-[var(--muted)] opacity-70">
                          {getDisplayHost(item.url)}
                        </span>
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                    <a
                      href={item.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="p-1.5 rounded hover:bg-[var(--surface-hover)] text-[var(--muted)] hover:text-[var(--text)] transition"
                      aria-label={`Open ${item.name}`}
                      title="Open URL"
                    >
                      <ExternalLink className="w-3.5 h-3.5" />
                    </a>
                    <button
                      onClick={() => handleDeleteItem(item.id, item.name)}
                      className="p-1.5 rounded hover:bg-[var(--surface-hover)] text-[var(--muted)] hover:text-[var(--danger-text)] transition cursor-pointer"
                      aria-label={`Delete ${item.name}`}
                      title="Delete item"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              );
            })}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

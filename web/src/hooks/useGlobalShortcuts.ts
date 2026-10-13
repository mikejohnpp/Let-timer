import { useEffect } from 'react';

export interface ShortcutHandlers {
  onToggleTimer?: () => void; // Space
  onOpenStartPicker?: () => void; // 's'
  onNewTask?: () => void; // 'n'
}

export function useGlobalShortcuts(handlers: ShortcutHandlers) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 1. Modifiers: ignore if Ctrl, Meta, Alt, Shift are held (e.g. Ctrl+S)
      if (e.ctrlKey || e.metaKey || e.altKey) return;

      // 2. Repeat or IME composition
      if (e.repeat || e.isComposing) return;

      // 3. Target is an input, textarea, select, button, etc.
      const target = e.target as HTMLElement | null;
      if (target) {
        const tag = target.tagName.toLowerCase();
        if (
          tag === 'input' ||
          tag === 'textarea' ||
          tag === 'select' ||
          tag === 'button' ||
          tag === 'a' ||
          target.isContentEditable ||
          target.getAttribute('role') === 'menuitem' ||
          target.getAttribute('role') === 'tab' ||
          target.getAttribute('role') === 'checkbox' ||
          target.getAttribute('role') === 'switch'
        ) {
          return;
        }
      }

      // 4. Any open dialog, drawer, or popover
      if (document.querySelector('[role="dialog"], [data-state="open"]')) {
        return;
      }

      if (e.key === ' ' && handlers.onToggleTimer) {
        e.preventDefault();
        handlers.onToggleTimer();
      } else if ((e.key === 's' || e.key === 'S') && handlers.onOpenStartPicker) {
        e.preventDefault();
        handlers.onOpenStartPicker();
      } else if ((e.key === 'n' || e.key === 'N') && handlers.onNewTask) {
        e.preventDefault();
        handlers.onNewTask();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handlers]);
}


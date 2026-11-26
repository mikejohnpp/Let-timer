import React, { createContext, useContext, useState, useCallback } from 'react';
import * as ToastPrimitive from '@radix-ui/react-toast';
import { X, AlertCircle, CheckCircle, Info } from 'lucide-react';
import { clsx } from 'clsx';

export interface ToastItem {
  id: string;
  title?: string;
  description: string;
  tone?: 'info' | 'success' | 'danger';
  open: boolean;
}

interface ToastContextType {
  showToast: (msg: string, tone?: 'info' | 'success' | 'danger') => void;
}

const ToastContext = createContext<ToastContextType | null>(null);

export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const showToast = useCallback((description: string, tone: 'info' | 'success' | 'danger' = 'info') => {
    const id = Math.random().toString(36).slice(2);
    setToasts((prev) => [...prev, { id, description, tone, open: true }]);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      <ToastPrimitive.Provider swipeDirection="right" duration={4000}>
        {children}
        {toasts.map((t) => (
          <ToastPrimitive.Root
            key={t.id}
            open={t.open}
            onOpenChange={(open) => {
              if (!open) {
                setToasts((prev) =>
                  prev.map((item) => (item.id === t.id ? { ...item, open: false } : item))
                );
                // Safety cleanup in case animationend does not fire (e.g. headless/test environments)
                setTimeout(() => {
                  setToasts((prev) => prev.filter((item) => item.id !== t.id));
                }, 300);
              }
            }}
            onAnimationEnd={(e) => {
              if (!t.open || e.animationName.includes('fade-out') || e.animationName.includes('slide-out')) {
                setToasts((prev) => prev.filter((item) => item.id !== t.id));
              }
            }}
            className={clsx(
              'pointer-events-auto flex items-center justify-between gap-3 p-3.5 rounded-[var(--radius-card)] border backdrop-blur-md shadow-xl text-xs select-none',
              'data-[state=open]:animate-[slide-in-right_var(--dur-base)_var(--ease-out)]',
              'data-[state=closed]:animate-[fade-out_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]',
              'data-[swipe=move]:translate-x-[var(--radix-toast-swipe-move-x)]',
              'data-[swipe=cancel]:translate-x-0 data-[swipe=cancel]:transition-transform data-[swipe=cancel]:duration-[var(--dur-fast)] data-[swipe=cancel]:ease-[var(--ease-out)]',
              'data-[swipe=end]:animate-[slide-out-right_calc(var(--dur-base)*var(--exit-ratio))_var(--ease-out)_forwards]',
              t.tone === 'danger' && 'bg-[var(--danger-bg)]/90 text-[var(--danger-text)] border-[var(--danger)]/30',
              t.tone === 'success' && 'bg-[var(--success-bg)]/90 text-[var(--success-text)] border-[var(--success)]/20',
              (!t.tone || t.tone === 'info') && 'bg-[var(--surface)]/90 text-[var(--text)] border-[var(--border)]/40'
            )}
          >
            <div className="flex items-center gap-2">
              {t.tone === 'danger' && <AlertCircle className="w-4 h-4 text-[var(--danger)] shrink-0" />}
              {t.tone === 'success' && <CheckCircle className="w-4 h-4 text-[var(--success)] shrink-0" />}
              {(!t.tone || t.tone === 'info') && <Info className="w-4 h-4 text-[var(--accent-text)] shrink-0" />}
              <ToastPrimitive.Description>{t.description}</ToastPrimitive.Description>
            </div>
            <ToastPrimitive.Close asChild>
              <button
                className="p-1 rounded-[var(--radius-control)] text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface-hover)] transition cursor-pointer"
                aria-label="Close notification"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </ToastPrimitive.Close>
          </ToastPrimitive.Root>
        ))}
        <ToastPrimitive.Viewport className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 w-full max-w-sm outline-none pointer-events-none" />
      </ToastPrimitive.Provider>
    </ToastContext.Provider>
  );
};

export function useToast() {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error('useToast must be used within ToastProvider');
  return ctx;
}


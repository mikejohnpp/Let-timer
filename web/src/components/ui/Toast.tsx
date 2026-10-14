import React, { createContext, useContext, useState, useCallback } from 'react';
import * as ToastPrimitive from '@radix-ui/react-toast';
import { X, AlertCircle, CheckCircle, Info } from 'lucide-react';
import { clsx } from 'clsx';
import { motion, AnimatePresence } from 'motion/react';

export interface ToastItem {
  id: string;
  title?: string;
  description: string;
  tone?: 'info' | 'success' | 'danger';
}

interface ToastContextType {
  showToast: (msg: string, tone?: 'info' | 'success' | 'danger') => void;
}

const ToastContext = createContext<ToastContextType | null>(null);

import { useMotionTokens, useMotionTransition } from '../../motion';

export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const tokens = useMotionTokens();
  const enterTransition = useMotionTransition('base');
  const exitTransition = useMotionTransition('base', { exit: true });

  const showToast = useCallback((description: string, tone: 'info' | 'success' | 'danger' = 'info') => {
    const id = Math.random().toString(36).slice(2);
    setToasts((prev) => [...prev, { id, description, tone }]);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      <ToastPrimitive.Provider swipeDirection="right" duration={4000}>
        {children}
        <AnimatePresence initial={false}>
          {toasts.map((t) => (
            <motion.div
              key={t.id}
              layout
              initial={{ opacity: 0, y: tokens.isReduced ? 0 : tokens.travelS }}
              animate={{ opacity: 1, y: 0, transition: enterTransition }}
              exit={{ opacity: 0, height: 0, overflow: 'hidden', transition: exitTransition }}
            >
              <ToastPrimitive.Root
                key={t.id}
                onOpenChange={(open) => {
                  if (!open) setToasts((prev) => prev.filter((item) => item.id !== t.id));
                }}
                className={clsx(
                  'flex items-center justify-between gap-3 p-3.5 rounded-[var(--radius-control)] border shadow-[var(--shadow-popover)] text-xs',
                  t.tone === 'danger' && 'bg-[var(--danger-bg)] text-[var(--danger-text)] border-[var(--danger)]/30',
                  t.tone === 'success' && 'bg-[var(--success-bg)] text-[var(--success-text)] border-[var(--success)]/20',
                  (!t.tone || t.tone === 'info') && 'bg-[var(--surface)] text-[var(--text)] border-[var(--border)]'
                )}
              >
            <div className="flex items-center gap-2">
              {t.tone === 'danger' && <AlertCircle className="w-4 h-4 text-[var(--danger)] shrink-0" />}
              {t.tone === 'success' && <CheckCircle className="w-4 h-4 text-[var(--success)] shrink-0" />}
              {(!t.tone || t.tone === 'info') && <Info className="w-4 h-4 text-[var(--accent-text)] shrink-0" />}
              <ToastPrimitive.Description>{t.description}</ToastPrimitive.Description>
            </div>
            <ToastPrimitive.Close asChild>
              <button className="p-1 rounded text-[var(--muted)] hover:text-[var(--text)] cursor-pointer">
                <X className="w-3.5 h-3.5" />
              </button>
            </ToastPrimitive.Close>
          </ToastPrimitive.Root>
        </motion.div>
      ))}
    </AnimatePresence>
    <ToastPrimitive.Viewport className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 w-full max-w-sm outline-none" />
      </ToastPrimitive.Provider>
    </ToastContext.Provider>
  );
};

export function useToast() {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error('useToast must be used within ToastProvider');
  return ctx;
}


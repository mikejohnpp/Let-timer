import React from 'react';
import { Calendar, Target, Folder, Image, Settings } from 'lucide-react';
import { clsx } from 'clsx';
import type { AppView } from '../hooks/useHashRoute';

export interface BottomTabBarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  className?: string;
}

export const BottomTabBar: React.FC<BottomTabBarProps> = ({ currentView, onNavigate, className }) => {
  const tabs: { view: AppView; label: string; icon: React.ReactNode }[] = [
    { view: 'today', label: 'Today', icon: <Calendar className="w-5 h-5" /> },
    { view: 'focus', label: 'Focus', icon: <Target className="w-5 h-5" /> },
    { view: 'workspaces', label: 'Spaces', icon: <Folder className="w-5 h-5" /> },
    { view: 'media', label: 'Media', icon: <Image className="w-5 h-5" /> },
    { view: 'settings', label: 'Settings', icon: <Settings className="w-5 h-5" /> },
  ];

  return (
    <nav
      className={clsx(
        'lg:hidden fixed bottom-0 inset-x-0 z-40 bg-[var(--surface)] border-t border-[var(--border)] h-[var(--tabbar-h)] pb-[env(safe-area-inset-bottom)] flex items-center justify-around select-none shadow-sm',
        className
      )}
      aria-label="Bottom Navigation"
    >
      {tabs.map((t) => {
        const isActive = currentView === t.view;
        return (
          <button
            key={t.view}
            onClick={() => onNavigate(t.view)}
            aria-current={isActive ? 'page' : undefined}
            className={clsx(
              'flex flex-col items-center justify-center flex-1 h-full relative cursor-pointer transition',
              isActive ? 'text-[var(--accent-text)]' : 'text-[var(--muted)] hover:text-[var(--text)]'
            )}
          >
            {isActive && (
              <span className="absolute top-0 inset-x-4 h-[2px] bg-[var(--accent-text)] rounded-full" />
            )}
            {t.icon}
            <span className="text-[10px] font-medium mt-0.5">{t.label}</span>
          </button>
        );
      })}
    </nav>
  );
};


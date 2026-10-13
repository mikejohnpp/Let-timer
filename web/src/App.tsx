import React, { useState, useEffect, useCallback } from 'react';
import { Header } from './components/Header';
import { TimerBar } from './components/TimerBar';
import { StartPickerModal } from './components/StartPickerModal';
import { TodayView } from './views/TodayView';
import { FocusView } from './views/FocusView';
import { WorkspacesView } from './views/WorkspacesView';
import { MediaView } from './views/MediaView';
import { SettingsView } from './views/SettingsView';
import { LoginView } from './views/LoginView';
import { useEventStream } from './hooks/useEventStream';
import { getStoredToken } from './api/client';

export const App: React.FC = () => {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null);
  const [currentView, setCurrentView] = useState<'today' | 'focus' | 'workspaces' | 'media' | 'settings'>('today');
  const [isStartPickerOpen, setIsStartPickerOpen] = useState(false);

  const { connectionStatus, snapshot, receivedAt, refetchSnapshot } = useEventStream();

  const checkAuth = useCallback(async () => {
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/auth/me', { headers });
      if (res.ok) {
        setIsAuthenticated(true);
      } else if (res.status === 401) {
        setIsAuthenticated(false);
      } else {
        // Assume ok if server returns something else
        setIsAuthenticated(true);
      }
    } catch {
      // In offline / server boot mode, allow UI to load
      setIsAuthenticated(true);
    }
  }, []);

  useEffect(() => {
    checkAuth();

    const handleAuthExpired = () => setIsAuthenticated(false);
    window.addEventListener('let-timer-auth-expired', handleAuthExpired);
    return () => window.removeEventListener('let-timer-auth-expired', handleAuthExpired);
  }, [checkAuth]);

  // Global Keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') {
        return;
      }

      if (e.code === 'Space') {
        e.preventDefault();
        if (snapshot && snapshot.state === 'running') {
          fetch(`/api/v1/tasks/${snapshot.task.id}/pause`, {
            method: 'POST',
            headers: { 'X-Let-Timer-Protocol': '1' },
          }).then(refetchSnapshot);
        } else if (snapshot && snapshot.state === 'paused') {
          fetch(`/api/v1/tasks/${snapshot.task.id}/resume`, {
            method: 'POST',
            headers: { 'X-Let-Timer-Protocol': '1' },
          }).then(refetchSnapshot);
        }
      } else if (e.key === 's') {
        setIsStartPickerOpen(true);
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [snapshot, refetchSnapshot]);

  if (isAuthenticated === null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-[var(--bg)] text-[var(--muted)] text-sm">
        Connecting to Let-timer…
      </div>
    );
  }

  if (isAuthenticated === false) {
    return <LoginView onLoggedIn={() => setIsAuthenticated(true)} />;
  }

  return (
    <div className="min-h-screen flex flex-col bg-[var(--bg)] text-[var(--text)] transition-colors">
      <Header
        currentView={currentView}
        onNavigate={setCurrentView}
        snapshot={snapshot}
        connectionStatus={connectionStatus}
        onOpenStartPicker={() => setIsStartPickerOpen(true)}
      />

      <TimerBar
        snapshot={snapshot}
        receivedAt={receivedAt}
        onOpenFocus={() => setCurrentView('focus')}
        onOpenStartPicker={() => setIsStartPickerOpen(true)}
        onActionComplete={refetchSnapshot}
      />

      <main className="flex-1 pb-16">
        {currentView === 'today' && (
          <TodayView
            snapshot={snapshot}
            onRefreshTimer={refetchSnapshot}
            onOpenStartPicker={() => setIsStartPickerOpen(true)}
          />
        )}
        {currentView === 'focus' && (
          <FocusView
            snapshot={snapshot}
            receivedAt={receivedAt}
            onRefreshTimer={refetchSnapshot}
            onOpenStartPicker={() => setIsStartPickerOpen(true)}
          />
        )}
        {currentView === 'workspaces' && <WorkspacesView />}
        {currentView === 'media' && <MediaView />}
        {currentView === 'settings' && <SettingsView />}
      </main>

      <StartPickerModal
        isOpen={isStartPickerOpen}
        onClose={() => setIsStartPickerOpen(false)}
        snapshot={snapshot}
        onStarted={refetchSnapshot}
      />
    </div>
  );
};
export default App;

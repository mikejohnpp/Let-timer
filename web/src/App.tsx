import React, { useState, useEffect, useCallback } from 'react';
import { Header } from './components/Header';
import { TimerBar } from './components/TimerBar';
import { BottomTabBar } from './components/BottomTabBar';
import { StartPickerModal } from './components/StartPickerModal';
import { TodayView } from './views/TodayView';
import { FocusView } from './views/FocusView';
import { WorkspacesView } from './views/WorkspacesView';
import { MediaView } from './views/MediaView';
import { SettingsView } from './views/SettingsView';
import { LoginView } from './views/LoginView';
import { useEventStream } from './hooks/useEventStream';
import { useHashRoute } from './hooks/useHashRoute';
import { useGlobalShortcuts } from './hooks/useGlobalShortcuts';
import { getStoredToken } from './api/client';
import { ToastProvider, ConfirmProvider, ErrorState } from './components/ui';

type AuthStatus = 'checking' | 'login' | 'unreachable' | 'ok';

export const AppContent: React.FC = () => {
  const [authStatus, setAuthStatus] = useState<AuthStatus>('checking');
  const [route, navigate] = useHashRoute();
  const currentView = route.view;
  const [isStartPickerOpen, setIsStartPickerOpen] = useState(false);

  const { connectionStatus, snapshot, receivedAt, refetchSnapshot } = useEventStream();

  const checkAuth = useCallback(async () => {
    setAuthStatus('checking');
    try {
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch('/api/v1/auth/me', { headers });
      if (res.ok) {
        setAuthStatus('ok');
      } else if (res.status === 401) {
        setAuthStatus('login');
      } else {
        setAuthStatus('ok');
      }
    } catch {
      setAuthStatus('unreachable');
    }
  }, []);

  useEffect(() => {
    checkAuth();

    const handleAuthExpired = () => setAuthStatus('login');
    window.addEventListener('let-timer-auth-expired', handleAuthExpired);
    return () => window.removeEventListener('let-timer-auth-expired', handleAuthExpired);
  }, [checkAuth]);

  // Global Keyboard Shortcuts
  useGlobalShortcuts({
    onToggleTimer: () => {
      if (!snapshot || snapshot.state === 'idle') return;
      const action = snapshot.state === 'running' ? 'pause' : 'resume';
      const token = getStoredToken();
      const headers: Record<string, string> = { 'X-Let-Timer-Protocol': '1' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      fetch(`/api/v1/tasks/${snapshot.task.id}/${action}`, {
        method: 'POST',
        headers,
      }).then(refetchSnapshot);
    },
    onOpenStartPicker: () => setIsStartPickerOpen(true),
    onNewTask: () => {
      if (currentView !== 'today') {
        navigate('today');
      }
      window.dispatchEvent(new CustomEvent('let-timer-new-task'));
    },
  });

  if (authStatus === 'checking') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-[var(--bg)] text-[var(--muted)] text-sm">
        Connecting to Let-timer…
      </div>
    );
  }

  if (authStatus === 'unreachable') {
    return (
      <div className="min-h-screen flex items-center justify-center p-4 bg-[var(--bg)]">
        <ErrorState
          title="Cannot connect to Let-timer server"
          description="Make sure the daemon/server is running and reachable at this address."
          onRetry={checkAuth}
        />
      </div>
    );
  }

  if (authStatus === 'login') {
    return <LoginView onLoggedIn={() => setAuthStatus('ok')} />;
  }

  const isFocusView = currentView === 'focus';

  return (
    <div className="min-h-screen flex flex-col bg-[var(--bg)] text-[var(--text)] transition-colors">
      <Header
        currentView={currentView}
        onNavigate={navigate}
        snapshot={snapshot}
        connectionStatus={connectionStatus}
        onOpenStartPicker={() => setIsStartPickerOpen(true)}
      />

      {!isFocusView && (
        <TimerBar
          snapshot={snapshot}
          receivedAt={receivedAt}
          onOpenFocus={() => navigate('focus')}
          onOpenStartPicker={() => setIsStartPickerOpen(true)}
          onActionComplete={refetchSnapshot}
        />
      )}

      <main className={`flex-1 ${!isFocusView ? 'pb-20 lg:pb-12' : ''}`}>
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

      {!isFocusView && (
        <BottomTabBar
          currentView={currentView}
          onNavigate={navigate}
        />
      )}

      <StartPickerModal
        isOpen={isStartPickerOpen}
        onClose={() => setIsStartPickerOpen(false)}
        snapshot={snapshot}
        onStarted={refetchSnapshot}
      />
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <ToastProvider>
      <ConfirmProvider>
        <AppContent />
      </ConfirmProvider>
    </ToastProvider>
  );
};

export default App;

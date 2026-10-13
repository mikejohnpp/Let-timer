import React, { useState, useEffect, useCallback, useRef } from 'react';
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
import { useWallpaper } from './hooks/useWallpaper';
import { useIdleTimer } from './hooks/useIdleTimer';
import { apiFetch } from './api/client';
import { ToastProvider, ConfirmProvider, ErrorState } from './components/ui';
import { AudioPlayerProvider, useAudioPlayer } from './context/AudioPlayerContext';
import { FloatingPlayerIsland } from './components/FloatingPlayerIsland';

type AuthStatus = 'checking' | 'login' | 'unreachable' | 'ok';

export const AppContent: React.FC = () => {
  const [authStatus, setAuthStatus] = useState<AuthStatus>('checking');
  const [route, navigate] = useHashRoute();
  const currentView = route.view;
  const isFocusView = currentView === 'focus';
  const focusControlsVisible = useIdleTimer(3000, isFocusView);
  const [isStartPickerOpen, setIsStartPickerOpen] = useState(false);
  const { wallpaperUrl } = useWallpaper();

  const { connectionStatus, snapshot, receivedAt, refetchSnapshot } = useEventStream();
  const { currentTrack, pause, clearPlayer, loadMediaListForTask } = useAudioPlayer();
  const prevSnapshotStateRef = useRef<string>(snapshot?.state || 'idle');

  useEffect(() => {
    const prevState = prevSnapshotStateRef.current;
    const currentState = snapshot?.state || 'idle';

    if ((prevState === 'idle' || !prevState) && currentState === 'running') {
      // Dừng lại việc chơi nhạc khi task bắt đầu
      pause();
      if (snapshot?.task?.media_list_id) {
        loadMediaListForTask(snapshot.task.media_list_id);
      } else {
        clearPlayer();
      }
    } else if (currentState !== 'idle' && snapshot?.task?.media_list_id) {
      loadMediaListForTask(snapshot.task.media_list_id);
    }

    prevSnapshotStateRef.current = currentState;
  }, [snapshot?.state, snapshot?.task?.media_list_id, pause, clearPlayer, loadMediaListForTask]);

  const checkAuth = useCallback(async () => {
    setAuthStatus('checking');
    try {
      const res = await apiFetch('/api/v1/auth/me');
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
      apiFetch(`/api/v1/tasks/${snapshot.task.id}/${action}`, {
        method: 'POST',
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

  return (
    <div className="relative min-h-screen flex flex-col bg-[var(--bg)] text-[var(--text)] transition-colors">
      {wallpaperUrl && (
        <div
          className="fixed inset-0 pointer-events-none z-0 bg-cover bg-center transition-all duration-700"
          style={{ backgroundImage: `url(${wallpaperUrl})` }}
        >
          <div className="absolute inset-0 bg-[var(--bg-glass-overlay)] backdrop-blur-[2px]" />
        </div>
      )}

      <Header
        currentView={currentView}
        onNavigate={navigate}
        connectionStatus={connectionStatus}
      />

      <main className={`relative z-10 flex-1 ${!isFocusView ? 'pb-24 lg:pb-16' : ''}`}>
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

      {/* Floating Bottom Island (TimerBar if task active, or FloatingPlayerIsland if idle & music playing) */}
      {!isFocusView && (
        <>
          {snapshot && snapshot.state !== 'idle' ? (
            <div className="fixed bottom-20 lg:bottom-6 left-1/2 -translate-x-1/2 z-50 max-w-[92vw] animate-in fade-in slide-in-from-bottom-4 duration-300">
              <TimerBar
                snapshot={snapshot}
                receivedAt={receivedAt}
                onOpenFocus={() => navigate('focus')}
                onOpenStartPicker={() => setIsStartPickerOpen(true)}
                onActionComplete={refetchSnapshot}
              />
            </div>
          ) : currentTrack ? (
            <div className="fixed bottom-20 lg:bottom-6 left-1/2 -translate-x-1/2 z-50 max-w-[92vw] animate-in fade-in slide-in-from-bottom-4 duration-300">
              <FloatingPlayerIsland />
            </div>
          ) : null}
        </>
      )}

      <BottomTabBar
        currentView={currentView}
        onNavigate={navigate}
        className={
          isFocusView
            ? `transition-all duration-300 ${
                focusControlsVisible
                  ? 'opacity-100 translate-y-0 pointer-events-auto'
                  : 'opacity-0 translate-y-2 pointer-events-none'
              }`
            : ''
        }
      />

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
        <AudioPlayerProvider>
          <AppContent />
        </AudioPlayerProvider>
      </ConfirmProvider>
    </ToastProvider>
  );
};

export default App;

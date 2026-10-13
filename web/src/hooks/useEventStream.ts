import { useEffect, useState, useCallback, useRef } from 'react';
import type { components } from '../api/schema';
import { getStoredToken, apiFetch } from '../api/client';

export type TimerSnapshot = components['schemas']['TimerSnapshotDto'];

export interface UseEventStreamResult {
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  snapshot: TimerSnapshot | null;
  receivedAt: number;
  refetchSnapshot: () => Promise<void>;
}

export function useEventStream(): UseEventStreamResult {
  const [connectionStatus, setConnectionStatus] = useState<'connected' | 'connecting' | 'disconnected'>('connecting');
  const [snapshot, setSnapshot] = useState<TimerSnapshot | null>(null);
  const [receivedAt, setReceivedAt] = useState<number>(Date.now());
  const eventSourceRef = useRef<EventSource | null>(null);

  const fetchCurrentSnapshot = useCallback(async () => {
    try {
      const res = await apiFetch('/api/v1/tasks/current');
      if (res.ok) {
        const data: TimerSnapshot = await res.json();
        setSnapshot(data);
        setReceivedAt(Date.now());
      }
    } catch {
      // Ignore network errors during polling
    }
  }, []);

  useEffect(() => {
    let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
    let isMounted = true;

    function connect() {
      if (!isMounted) return;

      const token = getStoredToken();
      const url = token ? `/api/v1/events?token=${encodeURIComponent(token)}` : '/api/v1/events';

      setConnectionStatus('connecting');
      const es = new EventSource(url);
      eventSourceRef.current = es;

      es.onopen = () => {
        if (!isMounted) return;
        setConnectionStatus('connected');
      };

      es.addEventListener('snapshot', (event) => {
        if (!isMounted) return;
        try {
          const data: TimerSnapshot = JSON.parse(event.data);
          setSnapshot(data);
          setReceivedAt(Date.now());
        } catch (e) {
          console.error('Failed to parse snapshot SSE event', e);
        }
      });

      es.addEventListener('timer_changed', (event) => {
        if (!isMounted) return;
        try {
          const data: TimerSnapshot = JSON.parse(event.data);
          setSnapshot(data);
          setReceivedAt(Date.now());
        } catch (e) {
          console.error('Failed to parse timer_changed SSE event', e);
        }
      });

      es.addEventListener('timer_expired', (event) => {
        if (!isMounted) return;
        try {
          const data = JSON.parse(event.data);
          // Show browser notification if granted
          if ('Notification' in window && Notification.permission === 'granted') {
            new Notification("Time's up!", {
              body: `Estimate finished for task #${data.task_id}.`,
              icon: '/favicon.svg',
            });
          }
        } catch (e) {
          console.error('Failed to parse timer_expired SSE event', e);
        }
        fetchCurrentSnapshot();
      });

      es.addEventListener('workspace_changed', () => {
        if (!isMounted) return;
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
        fetchCurrentSnapshot();
      });

      es.addEventListener('resync', () => {
        if (!isMounted) return;
        fetchCurrentSnapshot();
      });

      es.onerror = () => {
        if (!isMounted) return;
        setConnectionStatus('disconnected');
        es.close();
        // Exponential backoff reconnect
        reconnectTimeout = setTimeout(connect, 3000);
      };
    }

    connect();

    // 30s background sync fallback
    const interval = setInterval(fetchCurrentSnapshot, 30000);

    return () => {
      isMounted = false;
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
      clearInterval(interval);
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, [fetchCurrentSnapshot]);

  return {
    connectionStatus,
    snapshot,
    receivedAt,
    refetchSnapshot: fetchCurrentSnapshot,
  };
}

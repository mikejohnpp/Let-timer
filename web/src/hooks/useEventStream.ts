import { useEffect, useState, useCallback, useRef } from 'react';
import type { components } from '../api/schema';
import { apiFetch } from '../api/client';

export type TimerSnapshot = components['schemas']['TimerSnapshotDto'];

export interface UseEventStreamResult {
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  snapshot: TimerSnapshot | null;
  receivedAt: number;
  refetchSnapshot: () => Promise<void>;
}

export interface SseMessage {
  event: string;
  data: string;
}

export class SseParser {
  private buffer = '';
  private currentEvent = 'message';
  private currentData: string[] = [];

  constructor(private onMessage: (msg: SseMessage) => void) {}

  public feed(chunk: string): void {
    this.buffer += chunk;
    let newlineIndex: number;

    while ((newlineIndex = this.buffer.indexOf('\n')) !== -1) {
      let line = this.buffer.slice(0, newlineIndex);
      this.buffer = this.buffer.slice(newlineIndex + 1);

      if (line.endsWith('\r')) {
        line = line.slice(0, -1);
      }

      if (line === '') {
        if (this.currentData.length > 0) {
          this.onMessage({
            event: this.currentEvent,
            data: this.currentData.join('\n'),
          });
        }
        this.currentEvent = 'message';
        this.currentData = [];
      } else if (line.startsWith(':')) {
        // Comment / ping, ignore
      } else if (line.startsWith('event:')) {
        this.currentEvent = line.slice(6).trimStart();
      } else if (line.startsWith('data:')) {
        this.currentData.push(line.slice(5).trimStart());
      }
    }
  }

  public reset(): void {
    this.buffer = '';
    this.currentEvent = 'message';
    this.currentData = [];
  }
}

export function useEventStream(): UseEventStreamResult {
  const [connectionStatus, setConnectionStatus] = useState<'connected' | 'connecting' | 'disconnected'>('connecting');
  const [snapshot, setSnapshot] = useState<TimerSnapshot | null>(null);
  const [receivedAt, setReceivedAt] = useState<number>(Date.now());
  const abortControllerRef = useRef<AbortController | null>(null);

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

  const handleSseMessage = useCallback((event: string, data: string) => {
    switch (event) {
      case 'snapshot':
      case 'timer_changed': {
        try {
          const parsed: TimerSnapshot = JSON.parse(data);
          setSnapshot(parsed);
          setReceivedAt(Date.now());
        } catch (e) {
          console.error(`Failed to parse ${event} SSE event`, e);
        }
        break;
      }
      case 'timer_expired': {
        try {
          const parsed = JSON.parse(data);
          if ('Notification' in window && Notification.permission === 'granted') {
            new Notification("Time's up!", {
              body: `Estimate finished for task #${parsed.task_id || parsed.task?.id}.`,
              icon: '/favicon.svg',
            });
          }
        } catch (e) {
          console.error('Failed to parse timer_expired SSE event', e);
        }
        fetchCurrentSnapshot();
        break;
      }
      case 'workspace_changed': {
        window.dispatchEvent(new CustomEvent('let-timer-workspace-changed'));
        fetchCurrentSnapshot();
        break;
      }
      case 'resync': {
        fetchCurrentSnapshot();
        break;
      }
      default:
        break;
    }
  }, [fetchCurrentSnapshot]);

  useEffect(() => {
    let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
    let isMounted = true;

    async function connect() {
      if (!isMounted) return;

      setConnectionStatus('connecting');
      const controller = new AbortController();
      abortControllerRef.current = controller;

      try {
        const res = await apiFetch('/api/v1/events', {
          signal: controller.signal,
          headers: {
            Accept: 'text/event-stream',
          },
        });

        if (!res.ok) {
          throw new Error(`SSE HTTP error: ${res.status}`);
        }

        if (!res.body) {
          throw new Error('No response body for SSE stream');
        }

        if (!isMounted) return;
        setConnectionStatus('connected');

        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        const parser = new SseParser((msg) => {
          if (!isMounted) return;
          handleSseMessage(msg.event, msg.data);
        });

        while (isMounted) {
          const { value, done } = await reader.read();
          if (done) break;
          if (value) {
            parser.feed(decoder.decode(value, { stream: true }));
          }
        }
      } catch (err: unknown) {
        if (controller.signal.aborted) {
          return;
        }
        console.warn('SSE stream disconnected, scheduling reconnect...', err);
      } finally {
        if (isMounted) {
          setConnectionStatus('disconnected');
          reconnectTimeout = setTimeout(connect, 3000);
        }
      }
    }

    connect();

    // 30s background sync fallback
    const interval = setInterval(fetchCurrentSnapshot, 30000);

    return () => {
      isMounted = false;
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
      clearInterval(interval);
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  }, [fetchCurrentSnapshot, handleSseMessage]);

  return {
    connectionStatus,
    snapshot,
    receivedAt,
    refetchSnapshot: fetchCurrentSnapshot,
  };
}

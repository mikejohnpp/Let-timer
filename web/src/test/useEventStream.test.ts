import { describe, it, expect } from 'vitest';
import { SseParser, type SseMessage } from '../hooks/useEventStream';

describe('SseParser', () => {
  it('parses standard SSE events with LF and CRLF line endings', () => {
    const received: SseMessage[] = [];
    const parser = new SseParser((msg) => received.push(msg));

    // LF
    parser.feed('event: snapshot\ndata: {"id":1}\n\n');
    expect(received).toEqual([{ event: 'snapshot', data: '{"id":1}' }]);

    // CRLF
    parser.feed('event: timer_changed\r\ndata: {"id":2}\r\n\r\n');
    expect(received).toHaveLength(2);
    expect(received[1]).toEqual({ event: 'timer_changed', data: '{"id":2}' });
  });

  it('correctly handles chunk boundaries split in the middle of a line or payload', () => {
    const received: SseMessage[] = [];
    const parser = new SseParser((msg) => received.push(msg));

    parser.feed('eve');
    parser.feed('nt: snap');
    parser.feed('shot\nda');
    parser.feed('ta: {"status":"running');
    parser.feed('"}\n\n');

    expect(received).toEqual([
      { event: 'snapshot', data: '{"status":"running"}' },
    ]);
  });

  it('ignores comments and keep-alive pings (: ping)', () => {
    const received: SseMessage[] = [];
    const parser = new SseParser((msg) => received.push(msg));

    parser.feed(': keep-alive ping\n\n');
    parser.feed(': another comment\r\n');
    parser.feed('event: resync\ndata: lagged\n\n');

    expect(received).toEqual([{ event: 'resync', data: 'lagged' }]);
  });

  it('joins multiple data lines with a newline according to SSE spec', () => {
    const received: SseMessage[] = [];
    const parser = new SseParser((msg) => received.push(msg));

    parser.feed('event: multi\ndata: line1\ndata: line2\ndata: line3\n\n');
    expect(received).toEqual([
      { event: 'multi', data: 'line1\nline2\nline3' },
    ]);
  });

  it('defaults event type to "message" when no event line is specified', () => {
    const received: SseMessage[] = [];
    const parser = new SseParser((msg) => received.push(msg));

    parser.feed('data: default event type\n\n');
    expect(received).toEqual([
      { event: 'message', data: 'default event type' },
    ]);
  });
});

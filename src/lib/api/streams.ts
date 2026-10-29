import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** One Server-Sent Event of an open stream. */
export interface StreamEvent {
  streamId: string;
  /** `message` unless the server named the event. */
  event: string;
  data: string;
  id: string | null;
  at: number;
}

export interface StreamEnd {
  streamId: string;
  error: string | null;
}

export function stopEventStream(id: string): Promise<void> {
  return invoke('stop_event_stream', { id });
}

export function onStreamEvent(handler: (event: StreamEvent) => void): Promise<UnlistenFn> {
  return listen<StreamEvent>('stream://event', (event) => {
    handler(event.payload);
  });
}

export function onStreamEnd(handler: (end: StreamEnd) => void): Promise<UnlistenFn> {
  return listen<StreamEnd>('stream://end', (event) => {
    handler(event.payload);
  });
}

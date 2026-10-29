import type { UnlistenFn } from '@tauri-apps/api/event';
import { onStreamEnd, onStreamEvent, stopEventStream, type StreamEvent } from '../api/streams';

/** The newest events stay on screen; older ones drop off. */
const MAX_EVENTS = 500;

/** The events of one open `text/event-stream` response, as they arrive. */
export class EventStreamView {
  readonly id: string;
  events = $state<StreamEvent[]>([]);
  /** Every event so far, including ones no longer kept on screen. */
  count = $state(0);
  ended = $state(false);
  error = $state<string | null>(null);
  #unlisten: UnlistenFn[] = [];

  constructor(id: string) {
    this.id = id;
  }

  async listen(): Promise<void> {
    this.#unlisten = await Promise.all([
      onStreamEvent((event) => {
        if (event.streamId === this.id) {
          this.count += 1;
          this.events = [...this.events, event].slice(-MAX_EVENTS);
        }
      }),
      onStreamEnd((end) => {
        if (end.streamId === this.id) {
          this.ended = true;
          this.error = end.error;
          this.dispose();
        }
      }),
    ]);
  }

  async stop(): Promise<void> {
    if (!this.ended) {
      this.ended = true;
      await stopEventStream(this.id);
    }
    this.dispose();
  }

  dispose(): void {
    for (const unlisten of this.#unlisten) {
      unlisten();
    }
    this.#unlisten = [];
  }
}

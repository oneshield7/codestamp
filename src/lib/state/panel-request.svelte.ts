export type RequestedPanel = 'variables' | 'history';

/** A collection panel asked for from elsewhere, such as Variables from a notice. */
class PanelRequest {
  pending = $state<{ collectionId: string; panel: RequestedPanel } | null>(null);

  open(collectionId: string, panel: RequestedPanel): void {
    this.pending = { collectionId, panel };
  }

  /** The panel asked for this collection, once; null when there is none. */
  take(collectionId: string): RequestedPanel | null {
    const pending = this.pending;
    if (pending?.collectionId !== collectionId) {
      return null;
    }
    this.pending = null;
    return pending.panel;
  }
}

export const panelRequest = new PanelRequest();

import * as api from '../api/trash';
import { notices } from './notices.svelte';

/** Recently deleted: the kept copies, and the Undo offered right after a delete. */
class RecentlyDeleted {
  items = $state.raw<api.DeletedItem[]>([]);
  loaded = $state(false);
  /**
   * Set by the app: reloads what a restore brought back, and opens it when `open` is true (an
   * Undo right after deleting) rather than leaving the user on Recently deleted.
   */
  onRestored: (restored: api.RestoredItem, open: boolean) => Promise<void> = () =>
    Promise.resolve();

  /** The banner after a delete, with Undo. */
  announce(item: api.DeletedItem): void {
    const days = String(api.KEEP_DAYS);
    notices.inform(`“${item.name}” was deleted. It stays in Recently deleted for ${days} days.`, {
      label: 'Undo',
      run: () => void this.restore(item, true),
    });
  }

  async load(): Promise<void> {
    await notices.guard(async () => {
      this.items = await api.listDeleted();
      this.loaded = true;
    });
  }

  async restore(item: api.DeletedItem, open: boolean): Promise<void> {
    await notices.guard(async () => {
      const restored = await api.restoreDeleted(item.id);
      this.items = this.items.filter((entry) => entry.id !== item.id);
      await this.onRestored(restored, open);
      if (!open) {
        notices.inform(`“${item.name}” is back.`, {
          label: 'Open',
          run: () => void this.onRestored(restored, true),
        });
      }
    });
  }

  async forget(id: string): Promise<void> {
    await notices.guard(async () => {
      await api.forgetDeleted(id);
      this.items = this.items.filter((entry) => entry.id !== id);
    });
  }

  async forgetAll(): Promise<void> {
    await notices.guard(async () => {
      await api.forgetAllDeleted();
      this.items = [];
    });
  }
}

export const recentlyDeleted = new RecentlyDeleted();

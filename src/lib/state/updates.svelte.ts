import { toAppError } from '../api/http';
import * as api from '../api/update';

export type AvailableUpdate = Extract<api.UpdateCheck, { kind: 'available' }>;

/** Checks for a newer Codestamp and installs it; one instance for the banner and Settings. */
export class Updates {
  result = $state<api.UpdateCheck | null>(null);
  checking = $state(false);
  checkError = $state<string | null>(null);
  installing = $state(false);
  installError = $state<string | null>(null);
  progress = $state<api.DownloadProgress | null>(null);
  /** Hides the banner until the next start; Settings › About still offers the update. */
  dismissed = $state(false);

  readonly available = $derived<AvailableUpdate | null>(
    this.result?.kind === 'available' ? this.result : null,
  );

  /** A quiet check (at start) leaves no error behind: being offline is not news. */
  async check(quiet = false): Promise<void> {
    if (this.checking || this.installing) {
      return;
    }
    this.checking = true;
    this.checkError = null;
    // A failed install belongs to the update found before; the new answer replaces it.
    this.installError = null;
    try {
      this.result = await api.checkForUpdate();
    } catch (error) {
      if (!quiet) {
        this.checkError = toAppError(error).message;
      }
    } finally {
      this.checking = false;
    }
  }

  /** `readyToClose` saves pending edits and names what still blocks closing Codestamp. */
  async install(readyToClose: () => Promise<string | null>): Promise<void> {
    if (!this.available || this.installing) {
      return;
    }
    this.installError = await readyToClose();
    if (this.installError) {
      return;
    }
    this.installing = true;
    this.progress = { received: 0, total: null };
    try {
      await api.installUpdate((progress) => {
        this.progress = progress;
      });
    } catch (error) {
      this.installError = toAppError(error).message;
    } finally {
      this.installing = false;
    }
  }
}

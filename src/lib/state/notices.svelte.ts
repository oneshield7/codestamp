import { toAppError } from '../api/http';

export type NoticeTone = 'error' | 'info';

export interface NoticeAction {
  label: string;
  run: () => void;
}

/** One place that turns failed operations (and news the user should see) into the top banner. */
class Notices {
  message = $state<string | null>(null);
  tone = $state<NoticeTone>('error');
  action = $state<NoticeAction | null>(null);

  report(error: unknown): void {
    this.message = toAppError(error).message;
    this.tone = 'error';
    this.action = null;
  }

  inform(message: string, action: NoticeAction | null = null): void {
    this.message = message;
    this.tone = 'info';
    this.action = action;
  }

  dismiss(): void {
    this.message = null;
    this.action = null;
  }

  async guard(task: () => Promise<void>): Promise<void> {
    try {
      await task();
    } catch (error) {
      this.report(error);
    }
  }
}

export const notices = new Notices();

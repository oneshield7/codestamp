export type StatusTone = 'ok' | 'err' | 'neutral';

export function statusTone(status: number): StatusTone {
  if (status >= 200 && status < 300) {
    return 'ok';
  }
  return status >= 400 ? 'err' : 'neutral';
}

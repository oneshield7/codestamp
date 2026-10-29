const MINUTE_MS = 60 * 1000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

export function formatClock(timestamp: number, withSeconds = false): string {
  return new Date(timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    ...(withSeconds ? { second: '2-digit' } : {}),
  });
}

export function formatIso(timestamp: number): string {
  return new Date(timestamp).toISOString();
}

export function formatDateTime(timestamp: number): string {
  return new Date(timestamp).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' });
}

/** A cookie's expiry (Unix seconds); a cookie without one lasts for the session. */
export function formatExpiry(seconds: number | null): string {
  return seconds === null ? 'Session' : formatDateTime(seconds * 1000);
}

export function dayLabel(timestamp: number, now: number = Date.now()): string {
  const daysAgo = Math.round((startOfDay(now) - startOfDay(timestamp)) / DAY_MS);
  if (daysAgo === 0) {
    return 'Today';
  }
  if (daysAgo === 1) {
    return 'Yesterday';
  }
  return new Date(timestamp).toLocaleDateString([], {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}

function startOfDay(timestamp: number): number {
  const date = new Date(timestamp);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

/** "just now", "5 min ago", "today at 14:05", or a date. */
export function formatSince(timestamp: number, now: number = Date.now()): string {
  const elapsed = now - timestamp;
  if (elapsed < MINUTE_MS) {
    return 'just now';
  }
  if (elapsed < HOUR_MS) {
    return `${String(Math.floor(elapsed / MINUTE_MS))} min ago`;
  }
  if (dayLabel(timestamp, now) === 'Today') {
    return `today at ${formatClock(timestamp)}`;
  }
  return formatDateTime(timestamp);
}

/** "in 5 min", "in 2 h", "in 3 days"; null once the time has passed. */
export function formatUntil(timestamp: number, now: number = Date.now()): string | null {
  const left = timestamp - now;
  if (left <= 0) {
    return null;
  }
  // Rounded before choosing the unit, so 59.6 minutes reads "in 1 h", not "in 60 min".
  const minutes = Math.max(1, Math.round(left / MINUTE_MS));
  if (minutes < 60) {
    return `in ${String(minutes)} min`;
  }
  const hours = Math.round(left / HOUR_MS);
  if (hours < 24) {
    return `in ${String(hours)} h`;
  }
  return `in ${String(Math.round(left / DAY_MS))} days`;
}

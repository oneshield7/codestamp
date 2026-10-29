const BYTE_UNITS = ['B', 'KB', 'MB', 'GB'] as const;

export function formatBytes(bytes: number): string {
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < BYTE_UNITS.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const unit = BYTE_UNITS.at(unitIndex) ?? 'B';
  return unitIndex === 0 ? `${String(value)} ${unit}` : `${value.toFixed(1)} ${unit}`;
}

export function formatDuration(milliseconds: number): string {
  return milliseconds < 1000
    ? `${String(milliseconds)} ms`
    : `${(milliseconds / 1000).toFixed(2)} s`;
}

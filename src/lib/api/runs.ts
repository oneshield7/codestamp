import { invoke } from '@tauri-apps/api/core';

/** A run's data file: one pass of the run per row, its columns as variables. */
export interface RunData {
  /** The file name only. */
  name: string;
  columns: string[];
  /** One value per column, in column order. */
  rows: string[][];
}

/** Opens the system dialog for a CSV or JSON file; null when the user cancels. */
export function pickRunData(): Promise<RunData | null> {
  return invoke('pick_run_data');
}

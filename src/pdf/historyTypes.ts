// Cap undo snapshots so very large PDFs don't accumulate unbounded working copies.
export const MAX_UNDO_HISTORY = 50;
// Above this size, per-edit snapshots store compact binary deltas instead of full copies.
export const SNAPSHOT_BYTE_LIMIT = 32 * 1024 * 1024;

export interface HistorySnapshot {
  kind: 'full' | 'delta';
  path: string;
  baseIndex?: number;
  size: number;
}

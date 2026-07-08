import type { PageRangeScope, ResolvedPageRange } from './types';

export function resolvePageRange(
  scope: PageRangeScope,
  start: number,
  end: number,
  currentPage: number,
  pageCount: number | null,
): ResolvedPageRange {
  if (scope === 'current') return { start: currentPage, end: currentPage };
  if (scope === 'all') return { start: 0, end: Math.max(0, (pageCount ?? 1) - 1) };
  return { start, end };
}

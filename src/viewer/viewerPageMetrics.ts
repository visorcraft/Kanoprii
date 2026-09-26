import { VIEWER_PAGE_H, VIEWER_PAGE_W, viewerPageSize } from '../app/constants';
import type { PdfPageSize } from '../app/types';

/** Active page viewer bitmap size — kept in sync for coordinate helpers. */
let activeW = VIEWER_PAGE_W;
let activeH = VIEWER_PAGE_H;

export function syncViewerPageMetrics(size?: PdfPageSize): { w: number; h: number } {
  const next = viewerPageSize(size);
  activeW = next.w;
  activeH = next.h;
  return next;
}

export function activeViewerPageW(): number {
  return activeW;
}

export function activeViewerPageH(): number {
  return activeH;
}

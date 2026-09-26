import type { RefObject } from 'react';
import { activeViewerPageH, activeViewerPageW } from './viewerPageMetrics';

/** Map a viewport click to natural (unscaled) image pixels. */
export function getImageCoords(
  imgRef: RefObject<HTMLImageElement | null>,
  clientX: number,
  clientY: number,
): { x: number; y: number } {
  if (!imgRef.current) return { x: 0, y: 0 };
  const b = imgRef.current.getBoundingClientRect();
  if (b.width <= 0 || b.height <= 0) return { x: 0, y: 0 };
  const pageW = activeViewerPageW();
  const pageH = activeViewerPageH();
  return {
    x: Math.max(0, Math.min(pageW, (clientX - b.left) * (pageW / b.width))),
    y: Math.max(0, Math.min(pageH, (clientY - b.top) * (pageH / b.height))),
  };
}

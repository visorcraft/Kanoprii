import type { RefObject } from 'react';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';

/** Map a viewport click to natural (unscaled) image pixels. */
export function getImageCoords(
  imgRef: RefObject<HTMLImageElement | null>,
  clientX: number,
  clientY: number,
): { x: number; y: number } {
  if (!imgRef.current) return { x: 0, y: 0 };
  const b = imgRef.current.getBoundingClientRect();
  return {
    x: (clientX - b.left) * (VIEWER_PAGE_W / b.width),
    y: (clientY - b.top) * (VIEWER_PAGE_H / b.height),
  };
}

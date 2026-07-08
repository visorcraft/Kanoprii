import type { RefObject } from 'react';

/** Map a viewport click to natural (unscaled) image pixels. */
export function getImageCoords(
  imgRef: RefObject<HTMLImageElement | null>,
  zoom: number,
  clientX: number,
  clientY: number,
): { x: number; y: number } {
  if (!imgRef.current) return { x: 0, y: 0 };
  const b = imgRef.current.getBoundingClientRect();
  return {
    x: (clientX - b.left) / zoom,
    y: (clientY - b.top) / zoom,
  };
}

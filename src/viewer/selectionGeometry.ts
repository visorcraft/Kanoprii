import type { Rect, ShapeLineGeometry } from '../app/usePdfEditState';
import { activeViewerPageH, activeViewerPageW } from './viewerPageMetrics';

export type ResizeHandle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';

export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function moveRectWithinPage(rect: Rect, x: number, y: number): Rect {
  const pageW = activeViewerPageW();
  const pageH = activeViewerPageH();
  return {
    x: clamp(x, 0, Math.max(0, pageW - rect.w)),
    y: clamp(y, 0, Math.max(0, pageH - rect.h)),
    w: rect.w,
    h: rect.h,
  };
}

export function resizeRectWithinPage(
  start: Rect,
  handle: ResizeHandle,
  dx: number,
  dy: number,
  minSize: number,
): Rect {
  const pageW = activeViewerPageW();
  const pageH = activeViewerPageH();
  let left = start.x;
  let top = start.y;
  let right = start.x + start.w;
  let bottom = start.y + start.h;

  if (handle.includes('w')) left = clamp(left + dx, 0, right - minSize);
  if (handle.includes('e')) right = clamp(right + dx, left + minSize, pageW);
  if (handle.includes('n')) top = clamp(top + dy, 0, bottom - minSize);
  if (handle.includes('s')) bottom = clamp(bottom + dy, top + minSize, pageH);
  return { x: left, y: top, w: right - left, h: bottom - top };
}

export function moveLineWithinPage(line: ShapeLineGeometry, dx: number, dy: number): ShapeLineGeometry {
  const pageW = activeViewerPageW();
  const pageH = activeViewerPageH();
  const minX = Math.min(line.x1, line.x2);
  const maxX = Math.max(line.x1, line.x2);
  const minY = Math.min(line.y1, line.y2);
  const maxY = Math.max(line.y1, line.y2);
  const clampedDx = clamp(dx, -minX, pageW - maxX);
  const clampedDy = clamp(dy, -minY, pageH - maxY);
  return {
    x1: line.x1 + clampedDx,
    y1: line.y1 + clampedDy,
    x2: line.x2 + clampedDx,
    y2: line.y2 + clampedDy,
  };
}

export function moveLineEndpoint(
  line: ShapeLineGeometry,
  endpoint: 'start' | 'end',
  dx: number,
  dy: number,
): ShapeLineGeometry {
  const pageW = activeViewerPageW();
  const pageH = activeViewerPageH();
  const next = endpoint === 'start'
    ? {
      ...line,
      x1: clamp(line.x1 + dx, 0, pageW),
      y1: clamp(line.y1 + dy, 0, pageH),
    }
    : {
        ...line,
        x2: clamp(line.x2 + dx, 0, pageW),
        y2: clamp(line.y2 + dy, 0, pageH),
      };
  return Math.hypot(next.x2 - next.x1, next.y2 - next.y1) >= 5 ? next : line;
}

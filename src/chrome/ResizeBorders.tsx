import { useEffect, useRef } from 'react';
import { getCurrentWindow, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/window';
import { isTauriRuntime } from '../app/tauriRuntime';

type ResizeDirection = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

declare global {
  interface Window {
    __kanopriiStartResize?: (
      direction: ResizeDirection,
      e: React.MouseEvent,
    ) => void;
  }
}

export function ResizeBorders() {
  const isTauri = isTauriRuntime();
  const resizingRef = useRef<{
    direction: ResizeDirection;
    startX: number;
    startY: number;
    startWidth: number;
    startHeight: number;
    startLeft: number;
    startTop: number;
  } | null>(null);

  useEffect(() => {
    if (!isTauri) return;
    const win = getCurrentWindow();

    const onMouseMove = async (e: MouseEvent) => {
      if (!resizingRef.current) return;
      e.preventDefault();

      const { direction, startX, startY, startWidth, startHeight, startLeft, startTop } = resizingRef.current;
      const dx = e.screenX - startX;
      const dy = e.screenY - startY;

      let newWidth = startWidth;
      let newHeight = startHeight;
      let newLeft = startLeft;
      let newTop = startTop;

      if (direction.includes('e')) newWidth = Math.max(400, startWidth + dx);
      if (direction.includes('s')) newHeight = Math.max(300, startHeight + dy);
      if (direction.includes('w')) {
        newWidth = Math.max(400, startWidth - dx);
        newLeft = startLeft + (startWidth - newWidth);
      }
      if (direction.includes('n')) {
        newHeight = Math.max(300, startHeight - dy);
        newTop = startTop + (startHeight - newHeight);
      }

      await win.setPosition(new PhysicalPosition(Math.round(newLeft), Math.round(newTop)));
      await win.setSize(new PhysicalSize(Math.round(newWidth), Math.round(newHeight)));
    };

    const onMouseUp = () => {
      resizingRef.current = null;
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };

    const startResize = async (direction: ResizeDirection, e: React.MouseEvent) => {
      e.preventDefault();
      const pos = await win.outerPosition();
      const size = await win.outerSize();
      resizingRef.current = {
        direction,
        startX: e.screenX,
        startY: e.screenY,
        startWidth: size.width,
        startHeight: size.height,
        startLeft: pos.x,
        startTop: pos.y,
      };
      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    };

    // Store the startResize function on the window for the JSX handlers
    window.__kanopriiStartResize = startResize;

    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      delete window.__kanopriiStartResize;
    };
  }, [isTauri]);

  if (!isTauri) return null;

  const handleMouseDown = (direction: ResizeDirection) => (e: React.MouseEvent) => {
    window.__kanopriiStartResize?.(direction, e);
  };

  return (
    <>
      <div className="resize-edge top" onMouseDown={handleMouseDown('n')} />
      <div className="resize-edge bottom" onMouseDown={handleMouseDown('s')} />
      <div className="resize-edge left" onMouseDown={handleMouseDown('w')} />
      <div className="resize-edge right" onMouseDown={handleMouseDown('e')} />
      <div className="resize-edge top-left" onMouseDown={handleMouseDown('nw')} />
      <div className="resize-edge top-right" onMouseDown={handleMouseDown('ne')} />
      <div className="resize-edge bottom-left" onMouseDown={handleMouseDown('sw')} />
      <div className="resize-edge bottom-right" onMouseDown={handleMouseDown('se')} />
    </>
  );
}

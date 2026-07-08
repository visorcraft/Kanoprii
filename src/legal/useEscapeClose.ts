import { useEffect, useRef } from 'react';

export function useEscapeClose(onClose: () => void, restoreFocus?: boolean) {
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    const previous = document.activeElement;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCloseRef.current();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      if (restoreFocus && previous instanceof HTMLElement) {
        previous.focus();
      }
    };
  }, [restoreFocus]);
}

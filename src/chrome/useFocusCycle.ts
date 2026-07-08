import { useEffect } from 'react';

const PANES = ['.menu-bar', '.quick-toolbar', '.sidebar', '.viewer-main'];

export function useFocusCycle(enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'F6') return;
      e.preventDefault();
      const anyModalOpen =
        document.querySelector('.modal-backdrop, .command-palette-backdrop') !==
        null;
      if (anyModalOpen) return;
      const active = document.activeElement;
      let currentIndex = -1;
      for (let i = 0; i < PANES.length; i++) {
        const pane = document.querySelector(PANES[i]);
        if (pane && pane.contains(active)) {
          currentIndex = i;
          break;
        }
      }
      const nextIndex = (currentIndex + 1) % PANES.length;
      for (let offset = 0; offset < PANES.length; offset++) {
        const idx = (nextIndex + offset) % PANES.length;
        const pane = document.querySelector<HTMLElement>(PANES[idx]);
        const focusable = pane?.querySelector<HTMLElement>(
          'button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
        );
        if (focusable) {
          focusable.focus();
          return;
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [enabled]);
}

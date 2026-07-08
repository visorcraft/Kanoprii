import { useRef, type WheelEvent } from 'react';
import { WHEEL_NAV_COOLDOWN } from '../app/constants';
import type { ScrollViewMode, ViewMode } from '../app/types';

type UseWheelNavigationOptions = {
  pageCount: number | null;
  viewMode: ViewMode;
  scrollViewMode: ScrollViewMode;
  currentPage: number;
  goToPage: (index: number) => void;
};

export function useWheelNavigation({
  pageCount,
  viewMode,
  scrollViewMode,
  currentPage,
  goToPage,
}: UseWheelNavigationOptions) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const pendingScrollRef = useRef<'top' | 'bottom' | null>(null);
  const lastWheelNavRef = useRef(0);

  const handleWheel = (e: WheelEvent) => {
    if (scrollViewMode === 'continuous') return;
    const el = scrollRef.current;
    if (!el || pageCount === null || viewMode !== 'pdf') return;

    const atTop = el.scrollTop <= 0;
    const atBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 1;
    const now = Date.now();
    if (now - lastWheelNavRef.current < WHEEL_NAV_COOLDOWN) return;

    if (e.deltaY > 0 && atBottom && currentPage < pageCount - 1) {
      lastWheelNavRef.current = now;
      pendingScrollRef.current = 'top';
      goToPage(currentPage + 1);
    } else if (e.deltaY < 0 && atTop && currentPage > 0) {
      lastWheelNavRef.current = now;
      pendingScrollRef.current = 'bottom';
      goToPage(currentPage - 1);
    }
  };

  const handleImageLoad = () => {
    const el = scrollRef.current;
    if (!el || pendingScrollRef.current === null) return;
    el.scrollTop = pendingScrollRef.current === 'bottom' ? el.scrollHeight : 0;
    pendingScrollRef.current = null;
  };

  return { scrollRef, handleWheel, handleImageLoad };
}

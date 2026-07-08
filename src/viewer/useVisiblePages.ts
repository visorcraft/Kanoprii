import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from 'react';
import type { PdfPageSize } from '../app/types';
import { PDF_BASE_HEIGHT, PDF_BASE_WIDTH } from '../pdf/usePdfDocument';

type UseVisiblePagesOptions = {
  scrollRef: RefObject<HTMLDivElement | null>;
  pageCount: number | null;
  pageSizes: PdfPageSize[];
  zoom: number;
  onCurrentPageChange: (page: number) => void;
  documentKey?: string;
};

const DEFAULT_ASPECT = PDF_BASE_HEIGHT / PDF_BASE_WIDTH;

export function useVisiblePages({
  scrollRef,
  pageCount,
  pageSizes,
  zoom,
  onCurrentPageChange,
  documentKey = '',
}: UseVisiblePagesOptions) {
  const pageRefs = useRef(new Map<number, HTMLDivElement>());
  const [visible, setVisible] = useState<Set<number>>(() => new Set([0]));
  const scrollToPageRef = useRef<(page: number) => void>(() => {});

  // Refs mirror the latest state so the long-lived scroll handler (registered
  // once below) always reads fresh values without re-subscribing on every
  // change - re-subscribing would churn the IntersectionObserver and scroll
  // listener, which is what made large documents seize up during scroll.
  const visibleRef = useRef(visible);
  visibleRef.current = visible;
  const onCurrentPageChangeRef = useRef(onCurrentPageChange);
  onCurrentPageChangeRef.current = onCurrentPageChange;

  const placeholderHeight = useCallback(
    (page: number) => {
      const size = pageSizes[page];
      const aspect = size && size.width > 0 ? size.height / size.width : DEFAULT_ASPECT;
      return PDF_BASE_WIDTH * aspect * zoom + 24;
    },
    [pageSizes, zoom],
  );

  const observerRef = useRef<IntersectionObserver | null>(null);

  // Single stable ref callback. The page index is read from the element's
  // dataset instead of being closed over, which keeps the callback identity
  // stable. With an inline arrow (`ref={(el) => registerPageRef(page, el)}`)
  // React treats the callback as new on every render and unobserves/re-observes
  // every page slot every render - O(pages) churn that freezes large docs.
  const registerPageRef = useCallback((el: HTMLDivElement | null) => {
    if (!el) return;
    const page = Number(el.dataset.pageIndex);
    if (Number.isNaN(page)) return;
    const observer = observerRef.current;
    const prev = pageRefs.current.get(page);
    if (prev && prev !== el && observer) observer.unobserve(prev);
    pageRefs.current.set(page, el);
    observer?.observe(el);
  }, []);

  const scrollToPage = useCallback((page: number) => {
    const el = pageRefs.current.get(page);
    el?.scrollIntoView({ block: 'start', behavior: 'smooth' });
  }, []);

  scrollToPageRef.current = scrollToPage;

  useEffect(() => {
    const root = scrollRef.current;
    if (!root || pageCount === null || pageCount <= 0) {
      // No document shown: drop any page elements held over from a previous
      // document so detached DOM nodes can be garbage collected.
      pageRefs.current.clear();
      return;
    }

    // Prune entries for pages that no longer exist. Page slots unmount when
    // pageCount shrinks (delete/merge pages), but with a stable ref callback
    // React delivers ref(null) on unmount without the page index, so we clean
    // up here by range instead of leaking detached elements + observations.
    for (const page of pageRefs.current.keys()) {
      if (page >= pageCount) pageRefs.current.delete(page);
    }

    const observer = new IntersectionObserver(
      (entries) => {
        setVisible((prev) => {
          const next = new Set(prev);
          let changed = false;
          for (const entry of entries) {
            const page = Number((entry.target as HTMLElement).dataset.pageIndex);
            if (Number.isNaN(page)) continue;
            if (entry.isIntersecting) {
              if (!next.has(page)) changed = true;
              next.add(page);
            } else if (next.delete(page)) {
              changed = true;
            }
          }
          return changed ? next : prev;
        });
      },
      { root, rootMargin: '200px 0px', threshold: 0.01 },
    );
    observerRef.current = observer;
    for (const el of pageRefs.current.values()) {
      observer.observe(el);
    }

    let lastReported = -1;
    let rafId: number | null = null;

    const compute = () => {
      rafId = null;
      const rootRect = root.getBoundingClientRect();
      const centerY = rootRect.top + rootRect.height / 2;
      let best = -1;
      let bestDist = Number.POSITIVE_INFINITY;
      // Only score the handful of currently-visible candidates instead of
      // measuring every page in the document - getBoundingClientRect() per
      // page across hundreds/thousands of slots on every scroll tick is what
      // froze scrolling and piled on forced layout work over time.
      for (const page of visibleRef.current) {
        const el = pageRefs.current.get(page);
        if (!el) continue;
        const rect = el.getBoundingClientRect();
        const pageCenter = rect.top + rect.height / 2;
        const dist = Math.abs(pageCenter - centerY);
        if (dist < bestDist) {
          bestDist = dist;
          best = page;
        }
      }
      // Only notify when the centered page actually changes so we don't trigger
      // a re-render (setCurrentPage + setPageInput) on every scroll frame.
      if (best >= 0 && best !== lastReported) {
        lastReported = best;
        onCurrentPageChangeRef.current(best);
      }
    };

    const onScroll = () => {
      // Coalesce scroll bursts to one geometry pass per animation frame.
      if (rafId !== null) return;
      rafId = requestAnimationFrame(compute);
    };

    root.addEventListener('scroll', onScroll, { passive: true });
    compute();

    const pageRefsAtCleanup = pageRefs.current;
    return () => {
      if (rafId !== null) cancelAnimationFrame(rafId);
      observer.disconnect();
      observerRef.current = null;
      root.removeEventListener('scroll', onScroll);
      pageRefsAtCleanup.clear();
    };
  }, [pageCount, scrollRef, documentKey]);

  // Memoized so the Set identity stays stable across renders that don't change
  // which pages are visible. Without this, ContinuousViewer's effect (which
  // depends on renderPages) fired on every render and re-poked the render
  // queue for every visible page each time.
  const renderPages = useMemo(() => {
    const out = new Set<number>();
    for (const page of visible) {
      out.add(page);
      if (page > 0) out.add(page - 1);
      if (pageCount !== null && page < pageCount - 1) out.add(page + 1);
    }
    return out;
  }, [visible, pageCount]);

  return {
    visible,
    renderPages,
    placeholderHeight,
    registerPageRef,
    scrollToPage,
    scrollToPageRef,
  };
}

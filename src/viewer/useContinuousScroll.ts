import { useCallback, useMemo, type RefObject } from 'react';
import type { PdfPageSize, WorkspaceViewMode } from '../app/types';
import { usePageRenderQueue } from '../pdf/usePageRenderQueue';
import { useVisiblePages } from './useVisiblePages';

type UseContinuousScrollOptions = {
  filePath: string;
  pdfRevision: number;
  pageCount: number | null;
  pageSizes: PdfPageSize[];
  zoom: number;
  scrollRef: RefObject<HTMLDivElement | null>;
  setCurrentPage: (page: number) => void;
  setPageInput: (value: string) => void;
  showHiddenLayers: boolean;
  workspaceView: WorkspaceViewMode;
};

export function useContinuousScroll(opts: UseContinuousScrollOptions) {
  const { requestPage, getPageUrl } = usePageRenderQueue(opts.filePath, opts.pdfRevision, opts.showHiddenLayers);

  const onCurrentPageChange = useCallback(
    (page: number) => {
      opts.setCurrentPage(page);
      opts.setPageInput(String(page + 1));
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
    [opts.setCurrentPage, opts.setPageInput],
  );

  const visible = useVisiblePages({
    scrollRef: opts.scrollRef,
    pageCount: opts.pageCount,
    pageSizes: opts.pageSizes,
    zoom: opts.zoom,
    onCurrentPageChange,
    documentKey: `${opts.workspaceView}:${opts.filePath}`,
  });

  const goToPageContinuous = useCallback(
    (page: number) => {
      if (opts.pageCount === null) return;
      const clamped = Math.max(0, Math.min(page, opts.pageCount - 1));
      opts.setCurrentPage(clamped);
      opts.setPageInput(String(clamped + 1));
      // Reach through the ref so this callback doesn't depend on the `visible`
      // object (a fresh object every render) - that dependency made
      // goToPageContinuous churn identity each render and cascade re-renders.
      visible.scrollToPageRef.current(clamped);
      requestPage(clamped);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
    [opts.pageCount, opts.setCurrentPage, opts.setPageInput, requestPage, visible.scrollToPageRef],
  );

  const continuous = useMemo(
    () => ({
      placeholderHeight: visible.placeholderHeight,
      registerPageRef: visible.registerPageRef,
      getPageUrl,
      requestPage,
      renderPages: visible.renderPages,
    }),
    [visible.placeholderHeight, visible.registerPageRef, getPageUrl, requestPage, visible.renderPages],
  );

  return {
    continuous,
    scrollToPageRef: visible.scrollToPageRef,
    goToPageContinuous,
  };
}

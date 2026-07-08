import type { Dispatch, SetStateAction } from 'react';
import { useCallback, useMemo } from 'react';
import { useThumbnailReorder } from './useThumbnailReorder';
import { usePageZoom } from '../viewer/usePageZoom';
import { useWheelNavigation } from '../viewer/useWheelNavigation';
import { useContinuousScroll } from '../viewer/useContinuousScroll';
import { useAnnouncer } from '../ui/useAnnouncer';
import type { PdfPageSize, ScrollViewMode, ViewMode, WorkspaceViewMode } from './types';

type UseAppViewerWorkflowInput = {
  pageCount: number | null;
  viewMode: ViewMode;
  scrollViewMode: ScrollViewMode;
  currentPage: number;
  filePath: string;
  pdfRevision: number;
  pageSizes: PdfPageSize[];
  draggedIndex: number | null;
  zoom: number;
  zoomInput: string;
  pageInput: string;
  setDraggedIndex: (index: number | null) => void;
  setCurrentPage: (page: number) => void;
  setZoom: Dispatch<SetStateAction<number>>;
  setZoomInput: (value: string) => void;
  setPageInput: (value: string) => void;
  goToPage: (page: number) => void;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  loadThumbnails: (path: string) => Promise<void>;
  renderPage: (path: string, page: number) => Promise<void>;
  showHiddenLayers: boolean;
  workspaceView: WorkspaceViewMode;
};

export function useAppViewerWorkflow(input: UseAppViewerWorkflowInput) {
  const {
    pageCount,
    viewMode,
    scrollViewMode,
    currentPage,
    filePath,
    pdfRevision,
    pageSizes,
    draggedIndex,
    zoom,
    zoomInput,
    pageInput,
    setDraggedIndex,
    setCurrentPage,
    setZoom,
    setZoomInput,
    setPageInput,
    goToPage: goToPageSingle,
    withLoading,
    markPdfEdited,
    loadThumbnails,
    renderPage,
  } = input;

  const { scrollRef, handleWheel, handleImageLoad } = useWheelNavigation({
    pageCount,
    viewMode,
    scrollViewMode,
    currentPage,
    goToPage: goToPageSingle,
  });

  const { announce } = useAnnouncer();

  const continuousScroll = useContinuousScroll({
    filePath,
    pdfRevision,
    pageCount,
    pageSizes,
    zoom,
    scrollRef,
    setCurrentPage,
    setPageInput,
    showHiddenLayers: input.showHiddenLayers,
    workspaceView: input.workspaceView,
  });

  const goToPage = useCallback(
    (page: number) => {
      const target = pageCount === null ? page : Math.max(0, Math.min(page, pageCount - 1));
      if (scrollViewMode === 'continuous' && viewMode === 'pdf') {
        continuousScroll.goToPageContinuous(target);
      } else {
        goToPageSingle(target);
      }
      if (pageCount !== null) {
        announce(`Page ${target + 1} of ${pageCount}`);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
    [continuousScroll.goToPageContinuous, goToPageSingle, scrollViewMode, viewMode, pageCount, announce],
  );

  const { handleDragStart, handleDragOver, handleDrop } = useThumbnailReorder({
    filePath,
    draggedIndex,
    withLoading,
    markPdfEdited,
    loadThumbnails,
    renderPage,
    setDraggedIndex,
    setCurrentPage,
  });

  const { zoomIn, zoomOut, resetZoom, commitZoom, commitPage } = usePageZoom({
    zoom,
    setZoom,
    zoomInput,
    setZoomInput,
    pageInput,
    setPageInput,
    pageCount,
    currentPage,
    goToPage,
  });

  const continuous = useMemo(
    () => (scrollViewMode === 'continuous' ? continuousScroll.continuous : null),
    [continuousScroll.continuous, scrollViewMode],
  );

  return {
    scrollRef,
    handleWheel,
    handleImageLoad,
    handleDragStart,
    handleDragOver,
    handleDrop,
    zoomIn,
    zoomOut,
    resetZoom,
    commitZoom,
    commitPage,
    goToPage,
    continuous,
    scrollToPageRef: continuousScroll.scrollToPageRef,
  };
}

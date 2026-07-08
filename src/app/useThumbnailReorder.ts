import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';

type UseThumbnailReorderOptions = {
  filePath: string;
  draggedIndex: number | null;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  loadThumbnails: (path: string) => Promise<void>;
  renderPage: (path: string, page: number) => Promise<void>;
  setDraggedIndex: (index: number | null) => void;
  setCurrentPage: (page: number) => void;
};

export function useThumbnailReorder({
  filePath,
  draggedIndex,
  withLoading,
  markPdfEdited,
  loadThumbnails,
  renderPage,
  setDraggedIndex,
  setCurrentPage,
}: UseThumbnailReorderOptions) {
  const handleDragStart = useCallback((idx: number) => setDraggedIndex(idx), [setDraggedIndex]);

  const handleDragOver = useCallback((e: React.DragEvent) => e.preventDefault(), []);

  const handleDrop = useCallback(async (e: React.DragEvent, targetIdx: number) => {
    e.preventDefault();
    if (draggedIndex !== null && draggedIndex !== targetIdx) {
      await withLoading(async () => {
        await invoke('move_page', { path: filePath, fromIndex: draggedIndex, toIndex: targetIdx });
        markPdfEdited();
        await loadThumbnails(filePath);
        setDraggedIndex(null);
        setCurrentPage(targetIdx);
        await renderPage(filePath, targetIdx);
      });
    }
  }, [
    draggedIndex,
    filePath,
    withLoading,
    markPdfEdited,
    loadThumbnails,
    renderPage,
    setDraggedIndex,
    setCurrentPage,
  ]);

  return { handleDragStart, handleDragOver, handleDrop };
}

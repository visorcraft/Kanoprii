import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageTextEdit } from './types';

type UsePageTextEditsOptions = {
  filePath: string;
  currentPage: number;
  pageTextDraft: string;
  pageTextFontSize: string;
  pendingTextPos: { x: number; y: number } | null;
  editingTextIndex: number | null;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  renderPage: (path: string, page: number) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setShowPageTextModal: (open: boolean) => void;
  setShowPageEditsModal: (open: boolean) => void;
  setPendingTextPos: (pos: { x: number; y: number } | null) => void;
  setEditingTextIndex: (index: number | null) => void;
  setPageTextDraft: (text: string) => void;
  setPageTextFontSize: (size: string) => void;
  setTextEditMode: (mode: boolean) => void;
  setVectorEditMode: (mode: boolean) => void;
  cancelDrawing: () => void;
};

export function usePageTextEdits(opts: UsePageTextEditsOptions) {
  const submitPageText = useCallback(async () => {
    const text = opts.pageTextDraft.trim();
    const fontSize = Number.parseFloat(opts.pageTextFontSize);
    if (!opts.filePath || !text || Number.isNaN(fontSize)) return;
    const pos = opts.pendingTextPos;
    if (opts.editingTextIndex === null && !pos) return;
    await opts.withLoading(async () => {
      const wasEdit = opts.editingTextIndex !== null;
      if (wasEdit) {
        await invoke('update_page_text', {
          path: opts.filePath,
          pageIndex: opts.currentPage,
          index: opts.editingTextIndex,
          text,
          x: pos?.x ?? null,
          y: pos?.y ?? null,
          fontSize,
        });
      } else if (pos) {
        await invoke('add_page_text', {
          path: opts.filePath,
          pageIndex: opts.currentPage,
          x: pos.x,
          y: pos.y,
          fontSize,
          text,
        });
      }
      opts.markPdfEdited();
      opts.setShowPageTextModal(false);
      opts.setPendingTextPos(null);
      opts.setEditingTextIndex(null);
      await opts.renderPage(opts.filePath, opts.currentPage);
      opts.showToast(wasEdit ? 'Text updated' : 'Text added to page');
    });
  }, [opts]);

  const startEditPageText = useCallback((edit: PageTextEdit) => {
    opts.setEditingTextIndex(edit.index);
    opts.setPendingTextPos({ x: edit.x, y: edit.y });
    opts.setPageTextDraft(edit.text);
    opts.setPageTextFontSize(String(edit.font_size));
    opts.setShowPageEditsModal(false);
    opts.setShowPageTextModal(true);
  }, [opts]);

  const closePageTextModal = useCallback(() => {
    opts.setShowPageTextModal(false);
    opts.setEditingTextIndex(null);
    opts.setPendingTextPos(null);
  }, [opts]);

  const exitTextEditMode = useCallback(() => {
    opts.setTextEditMode(false);
    opts.setShowPageTextModal(false);
    opts.setPendingTextPos(null);
    opts.setEditingTextIndex(null);
  }, [opts]);

  const exitVectorEditMode = useCallback(() => {
    opts.cancelDrawing();
    opts.setVectorEditMode(false);
  }, [opts]);

  const removePageTextEdit = useCallback(async (index: number) => {
    if (!opts.filePath) return;
    await opts.withLoading(async () => {
      await invoke('remove_page_text', { path: opts.filePath, pageIndex: opts.currentPage, index });
      opts.markPdfEdited();
      await opts.renderPage(opts.filePath, opts.currentPage);
      opts.showToast('Text removed');
    });
  }, [opts]);

  const removePageVectorEdit = useCallback(async (index: number) => {
    if (!opts.filePath) return;
    await opts.withLoading(async () => {
      await invoke('remove_page_vector', { path: opts.filePath, pageIndex: opts.currentPage, index });
      opts.markPdfEdited();
      await opts.renderPage(opts.filePath, opts.currentPage);
      opts.showToast('Vector shape removed');
    });
  }, [opts]);

  return {
    submitPageText,
    startEditPageText,
    closePageTextModal,
    exitTextEditMode,
    exitVectorEditMode,
    removePageTextEdit,
    removePageVectorEdit,
  };
}

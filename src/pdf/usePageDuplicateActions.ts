import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { RunEdit } from './runEditTypes';

type UsePageDuplicateActionsOptions = {
  filePath: string;
  currentPage: number;
  pageInput: string;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  loadThumbnails: (path: string) => Promise<void>;
  renderPage: (path: string, page: number) => Promise<void>;
  runEdit: RunEdit;
  showToast: (msg: string, kind?: 'error') => void;
  setPageCount: (count: number) => void;
  setCurrentPage: (page: number) => void;
  setPageInput: (value: string) => void;
};

export function usePageDuplicateActions(opts: UsePageDuplicateActionsOptions) {
  const handleRotatePage = useCallback(async () => {
    await opts.runEdit({ command: 'rotate_page', args: { pageIndex: opts.currentPage }, toast: 'Page rotated 90°' });
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.runEdit, opts.currentPage]);

  const handleDuplicatePageBefore = useCallback(async () => {
    await opts.runEdit<number>({
      command: 'duplicate_page_before',
      args: { pageIndex: opts.currentPage },
      reloadAt: (newIndex) => newIndex,
      toast: () => `Duplicated page ${opts.currentPage + 1} before itself`,
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.runEdit, opts.currentPage]);

  const handleDuplicatePage = useCallback(async () => {
    if (!opts.filePath) return;
    const sourcePage = opts.currentPage;
    await opts.withLoading(async () => {
      const newIndex = await invoke<number>('duplicate_page', {
        path: opts.filePath,
        pageIndex: sourcePage,
      });
      opts.markPdfEdited();
      const count = await invoke<number>('get_pdf_page_count', { path: opts.filePath });
      opts.setPageCount(count);
      opts.setCurrentPage(newIndex);
      opts.setPageInput(String(newIndex + 1));
      await opts.renderPage(opts.filePath, newIndex);
      await opts.loadThumbnails(opts.filePath);
      opts.showToast(`Page ${sourcePage + 1} duplicated`);
    });
  }, [opts]);

  const handleDuplicatePageToEnd = useCallback(async () => {
    await opts.runEdit<number>({
      command: 'duplicate_page_to_end',
      args: { pageIndex: opts.currentPage },
      reloadAt: (last) => last,
      toast: () => `Duplicated page ${opts.currentPage + 1} to end`,
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.runEdit, opts.currentPage]);

  return {
    handleRotatePage,
    handleDuplicatePageBefore,
    handleDuplicatePage,
    handleDuplicatePageToEnd,
  };
}

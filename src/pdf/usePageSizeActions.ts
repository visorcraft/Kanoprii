import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageSizePreset } from '../modals/PageSizeModal';
import type { PageRangeController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';
import { fileNameFromPath } from '../app/utils';

type UsePageSizeActionsOptions = {
  filePath: string;
  pageCount: number | null;
  pageSizePreset: PageSizePreset;
  pageSizeRange: PageRangeController;
  runEdit: RunEdit;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  showToast: (msg: string, kind?: 'error') => void;
  setPageSizePreset: (preset: PageSizePreset) => void;
  setShowPageSizeModal: (open: boolean) => void;
};

export function usePageSizeActions(opts: UsePageSizeActionsOptions) {
  const handleSplitOddEven = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) {
      opts.showToast('Need at least 2 pages', 'error');
      return;
    }
    await opts.withLoading(async () => {
      const outputs = await invoke<string[]>('split_odd_even_pages', { path: opts.filePath });
      opts.showToast(`Split into ${outputs.length} files: ${outputs.map((p) => fileNameFromPath(p)).join(', ')}`);
    });
  }, [opts]);

  const handleDuplicateAllPages = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    await opts.runEdit({
      command: 'duplicate_all_pages',
      reloadAt: opts.pageCount,
      toast: (n) => `Duplicated all ${n} pages at end`,
    });
  }, [opts]);

  const openPageSizeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.pageSizeRange.reset();
    opts.setPageSizePreset('letter');
    opts.setShowPageSizeModal(true);
  }, [opts]);

  const handleSetPageSize = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.pageSizeRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'set_page_size',
      args: { startPage: start, endPage: end, preset: opts.pageSizePreset },
      toast: (n) => `Resized ${n} page${n === 1 ? '' : 's'} to ${opts.pageSizePreset.toUpperCase()}`,
      onSuccess: () => opts.setShowPageSizeModal(false),
    });
  }, [opts]);

  const handleSetPageSizeOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'set_page_size_odd_pages',
      args: { preset: opts.pageSizePreset },
      toast: (n) => `Resized ${n} odd page${n === 1 ? '' : 's'} to ${opts.pageSizePreset.toUpperCase()}`,
      onSuccess: () => opts.setShowPageSizeModal(false),
    });
  }, [opts]);

  const handleSetPageSizeEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'set_page_size_even_pages',
      args: { preset: opts.pageSizePreset },
      toast: (n) => `Resized ${n} even page${n === 1 ? '' : 's'} to ${opts.pageSizePreset.toUpperCase()}`,
      onSuccess: () => opts.setShowPageSizeModal(false),
    });
  }, [opts]);

  return {
    handleSplitOddEven,
    handleDuplicateAllPages,
    openPageSizeModal,
    handleSetPageSize,
    handleSetPageSizeOddPages,
    handleSetPageSizeEvenPages,
  };
}

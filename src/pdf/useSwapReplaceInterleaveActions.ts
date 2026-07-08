import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type UseSwapReplaceInterleaveActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  swapPageA: number;
  swapPageB: number;
  replaceSourcePath: string;
  replaceSourcePage: number;
  interleaveFilePath: string;
  interleaveRange: PageRangePairController;
  runEdit: RunEdit;
  showToast: (msg: string, kind?: 'error') => void;
  setSwapPageA: (page: number) => void;
  setSwapPageB: (page: number) => void;
  setShowSwapPagesModal: (open: boolean) => void;
  setReplaceSourcePath: (path: string) => void;
  setReplaceSourcePage: (page: number | ((prev: number) => number)) => void;
  setReplaceSourcePageCount: (count: number | null) => void;
  setShowReplacePageModal: (open: boolean) => void;
  setInterleaveFilePath: (path: string) => void;
  setInterleaveSourcePageCount: (count: number | null) => void;
  setShowInterleaveModal: (open: boolean) => void;
};

export function useSwapReplaceInterleaveActions(opts: UseSwapReplaceInterleaveActionsOptions) {
  const openSwapPagesModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.setSwapPageA(opts.currentPage);
    opts.setSwapPageB(Math.min(opts.currentPage + 1, opts.pageCount - 1));
    opts.setShowSwapPagesModal(true);
  }, [opts]);

  const handleSwapPages = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    if (opts.swapPageA === opts.swapPageB) {
      opts.showToast('Choose two different pages', 'error');
      return;
    }
    await opts.runEdit({
      command: 'swap_pages',
      args: { pageIndexA: opts.swapPageA, pageIndexB: opts.swapPageB },
      reloadAt:
        opts.swapPageA === opts.currentPage
          ? opts.swapPageB
          : opts.swapPageB === opts.currentPage
            ? opts.swapPageA
            : opts.currentPage,
      toast: `Swapped pages ${opts.swapPageA + 1} and ${opts.swapPageB + 1}`,
      onSuccess: () => opts.setShowSwapPagesModal(false),
    });
  }, [opts]);

  const openReplacePageModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setReplaceSourcePath('');
    opts.setReplaceSourcePage(opts.currentPage);
    opts.setReplaceSourcePageCount(null);
    opts.setShowReplacePageModal(true);
  }, [opts]);

  const handleReplaceSourcePathChange = useCallback(async (value: string) => {
    opts.setReplaceSourcePath(value);
    const trimmed = value.trim();
    if (!trimmed) {
      opts.setReplaceSourcePageCount(null);
      return;
    }
    try {
      const count = await invoke<number>('get_pdf_page_count', { path: trimmed });
      opts.setReplaceSourcePageCount(count);
      opts.setReplaceSourcePage((prev) => Math.min(prev, Math.max(0, count - 1)));
    } catch {
      opts.setReplaceSourcePageCount(null);
    }
  }, [opts]);

  const handleReplacePage = useCallback(async () => {
    const source = opts.replaceSourcePath.trim();
    if (!opts.filePath || !source) return;
    await opts.runEdit({
      command: 'replace_page',
      args: { pageIndex: opts.currentPage, sourcePath: source, sourcePageIndex: opts.replaceSourcePage },
      toast: `Replaced page ${opts.currentPage + 1}`,
      onSuccess: () => opts.setShowReplacePageModal(false),
    });
  }, [opts]);

  const openInterleaveModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setInterleaveFilePath('');
    opts.interleaveRange.reset(0, 0);
    opts.setInterleaveSourcePageCount(null);
    opts.setShowInterleaveModal(true);
  }, [opts]);

  const handleInterleaveSourcePathChange = useCallback(async (value: string) => {
    opts.setInterleaveFilePath(value);
    const trimmed = value.trim();
    if (!trimmed) {
      opts.setInterleaveSourcePageCount(null);
      return;
    }
    try {
      const count = await invoke<number>('get_pdf_page_count', { path: trimmed });
      opts.setInterleaveSourcePageCount(count);
      opts.interleaveRange.reset(0, Math.max(0, count - 1));
    } catch {
      opts.setInterleaveSourcePageCount(null);
    }
  }, [opts]);

  const handleInterleavePdf = useCallback(async () => {
    const source = opts.interleaveFilePath.trim();
    if (!opts.filePath || !source) return;
    const range = opts.interleaveRange.validate();
    if (!range) return;
    await opts.runEdit({
      command: 'interleave_pdf',
      args: { otherPath: source, otherStart: opts.interleaveRange.startPage, otherEnd: opts.interleaveRange.endPage },
      toast: (n) => `Interleaved ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowInterleaveModal(false),
    });
  }, [opts]);

  return {
    openSwapPagesModal,
    handleSwapPages,
    openReplacePageModal,
    handleReplaceSourcePathChange,
    handleReplacePage,
    openInterleaveModal,
    handleInterleaveSourcePathChange,
    handleInterleavePdf,
  };
}

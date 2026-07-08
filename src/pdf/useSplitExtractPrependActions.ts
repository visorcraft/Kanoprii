import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type UseSplitExtractPrependActionsOptions = {
  filePath: string;
  originalPath: string;
  pageCount: number | null;
  currentPage: number;
  splitAtPage: number;
  deleteNthValue: number;
  extractOddOutputPath: string;
  extractEvenOutputPath: string;
  prependFilePath: string;
  prependRange: PageRangePairController;
  splitEveryN: number;
  runEdit: RunEdit;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  reloadOpenPdf: (page: number) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setSplitAtPage: (page: number) => void;
  setDeleteNthValue: (n: number) => void;
  setExtractOddOutputPath: (path: string) => void;
  setExtractEvenOutputPath: (path: string) => void;
  setPrependFilePath: (path: string) => void;
  setPrependSourcePageCount: (count: number | null) => void;
  setSplitEveryN: (n: number) => void;
  setShowSplitAtModal: (open: boolean) => void;
  setShowDeleteNthModal: (open: boolean) => void;
  setShowExtractOddModal: (open: boolean) => void;
  setShowExtractEvenModal: (open: boolean) => void;
  setShowPrependModal: (open: boolean) => void;
  setShowSplitEveryModal: (open: boolean) => void;
};

export function useSplitExtractPrependActions(opts: UseSplitExtractPrependActionsOptions) {
  const openSplitAtModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    opts.setSplitAtPage(Math.min(opts.currentPage + 1, opts.pageCount - 1) + 1);
    opts.setShowSplitAtModal(true);
  }, [opts]);

  const handleSplitPdfAtPage = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    const atIndex = opts.splitAtPage - 1;
    if (atIndex < 1 || atIndex >= opts.pageCount) {
      opts.showToast(`Split page must be between 2 and ${opts.pageCount}`, 'error');
      return;
    }
    await opts.withLoading(async () => {
      const written = await invoke<string[]>('split_pdf_at_page', {
        path: opts.filePath,
        atPage: atIndex,
      });
      opts.setShowSplitAtModal(false);
      opts.showToast(`Split into ${written.length} files at page ${opts.splitAtPage}`);
    });
  }, [opts]);

  const openDeleteNthModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    opts.setDeleteNthValue(2);
    opts.setShowDeleteNthModal(true);
  }, [opts]);

  const handleDeleteEveryNthPage = useCallback(async () => {
    if (!opts.filePath || opts.deleteNthValue < 2) return;
    await opts.withLoading(async () => {
      const deleted = await invoke<number>('delete_every_nth_page', {
        path: opts.filePath,
        nth: opts.deleteNthValue,
      });
      if (deleted === 0) {
        opts.showToast(`No pages are every ${opts.deleteNthValue}th page`, 'error');
        return;
      }
      opts.markPdfEdited();
      await opts.reloadOpenPdf(Math.min(opts.currentPage, (opts.pageCount ?? 1) - deleted - 1));
      opts.setShowDeleteNthModal(false);
      opts.showToast(`Deleted ${deleted} page${deleted === 1 ? '' : 's'} (every ${opts.deleteNthValue}th)`);
    });
  }, [opts]);

  const openExtractOddModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    const base = (opts.originalPath || opts.filePath).replace(/\.pdf$/i, '');
    opts.setExtractOddOutputPath(`${base}_odd_extract.pdf`);
    opts.setShowExtractOddModal(true);
  }, [opts]);

  const handleExtractOddPages = useCallback(async () => {
    if (!opts.filePath || !opts.extractOddOutputPath.trim()) return;
    await opts.withLoading(async () => {
      const written = await invoke<string>('extract_odd_pages', {
        path: opts.filePath,
        outputPath: opts.extractOddOutputPath.trim(),
      });
      opts.setShowExtractOddModal(false);
      opts.showToast(`Extracted odd pages to ${written}`);
    });
  }, [opts]);

  const openExtractEvenModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    const base = (opts.originalPath || opts.filePath).replace(/\.pdf$/i, '');
    opts.setExtractEvenOutputPath(`${base}_even_extract.pdf`);
    opts.setShowExtractEvenModal(true);
  }, [opts]);

  const handleExtractEvenPages = useCallback(async () => {
    if (!opts.filePath || !opts.extractEvenOutputPath.trim()) return;
    await opts.withLoading(async () => {
      const written = await invoke<string>('extract_even_pages', {
        path: opts.filePath,
        outputPath: opts.extractEvenOutputPath.trim(),
      });
      opts.setShowExtractEvenModal(false);
      opts.showToast(`Extracted even pages to ${written}`);
    });
  }, [opts]);

  const openPrependModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setPrependFilePath('');
    opts.prependRange.reset(0, 0);
    opts.setPrependSourcePageCount(null);
    opts.setShowPrependModal(true);
  }, [opts]);

  const handlePrependSourcePathChange = useCallback(async (value: string) => {
    opts.setPrependFilePath(value);
    const trimmed = value.trim();
    if (!trimmed) {
      opts.setPrependSourcePageCount(null);
      return;
    }
    try {
      const count = await invoke<number>('get_pdf_page_count', { path: trimmed });
      opts.setPrependSourcePageCount(count);
      opts.prependRange.reset(0, Math.max(0, count - 1));
    } catch {
      opts.setPrependSourcePageCount(null);
    }
  }, [opts]);

  const handlePrependPdf = useCallback(async () => {
    const source = opts.prependFilePath.trim();
    if (!opts.filePath || !source) return;
    const range = opts.prependRange.validate();
    if (!range) return;
    await opts.runEdit<number>({
      command: 'prepend_pdf',
      args: { sourcePath: source, sourceStart: opts.prependRange.startPage, sourceEnd: opts.prependRange.endPage },
      reloadAt: (added) => opts.currentPage + added,
      toast: (added) => `Prepended ${added} page${added === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPrependModal(false),
    });
  }, [opts]);

  const openSplitEveryModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setSplitEveryN(2);
    opts.setShowSplitEveryModal(true);
  }, [opts]);

  const handleSplitEveryN = useCallback(async () => {
    if (!opts.filePath || opts.splitEveryN < 1) return;
    await opts.withLoading(async () => {
      const outputs = await invoke<string[]>('split_every_n_pages', {
        path: opts.filePath,
        pagesPerFile: opts.splitEveryN,
      });
      opts.setShowSplitEveryModal(false);
      opts.showToast(`Split into ${outputs.length} file${outputs.length === 1 ? '' : 's'}`);
    });
  }, [opts]);

  return {
    openSplitAtModal,
    handleSplitPdfAtPage,
    openDeleteNthModal,
    handleDeleteEveryNthPage,
    openExtractOddModal,
    handleExtractOddPages,
    openExtractEvenModal,
    handleExtractEvenPages,
    openPrependModal,
    handlePrependSourcePathChange,
    handlePrependPdf,
    openSplitEveryModal,
    handleSplitEveryN,
  };
}

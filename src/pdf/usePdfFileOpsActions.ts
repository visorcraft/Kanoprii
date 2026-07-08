import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';

type UsePdfFileOpsActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  deletePageInput: string;
  splitRanges: string;
  insertFilePath: string;
  insertAtPage: number;
  mergeFilePath: string;
  extractOutputPath: string;
  insertRange: PageRangePairController;
  mergeRange: PageRangePairController;
  extractRange: PageRangePairController;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  loadThumbnails: (path: string) => Promise<void>;
  renderPage: (path: string, page: number) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setPageCount: (count: number) => void;
  setCurrentPage: (page: number) => void;
  setDeletePageInput: (value: string) => void;
  setShowDeleteModal: (open: boolean) => void;
  setShowSplitModal: (open: boolean) => void;
  setSplitRanges: (ranges: string) => void;
  setShowInsertModal: (open: boolean) => void;
  setInsertFilePath: (path: string) => void;
  setInsertAtPage: (page: number) => void;
  setShowMergeModal: (open: boolean) => void;
  setMergeFilePath: (path: string) => void;
  setShowExtractModal: (open: boolean) => void;
};

export function usePdfFileOpsActions(opts: UsePdfFileOpsActionsOptions) {
  const openMergeModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setShowMergeModal(true);
  }, [opts]);

  const handleSplitPdf = useCallback(async () => {
    if (!opts.filePath || !opts.splitRanges) return;
    await opts.withLoading(async () => {
      const ranges = opts.splitRanges.split(',').map((r) => {
        const [start, end] = r.trim().split('-').map((n) => parseInt(n.trim(), 10) - 1);
        return [start, end] as [number, number];
      });
      const outputPaths = await invoke<string[]>('split_pdf', { path: opts.filePath, pageRanges: ranges });
      opts.showToast(`PDF split into ${outputPaths.length} file(s)`);
      opts.setShowSplitModal(false);
      opts.setSplitRanges('');
    });
  }, [opts]);

  const handleDeletePage = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    if (opts.pageCount <= 1) {
      opts.showToast('Cannot delete the only page', 'error');
      return;
    }
    const pageNumber = parseInt(opts.deletePageInput, 10);
    if (Number.isNaN(pageNumber) || pageNumber < 1 || pageNumber > opts.pageCount) {
      opts.showToast(`Enter a page from 1 to ${opts.pageCount}`, 'error');
      opts.setDeletePageInput(String(opts.currentPage + 1));
      return;
    }
    const targetPage = pageNumber - 1;
    await opts.withLoading(async () => {
      await invoke('delete_page', { path: opts.filePath, pageIndex: targetPage });
      opts.markPdfEdited();
      const count = await invoke<number>('get_pdf_page_count', { path: opts.filePath });
      opts.setPageCount(count);
      const newPage = Math.min(targetPage, count - 1);
      opts.setCurrentPage(newPage);
      await opts.loadThumbnails(opts.filePath);
      await opts.renderPage(opts.filePath, newPage);
      opts.setShowDeleteModal(false);
      opts.showToast(`Page ${pageNumber} deleted`);
    });
  }, [opts]);

  const handleExtractPdf = useCallback(async () => {
    const output = opts.extractOutputPath.trim();
    if (!opts.filePath || !output) return;
    const range = opts.extractRange.validate();
    if (!range) return;
    await opts.withLoading(async () => {
      const written = await invoke<string>('extract_pdf_pages', {
        path: opts.filePath,
        outputPath: output,
        startPage: opts.extractRange.startPage,
        endPage: opts.extractRange.endPage,
      });
      opts.showToast(`Extracted pages to ${written}`);
      opts.setShowExtractModal(false);
    });
  }, [opts]);

  const handleInsertPdf = useCallback(async () => {
    if (!opts.filePath || !opts.insertFilePath) return;
    if (!opts.insertRange.validate()) return;
    await opts.withLoading(async () => {
      await invoke('insert_pdf', {
        path: opts.filePath,
        insertPath: opts.insertFilePath,
        atIndex: opts.insertAtPage,
        insertStart: opts.insertRange.startPage,
        insertEnd: opts.insertRange.endPage,
      });
      opts.markPdfEdited();
      opts.showToast('PDF inserted successfully');
      opts.setShowInsertModal(false);
      opts.setInsertFilePath('');
      opts.setInsertAtPage(0);
      opts.insertRange.reset(0, 0);
      await opts.loadThumbnails(opts.filePath);
      const count = await invoke<number>('get_pdf_page_count', { path: opts.filePath });
      opts.setPageCount(count);
    });
  }, [opts]);

  const handleMergePdf = useCallback(async () => {
    if (!opts.filePath || !opts.mergeFilePath) return;
    if (!opts.mergeRange.validate()) return;
    await opts.withLoading(async () => {
      const added = await invoke<number>('merge_pdf', {
        path: opts.filePath,
        mergePath: opts.mergeFilePath,
        mergeStart: opts.mergeRange.startPage,
        mergeEnd: opts.mergeRange.endPage,
      });
      opts.markPdfEdited();
      opts.showToast(`Merged ${added} page${added === 1 ? '' : 's'} from source PDF`);
      opts.setShowMergeModal(false);
      opts.setMergeFilePath('');
      opts.mergeRange.reset(0, 0);
      await opts.loadThumbnails(opts.filePath);
      const count = await invoke<number>('get_pdf_page_count', { path: opts.filePath });
      opts.setPageCount(count);
    });
  }, [opts]);

  const handleOptimizePdf = useCallback(async () => {
    if (!opts.filePath) return;
    await opts.withLoading(async () => {
      const result = await invoke<string>('optimize_pdf', { path: opts.filePath });
      opts.showToast(result);
    });
  }, [opts]);

  return {
    openMergeModal,
    handleSplitPdf,
    handleDeletePage,
    handleExtractPdf,
    handleInsertPdf,
    handleMergePdf,
    handleOptimizePdf,
  };
}

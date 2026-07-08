import { useCallback } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';

type UsePdfModalOpenersOptions = {
  filePath: string;
  originalPath: string;
  currentPage: number;
  pageCount: number | null;
  extractRange: PageRangePairController;
  setDeletePageInput: (value: string) => void;
  setShowDeleteModal: (open: boolean) => void;
  setShowInsertModal: (open: boolean) => void;
  setShowSplitModal: (open: boolean) => void;
  setExtractOutputPath: (path: string) => void;
  setShowExtractModal: (open: boolean) => void;
};

export function usePdfModalOpeners(opts: UsePdfModalOpenersOptions) {
  const defaultExtractOutputPath = useCallback((start: number, end: number) => {
    const base = (opts.originalPath || opts.filePath).replace(/\.pdf$/i, '');
    return `${base}_pages_${start + 1}-${end + 1}.pdf`;
  }, [opts.filePath, opts.originalPath]);

  const openDeleteModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.setDeletePageInput(String(opts.currentPage + 1));
    opts.setShowDeleteModal(true);
  }, [opts]);

  const openInsertModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setShowInsertModal(true);
  }, [opts]);

  const openSplitModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setShowSplitModal(true);
  }, [opts]);

  const openExtractModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.extractRange.reset(opts.currentPage, opts.currentPage);
    opts.setExtractOutputPath(defaultExtractOutputPath(opts.currentPage, opts.currentPage));
    opts.setShowExtractModal(true);
  }, [opts, defaultExtractOutputPath]);

  return {
    defaultExtractOutputPath,
    openDeleteModal,
    openInsertModal,
    openSplitModal,
    openExtractModal,
  };
}

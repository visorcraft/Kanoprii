import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageRangeController } from '../pageRange/usePageRange';
import { ensureExtension } from '../app/utils';

type UseExportPagesActionsOptions = {
  filePath: string;
  originalPath: string;
  pageCount: number | null;
  currentPage: number;
  exportPagesPdfOutputDir: string;
  exportPagePdfPath: string;
  exportPagesPdfRange: PageRangeController;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  showToast: (msg: string, kind?: 'error') => void;
  setExportPagesPdfOutputDir: (dir: string) => void;
  setExportPagePdfPath: (path: string) => void;
  setShowExportPagesPdfModal: (open: boolean) => void;
  setShowExportPagePdfModal: (open: boolean) => void;
};

export function useExportPagesActions(opts: UseExportPagesActionsOptions) {
  const defaultExportPagesPdfDir = useCallback(() => {
    const base = (opts.originalPath || opts.filePath).replace(/\.pdf$/i, '');
    return `${base}_pages`;
  }, [opts.filePath, opts.originalPath]);

  const openExportPagesPdfModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.exportPagesPdfRange.reset();
    opts.setExportPagesPdfOutputDir(defaultExportPagesPdfDir());
    opts.setShowExportPagesPdfModal(true);
  }, [opts, defaultExportPagesPdfDir]);

  const handleExportPagesPdf = useCallback(async () => {
    const outputDir = opts.exportPagesPdfOutputDir.trim();
    if (!opts.filePath || !outputDir) return;
    const range = opts.exportPagesPdfRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.withLoading(async () => {
      const written = await invoke<string[]>('export_pdf_pages_as_pdf', {
        path: opts.filePath,
        startPage: start,
        endPage: end,
        outputDir,
      });
      opts.setShowExportPagesPdfModal(false);
      opts.showToast(`Exported ${written.length} PDF file${written.length === 1 ? '' : 's'} to ${outputDir}`);
    });
  }, [opts]);

  const handleExportOddPagesAsPdf = useCallback(async () => {
    const outputDir = opts.exportPagesPdfOutputDir.trim();
    if (!opts.filePath || !outputDir) return;
    await opts.withLoading(async () => {
      const written = await invoke<string[]>('export_odd_pages_as_pdf', { path: opts.filePath, outputDir });
      opts.setShowExportPagesPdfModal(false);
      opts.showToast(`Exported ${written.length} odd page PDF${written.length === 1 ? '' : 's'} to ${outputDir}`);
    });
  }, [opts]);

  const handleExportEvenPagesAsPdf = useCallback(async () => {
    const outputDir = opts.exportPagesPdfOutputDir.trim();
    if (!opts.filePath || !outputDir) return;
    await opts.withLoading(async () => {
      const written = await invoke<string[]>('export_even_pages_as_pdf', { path: opts.filePath, outputDir });
      opts.setShowExportPagesPdfModal(false);
      opts.showToast(`Exported ${written.length} even page PDF${written.length === 1 ? '' : 's'} to ${outputDir}`);
    });
  }, [opts]);

  const defaultExportPagePdfPath = useCallback(() => {
    const base = (opts.originalPath || opts.filePath).replace(/\.pdf$/i, '');
    return `${base}_page_${opts.currentPage + 1}.pdf`;
  }, [opts.filePath, opts.originalPath, opts.currentPage]);

  const openExportPagePdfModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setExportPagePdfPath(defaultExportPagePdfPath());
    opts.setShowExportPagePdfModal(true);
  }, [opts, defaultExportPagePdfPath]);

  const handleExportPagePdf = useCallback(async () => {
    const output = opts.exportPagePdfPath.trim();
    if (!opts.filePath || !output) return;
    await opts.withLoading(async () => {
      const written = await invoke<string>('export_page_as_pdf', {
        path: opts.filePath,
        pageIndex: opts.currentPage,
        outputPath: ensureExtension(output, 'pdf'),
      });
      opts.showToast(`Exported page to ${written}`);
      opts.setShowExportPagePdfModal(false);
    });
  }, [opts]);

  return {
    defaultExportPagesPdfDir,
    openExportPagesPdfModal,
    handleExportPagesPdf,
    handleExportOddPagesAsPdf,
    handleExportEvenPagesAsPdf,
    defaultExportPagePdfPath,
    openExportPagePdfModal,
    handleExportPagePdf,
  };
}

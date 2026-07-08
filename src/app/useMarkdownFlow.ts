import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { MarkdownSaveResult, PdfSummaryResult, SummarySaveResult, ViewMode } from './types';
import {
  formatSummaryMarkdown,
  markdownOcrNoticeFromResult,
  markdownSaveToastMessage,
  pickSaveWithNativeDialog,
  siblingMarkdownPath,
  ensureExtension,
} from './utils';
import { MARKDOWN_DIALOG_FILTER } from './constants';

type UseMarkdownFlowOptions = {
  filePath: string;
  originalPath: string;
  viewMode: ViewMode;
  markdownText: string;
  markdownPath: string;
  markdownSaveAsPath: string;
  pdfRevision: number;
  markdownRevision: number | null;
  nativeDialogs: boolean;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  shouldShowTesseractReminder: () => boolean;
  setViewMode: (mode: ViewMode) => void;
  setMarkdownText: (text: string) => void;
  setMarkdownPath: (path: string) => void;
  setMarkdownRevision: (revision: number | null) => void;
  setMarkdownOcrNotice: (notice: ReturnType<typeof markdownOcrNoticeFromResult>) => void;
  setShowMarkdownSaveAsModal: (open: boolean) => void;
  setMarkdownSaveAsPath: (path: string) => void;
  setTesseractReminderSource: (source: 'markdown' | 'launch') => void;
  setShowTesseractModal: (open: boolean) => void;
  pdfSummary: PdfSummaryResult | null;
  setPdfSummary: (summary: PdfSummaryResult) => void;
  setShowSummaryModal: (open: boolean) => void;
  showToast: (msg: string, kind?: 'error') => void;
};

export function useMarkdownFlow(opts: UseMarkdownFlowOptions) {
  const saveMarkdownToPath = useCallback(async (target: string, switchToMarkdown: boolean) => {
    if (!opts.filePath || !target) return;
    let result = await invoke<MarkdownSaveResult>('save_pdf_markdown', {
      path: opts.filePath,
      overwrite: false,
      outputPath: target,
    });
    if (result.conflict) {
      const overwrite = window.confirm('Overwrite Markdown File?');
      if (!overwrite) return;
      result = await invoke<MarkdownSaveResult>('save_pdf_markdown', {
        path: opts.filePath,
        overwrite: true,
        outputPath: target,
      });
    }
    opts.setMarkdownText(result.markdown);
    opts.setMarkdownPath(result.markdownPath);
    opts.setMarkdownRevision(opts.pdfRevision);
    opts.setMarkdownOcrNotice(markdownOcrNoticeFromResult(result));
    if (switchToMarkdown) opts.setViewMode('markdown');
    opts.showToast(markdownSaveToastMessage(result));
  }, [opts]);

  const handleMarkdownView = useCallback(async () => {
    if (!opts.filePath) return;
    if (opts.markdownText && opts.markdownRevision === opts.pdfRevision) {
      opts.setViewMode('markdown');
      return;
    }
    await opts.withLoading(async () => {
      await saveMarkdownToPath(siblingMarkdownPath(opts.originalPath || opts.filePath), true);
    });
  }, [opts, saveMarkdownToPath]);

  const toggleMarkdownView = useCallback(async () => {
    if (!opts.filePath) return;
    if (opts.viewMode === 'markdown') {
      opts.setViewMode('pdf');
      return;
    }
    if (opts.shouldShowTesseractReminder()) {
      opts.setTesseractReminderSource('markdown');
      opts.setShowTesseractModal(true);
      return;
    }
    await handleMarkdownView();
  }, [opts, handleMarkdownView]);

  const handleMarkdownSaveAs = useCallback(async () => {
    const target = opts.markdownSaveAsPath.trim();
    if (!opts.filePath || !target) return;
    await opts.withLoading(async () => {
      await saveMarkdownToPath(target, opts.viewMode === 'markdown');
      opts.setShowMarkdownSaveAsModal(false);
    });
  }, [opts, saveMarkdownToPath]);

  const markdownSaveAsViaNativeDialog = useCallback(async () => {
    if (!opts.filePath) return;
    const defaultPath = opts.markdownPath || siblingMarkdownPath(opts.originalPath || opts.filePath);
    const picked = await pickSaveWithNativeDialog(opts.markdownSaveAsPath || defaultPath, MARKDOWN_DIALOG_FILTER);
    if (!picked) return;
    const target = ensureExtension(picked, 'md');
    await opts.withLoading(async () => {
      await saveMarkdownToPath(target, opts.viewMode === 'markdown');
      opts.setShowMarkdownSaveAsModal(false);
    });
  }, [opts, saveMarkdownToPath]);

  const chooseMarkdownSaveAsNative = useCallback(async () => {
    const defaultPath = opts.markdownPath || siblingMarkdownPath(opts.originalPath || opts.filePath);
    const picked = await pickSaveWithNativeDialog(opts.markdownSaveAsPath || defaultPath, MARKDOWN_DIALOG_FILTER);
    if (!picked) return;
    opts.setMarkdownSaveAsPath(ensureExtension(picked, 'md'));
  }, [opts]);

  const openMarkdownSaveAs = useCallback(() => {
    if (opts.nativeDialogs) {
      void markdownSaveAsViaNativeDialog();
      return;
    }
    const defaultPath = opts.markdownPath || siblingMarkdownPath(opts.originalPath || opts.filePath);
    opts.setMarkdownSaveAsPath(defaultPath);
    opts.setShowMarkdownSaveAsModal(true);
  }, [opts, markdownSaveAsViaNativeDialog]);

  const handleSummarizePdf = useCallback(async () => {
    if (!opts.filePath) return;
    await opts.withLoading(async () => {
      const summary = await invoke<PdfSummaryResult>('summarize_pdf', { path: opts.filePath });
      opts.setPdfSummary(summary);
      opts.setShowSummaryModal(true);
    });
  }, [opts]);

  const handleCopySummary = useCallback(async () => {
    if (!opts.pdfSummary) return;
    try {
      await navigator.clipboard.writeText(formatSummaryMarkdown(opts.pdfSummary));
      opts.showToast('Summary copied');
    } catch {
      opts.showToast('Could not copy summary', 'error');
    }
  }, [opts]);

  const handleSaveSummary = useCallback(async () => {
    if (!opts.filePath) return;
    await opts.withLoading(async () => {
      let result = await invoke<SummarySaveResult>('save_pdf_summary', { path: opts.filePath, overwrite: false });
      if (result.conflict) {
        const overwrite = window.confirm('Overwrite existing summary file?');
        if (!overwrite) return;
        result = await invoke<SummarySaveResult>('save_pdf_summary', { path: opts.filePath, overwrite: true });
      }
      opts.setPdfSummary(result.summary);
      opts.showToast(result.written ? `Summary saved to ${result.summaryPath}` : 'Summary already saved');
    });
  }, [opts]);

  return {
    saveMarkdownToPath,
    handleMarkdownView,
    toggleMarkdownView,
    handleMarkdownSaveAs,
    markdownSaveAsViaNativeDialog,
    chooseMarkdownSaveAsNative,
    openMarkdownSaveAs,
    handleSummarizePdf,
    handleCopySummary,
    handleSaveSummary,
  };
}

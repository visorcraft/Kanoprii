import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';
import { useAnnouncer } from '../ui/useAnnouncer';
import type { PageRangePairController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

export type UseDocumentEnhancementActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  pdfRevision: number;
  ocrAvailable: boolean | null;
  batesRange: PageRangePairController;
  batesPrefix: string;
  batesStartNumber: number;
  batesDigits: number;
  batesPosition: string;
  applyRedactionsOcrAfter: boolean;
  runEdit: RunEdit;
  showToast: (msg: string, kind?: 'error') => void;
  openTesseractGuide: () => void;
  setShowBatesNumberModal: (open: boolean) => void;
  setShowApplyRedactionsModal: (open: boolean) => void;
  setBatesPrefix: (value: string) => void;
  setBatesStartNumber: (value: number) => void;
  setBatesDigits: (value: number) => void;
  setBatesPosition: (value: string) => void;
};

export function useDocumentEnhancementActions(
  opts: UseDocumentEnhancementActionsOptions
) {
  const { announce } = useAnnouncer();
  const [hasRedactions, setHasRedactions] = useState(false);

  useEffect(() => {
    if (!opts.filePath) {
      setHasRedactions(false);
      return;
    }
    void invoke<boolean>('has_redaction_boxes', { path: opts.filePath })
      .then(setHasRedactions)
      .catch(() => setHasRedactions(false));
  }, [opts.filePath, opts.currentPage, opts.pdfRevision]);

  const handleMakePdfSearchable = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    if (opts.ocrAvailable === false) {
      opts.openTesseractGuide();
      return;
    }
    const endPage = opts.pageCount - 1;
    opts.showToast('Making PDF searchable (OCR)…');
    const result = await opts.runEdit<number>({
      command: 'make_pdf_searchable',
      args: { startPage: 0, endPage },
      reloadAt: opts.currentPage,
      toast: (n) => `OCR text layer added to ${n} page${n === 1 ? '' : 's'}`,
    });
    if (result != null) {
      const n = result;
      announce(`OCR text layer added to ${n} page${n === 1 ? '' : 's'}`);
    }
  }, [opts, announce]);

  const openBatesNumberModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.batesRange.reset();
    opts.setBatesPrefix('');
    opts.setBatesStartNumber(1);
    opts.setBatesDigits(6);
    opts.setBatesPosition('footer-right');
    opts.setShowBatesNumberModal(true);
  }, [opts]);

  const handleAddBatesNumbers = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.batesRange.validate();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'add_bates_numbers',
      args: {
        startPage: start,
        endPage: end,
        prefix: opts.batesPrefix,
        startNumber: opts.batesStartNumber,
        digits: opts.batesDigits,
        position: opts.batesPosition,
      },
      reloadAt: opts.currentPage,
      toast: 'Bates numbers added',
      onSuccess: () => {
        opts.setShowBatesNumberModal(false);
        announce('Bates numbers added');
      },
    });
  }, [opts, announce]);

  const openApplyRedactionsModal = useCallback(() => {
    if (!opts.filePath || !hasRedactions) return;
    opts.setShowApplyRedactionsModal(true);
  }, [hasRedactions, opts]);

  const handleApplyRedactions = useCallback(async () => {
    if (!opts.filePath) return;
    if (opts.applyRedactionsOcrAfter && opts.ocrAvailable === false) {
      opts.openTesseractGuide();
      return;
    }
    const result = await opts.runEdit<number>({
      command: 'apply_redactions',
      args: { ocrAfter: opts.applyRedactionsOcrAfter },
      reloadAt: opts.currentPage,
      toast: (n) => `Redactions applied to ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowApplyRedactionsModal(false),
    });
    if (result != null) {
      const n = result;
      announce(`Redactions applied to ${n} page${n === 1 ? '' : 's'}`);
    }
  }, [opts, announce]);

  return {
    hasRedactions,
    handleMakePdfSearchable,
    openBatesNumberModal,
    handleAddBatesNumbers,
    openApplyRedactionsModal,
    handleApplyRedactions,
  };
}

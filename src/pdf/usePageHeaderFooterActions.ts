import { useCallback } from 'react';
import type { PageRangeController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type UsePageHeaderFooterActionsOptions = {
  filePath: string;
  pageCount: number | null;
  pageHeaderText: string;
  pageFooterText: string;
  pageHeaderRange: PageRangeController;
  pageFooterRange: PageRangeController;
  runEdit: RunEdit;
  setPageHeaderText: (text: string) => void;
  setPageFooterText: (text: string) => void;
  setShowPageHeaderModal: (open: boolean) => void;
  setShowPageFooterModal: (open: boolean) => void;
};

export function usePageHeaderFooterActions(opts: UsePageHeaderFooterActionsOptions) {
  const openPageHeaderModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.pageHeaderRange.reset();
    opts.setPageHeaderText('DRAFT');
    opts.setShowPageHeaderModal(true);
  }, [opts]);

  const handleAddPageHeader = useCallback(async () => {
    if (!opts.filePath || !opts.pageHeaderText.trim()) return;
    const range = opts.pageHeaderRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'add_page_header',
      args: { startPage: start, endPage: end, text: opts.pageHeaderText.trim() },
      toast: (n) => `Added header to ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageHeaderModal(false),
    });
  }, [opts]);

  const handleAddPageHeaderOddPages = useCallback(async () => {
    if (!opts.filePath || !opts.pageHeaderText.trim()) return;
    await opts.runEdit({
      command: 'add_page_header_odd_pages',
      args: { text: opts.pageHeaderText.trim() },
      toast: (n) => `Added header to ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageHeaderModal(false),
    });
  }, [opts]);

  const handleAddPageHeaderEvenPages = useCallback(async () => {
    if (!opts.filePath || !opts.pageHeaderText.trim()) return;
    await opts.runEdit({
      command: 'add_page_header_even_pages',
      args: { text: opts.pageHeaderText.trim() },
      toast: (n) => `Added header to ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageHeaderModal(false),
    });
  }, [opts]);

  const openPageFooterModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.pageFooterRange.reset();
    opts.setPageFooterText('Confidential');
    opts.setShowPageFooterModal(true);
  }, [opts]);

  const handleAddPageFooter = useCallback(async () => {
    if (!opts.filePath || !opts.pageFooterText.trim()) return;
    const range = opts.pageFooterRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'add_page_footer',
      args: { startPage: start, endPage: end, text: opts.pageFooterText.trim() },
      toast: (n) => `Added footer to ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageFooterModal(false),
    });
  }, [opts]);

  const handleAddPageFooterOddPages = useCallback(async () => {
    if (!opts.filePath || !opts.pageFooterText.trim()) return;
    await opts.runEdit({
      command: 'add_page_footer_odd_pages',
      args: { text: opts.pageFooterText.trim() },
      toast: (n) => `Added footer to ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageFooterModal(false),
    });
  }, [opts]);

  const handleAddPageFooterEvenPages = useCallback(async () => {
    if (!opts.filePath || !opts.pageFooterText.trim()) return;
    await opts.runEdit({
      command: 'add_page_footer_even_pages',
      args: { text: opts.pageFooterText.trim() },
      toast: (n) => `Added footer to ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageFooterModal(false),
    });
  }, [opts]);

  return {
    openPageHeaderModal,
    handleAddPageHeader,
    handleAddPageHeaderOddPages,
    handleAddPageHeaderEvenPages,
    openPageFooterModal,
    handleAddPageFooter,
    handleAddPageFooterOddPages,
    handleAddPageFooterEvenPages,
  };
}

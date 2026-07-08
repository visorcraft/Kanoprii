import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageRangeController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type UsePageDecorActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  pageBorderRange: PageRangeController;
  expandMarginsRange: PageRangeController;
  shrinkMarginsRange: PageRangeController;
  pageNumbersRange: PageRangeController;
  watermarkRange: PageRangeController;
  flattenRange: PageRangeController;
  pageBorderInset: number;
  expandMarginTop: number;
  expandMarginRight: number;
  expandMarginBottom: number;
  expandMarginLeft: number;
  shrinkMarginTop: number;
  shrinkMarginRight: number;
  shrinkMarginBottom: number;
  shrinkMarginLeft: number;
  cropMarginTop: number;
  cropMarginRight: number;
  cropMarginBottom: number;
  cropMarginLeft: number;
  cropApplyAll: boolean;
  pageNumbersPrefix: string;
  watermarkText: string;
  insertImageAtIndex: number;
  insertImagePagePath: string;
  runEdit: RunEdit;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  reloadOpenPdf: (page: number) => Promise<void>;
  loadPageSizes: (path: string) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setPageBorderInset: (inset: number) => void;
  setExpandMarginTop: (value: number) => void;
  setExpandMarginRight: (value: number) => void;
  setExpandMarginBottom: (value: number) => void;
  setExpandMarginLeft: (value: number) => void;
  setShrinkMarginTop: (value: number) => void;
  setShrinkMarginRight: (value: number) => void;
  setShrinkMarginBottom: (value: number) => void;
  setShrinkMarginLeft: (value: number) => void;
  setCropMarginTop: (value: number) => void;
  setCropMarginRight: (value: number) => void;
  setCropMarginBottom: (value: number) => void;
  setCropMarginLeft: (value: number) => void;
  setCropApplyAll: (value: boolean) => void;
  setPageNumbersPrefix: (prefix: string) => void;
  setWatermarkText: (text: string) => void;
  setInsertImageAtIndex: (index: number) => void;
  setInsertImagePagePath: (path: string) => void;
  setShowPageBorderModal: (open: boolean) => void;
  setShowExpandMarginsModal: (open: boolean) => void;
  setShowShrinkMarginsModal: (open: boolean) => void;
  setShowInsertImagePageModal: (open: boolean) => void;
  setShowPageNumbersModal: (open: boolean) => void;
  setShowWatermarkModal: (open: boolean) => void;
  setShowCropModal: (open: boolean) => void;
  setShowFlattenModal: (open: boolean) => void;
};

export function usePageDecorActions(opts: UsePageDecorActionsOptions) {
  const openPageBorderModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.pageBorderRange.reset();
    opts.setPageBorderInset(20);
    opts.setShowPageBorderModal(true);
  }, [opts]);

  const handleAddPageBorder = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.pageBorderRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'add_page_border',
      args: { startPage: start, endPage: end, inset: opts.pageBorderInset },
      toast: (n) => `Added border to ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageBorderModal(false),
    });
  }, [opts]);

  const handleAddPageBorderOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'add_page_border_odd_pages',
      args: { inset: opts.pageBorderInset },
      toast: (n) => `Added border to ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageBorderModal(false),
    });
  }, [opts]);

  const handleAddPageBorderEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'add_page_border_even_pages',
      args: { inset: opts.pageBorderInset },
      toast: (n) => `Added border to ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageBorderModal(false),
    });
  }, [opts]);

  const handleInsertBlankBeforeOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'insert_blank_before_odd_pages',
      toast: (n) => `Inserted ${n} blank page${n === 1 ? '' : 's'} before odd pages`,
    });
  }, [opts]);

  const handleInsertBlankBeforeEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'insert_blank_before_even_pages',
      toast: (n) => `Inserted ${n} blank page${n === 1 ? '' : 's'} before even pages`,
    });
  }, [opts]);

  const handleInsertBlankAfterOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'insert_blank_after_odd_pages',
      toast: (n) => `Inserted ${n} blank page${n === 1 ? '' : 's'} after odd pages`,
    });
  }, [opts]);

  const handleInsertBlankAfterEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'insert_blank_after_even_pages',
      toast: (n) => `Inserted ${n} blank page${n === 1 ? '' : 's'} after even pages`,
    });
  }, [opts]);

  const handleDuplicateOddPagesToEnd = useCallback(async () => {
    await opts.runEdit({
      command: 'duplicate_odd_pages_to_end',
      toast: (n) => `Moved ${n} odd page cop${n === 1 ? 'y' : 'ies'} to end`,
    });
  }, [opts]);

  const handleDuplicateEvenPagesToEnd = useCallback(async () => {
    await opts.runEdit({
      command: 'duplicate_even_pages_to_end',
      toast: (n) => `Moved ${n} even page cop${n === 1 ? 'y' : 'ies'} to end`,
    });
  }, [opts]);

  const handleDuplicateOddPagesToStart = useCallback(async () => {
    await opts.runEdit({
      command: 'duplicate_odd_pages_to_start',
      reloadAt: 0,
      toast: (n) => `Inserted ${n} odd page cop${n === 1 ? 'y' : 'ies'} at start`,
    });
  }, [opts]);

  const handleDuplicateEvenPagesToStart = useCallback(async () => {
    await opts.runEdit({
      command: 'duplicate_even_pages_to_start',
      reloadAt: 0,
      toast: (n) => `Inserted ${n} even page cop${n === 1 ? 'y' : 'ies'} at start`,
    });
  }, [opts]);

  const openExpandMarginsModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.expandMarginsRange.reset();
    opts.setExpandMarginTop(20);
    opts.setExpandMarginRight(20);
    opts.setExpandMarginBottom(20);
    opts.setExpandMarginLeft(20);
    opts.setShowExpandMarginsModal(true);
  }, [opts]);

  const openShrinkMarginsModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.shrinkMarginsRange.reset();
    opts.setShrinkMarginTop(20);
    opts.setShrinkMarginRight(20);
    opts.setShrinkMarginBottom(20);
    opts.setShrinkMarginLeft(20);
    opts.setShowShrinkMarginsModal(true);
  }, [opts]);

  const handleShrinkPageMargins = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.shrinkMarginsRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'shrink_page_margins',
      args: {
        startPage: start,
        endPage: end,
        marginTop: opts.shrinkMarginTop,
        marginRight: opts.shrinkMarginRight,
        marginBottom: opts.shrinkMarginBottom,
        marginLeft: opts.shrinkMarginLeft,
      },
      toast: (n) => `Shrunk margins on ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowShrinkMarginsModal(false),
    });
  }, [opts]);

  const handleExpandPageMargins = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.expandMarginsRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'expand_page_margins',
      args: {
        startPage: start,
        endPage: end,
        marginTop: opts.expandMarginTop,
        marginRight: opts.expandMarginRight,
        marginBottom: opts.expandMarginBottom,
        marginLeft: opts.expandMarginLeft,
      },
      toast: (n) => `Expanded margins on ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowExpandMarginsModal(false),
    });
  }, [opts]);

  const openInsertImagePageModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setInsertImageAtIndex(opts.currentPage + 1);
    opts.setInsertImagePagePath('');
    opts.setShowInsertImagePageModal(true);
  }, [opts]);

  const handleInsertImagePage = useCallback(async () => {
    const image = opts.insertImagePagePath.trim();
    if (!opts.filePath || !image) return;
    await opts.runEdit<number>({
      command: 'insert_image_page',
      args: { atIndex: opts.insertImageAtIndex, imagePath: image },
      reloadAt: (newIndex) => newIndex,
      toast: (newIndex) => `Image page inserted at position ${newIndex + 1}`,
      onSuccess: () => opts.setShowInsertImagePageModal(false),
    });
  }, [opts]);

  const openPageNumbersModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.pageNumbersRange.reset();
    opts.setPageNumbersPrefix('Page ');
    opts.setShowPageNumbersModal(true);
  }, [opts]);

  const handleAddPageNumbers = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.pageNumbersRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'add_page_numbers',
      args: { startPage: start, endPage: end, prefix: opts.pageNumbersPrefix || null },
      toast: (n) => `Added page numbers to ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageNumbersModal(false),
    });
  }, [opts]);

  const handleAddPageNumbersOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'add_page_numbers_odd_pages',
      args: { prefix: opts.pageNumbersPrefix || null },
      toast: (n) => `Added page numbers to ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageNumbersModal(false),
    });
  }, [opts]);

  const handleAddPageNumbersEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'add_page_numbers_even_pages',
      args: { prefix: opts.pageNumbersPrefix || null },
      toast: (n) => `Added page numbers to ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowPageNumbersModal(false),
    });
  }, [opts]);

  const openWatermarkModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.watermarkRange.reset();
    opts.setWatermarkText('DRAFT');
    opts.setShowWatermarkModal(true);
  }, [opts]);

  const handleAddWatermark = useCallback(async () => {
    if (!opts.filePath || !opts.watermarkText.trim()) return;
    const range = opts.watermarkRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'add_text_watermark',
      args: { text: opts.watermarkText.trim(), startPage: start, endPage: end },
      toast: (n) => `Watermarked ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowWatermarkModal(false),
    });
  }, [opts]);

  const handleAddWatermarkOddPages = useCallback(async () => {
    if (!opts.filePath || !opts.watermarkText.trim()) return;
    await opts.runEdit({
      command: 'add_text_watermark_odd_pages',
      args: { text: opts.watermarkText.trim() },
      toast: (n) => `Watermarked ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowWatermarkModal(false),
    });
  }, [opts]);

  const handleAddWatermarkEvenPages = useCallback(async () => {
    if (!opts.filePath || !opts.watermarkText.trim()) return;
    await opts.runEdit({
      command: 'add_text_watermark_even_pages',
      args: { text: opts.watermarkText.trim() },
      toast: (n) => `Watermarked ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowWatermarkModal(false),
    });
  }, [opts]);

  const openCropModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setCropMarginTop(50);
    opts.setCropMarginRight(50);
    opts.setCropMarginBottom(50);
    opts.setCropMarginLeft(50);
    opts.setCropApplyAll(false);
    void opts.loadPageSizes(opts.filePath);
    opts.setShowCropModal(true);
  }, [opts]);

  const handleCropPage = useCallback(async () => {
    if (!opts.filePath) return;
    await opts.withLoading(async () => {
      if (opts.cropApplyAll) {
        const count = await invoke<number>('crop_all_pages', {
          path: opts.filePath,
          marginTop: opts.cropMarginTop,
          marginRight: opts.cropMarginRight,
          marginBottom: opts.cropMarginBottom,
          marginLeft: opts.cropMarginLeft,
        });
        opts.markPdfEdited();
        await opts.reloadOpenPdf(opts.currentPage);
        opts.setShowCropModal(false);
        opts.showToast(`Cropped ${count} page${count === 1 ? '' : 's'}`);
        return;
      }
      await invoke('crop_page', {
        path: opts.filePath,
        pageIndex: opts.currentPage,
        marginTop: opts.cropMarginTop,
        marginRight: opts.cropMarginRight,
        marginBottom: opts.cropMarginBottom,
        marginLeft: opts.cropMarginLeft,
      });
      opts.markPdfEdited();
      await opts.reloadOpenPdf(opts.currentPage);
      opts.setShowCropModal(false);
      opts.showToast(`Cropped page ${opts.currentPage + 1}`);
    });
  }, [opts]);

  const handleClearPageCrop = useCallback(async () => {
    await opts.runEdit({
      command: 'clear_page_crop',
      args: { pageIndex: opts.currentPage },
      toast: `Cleared crop on page ${opts.currentPage + 1}`,
    });
  }, [opts]);

  const openFlattenModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.flattenRange.reset();
    opts.setShowFlattenModal(true);
  }, [opts]);

  const handleFlattenAnnotations = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.flattenRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    await opts.runEdit({
      command: 'flatten_annotations',
      args: { startPage: start, endPage: end },
      toast: (n) => `Removed ${n} annotation${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowFlattenModal(false),
    });
  }, [opts]);

  return {
    openPageBorderModal,
    handleAddPageBorder,
    handleAddPageBorderOddPages,
    handleAddPageBorderEvenPages,
    handleInsertBlankBeforeOddPages,
    handleInsertBlankBeforeEvenPages,
    handleInsertBlankAfterOddPages,
    handleInsertBlankAfterEvenPages,
    handleDuplicateOddPagesToEnd,
    handleDuplicateEvenPagesToEnd,
    handleDuplicateOddPagesToStart,
    handleDuplicateEvenPagesToStart,
    openExpandMarginsModal,
    openShrinkMarginsModal,
    handleShrinkPageMargins,
    handleExpandPageMargins,
    openInsertImagePageModal,
    handleInsertImagePage,
    openPageNumbersModal,
    handleAddPageNumbers,
    handleAddPageNumbersOddPages,
    handleAddPageNumbersEvenPages,
    openWatermarkModal,
    handleAddWatermark,
    handleAddWatermarkOddPages,
    handleAddWatermarkEvenPages,
    openCropModal,
    handleCropPage,
    handleClearPageCrop,
    openFlattenModal,
    handleFlattenAnnotations,
  };
}

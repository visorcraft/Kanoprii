import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type UseRangeModalActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  reverseRange: PageRangePairController;
  cropRange: PageRangePairController;
  keepRange: PageRangePairController;
  moveRange: PageRangePairController;
  deleteRange: PageRangePairController;
  insertBlankCount: number;
  insertBlankAtIndex: number;
  moveRangeToIndex: number;
  cropMarginTop: number;
  cropMarginRight: number;
  cropMarginBottom: number;
  cropMarginLeft: number;
  runEdit: RunEdit;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  reloadOpenPdf: (page: number) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setInsertBlankCount: (count: number) => void;
  setInsertBlankAtIndex: (index: number) => void;
  setMoveRangeToIndex: (index: number) => void;
  setCropMarginTop: (value: number) => void;
  setCropMarginRight: (value: number) => void;
  setCropMarginBottom: (value: number) => void;
  setCropMarginLeft: (value: number) => void;
  setMetadataTitle: (value: string) => void;
  setMetadataAuthor: (value: string) => void;
  setMetadataSubject: (value: string) => void;
  setMetadataKeywords: (value: string) => void;
  setMetadataCreator: (value: string) => void;
  setMetadataProducer: (value: string) => void;
  setMetadataCreationDate: (value: string) => void;
  setMetadataModDate: (value: string) => void;
  setShowReverseRangeModal: (open: boolean) => void;
  setShowInsertBlankPagesModal: (open: boolean) => void;
  setShowCropRangeModal: (open: boolean) => void;
  setShowKeepRangeModal: (open: boolean) => void;
  setShowMoveRangeModal: (open: boolean) => void;
  setShowDeleteRangeModal: (open: boolean) => void;
};

export function useRangeModalActions(opts: UseRangeModalActionsOptions) {
  const openReverseRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.reverseRange.reset(opts.currentPage, opts.currentPage);
    opts.setShowReverseRangeModal(true);
  }, [opts]);

  const handleReversePageRange = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.reverseRange.validate();
    if (!range) return;
    await opts.runEdit({
      command: 'reverse_page_range',
      args: {
        startPage: opts.reverseRange.startPage,
        endPage: opts.reverseRange.endPage,
      },
      toast: `Reversed pages ${opts.reverseRange.startPage + 1}–${opts.reverseRange.endPage + 1}`,
      onSuccess: () => opts.setShowReverseRangeModal(false),
    });
  }, [opts]);

  const openInsertBlankPagesModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setInsertBlankCount(1);
    opts.setInsertBlankAtIndex(opts.currentPage + 1);
    opts.setShowInsertBlankPagesModal(true);
  }, [opts]);

  const handleInsertBlankPages = useCallback(async () => {
    if (!opts.filePath || opts.insertBlankCount < 1) return;
    await opts.runEdit({
      command: 'insert_blank_pages',
      args: { atIndex: opts.insertBlankAtIndex, count: opts.insertBlankCount },
      reloadAt: opts.insertBlankAtIndex,
      toast: (n) => `Inserted ${n} blank page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowInsertBlankPagesModal(false),
    });
  }, [opts]);

  const openCropRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.cropRange.reset(opts.currentPage, opts.currentPage);
    opts.setCropMarginTop(50);
    opts.setCropMarginRight(50);
    opts.setCropMarginBottom(50);
    opts.setCropMarginLeft(50);
    opts.setShowCropRangeModal(true);
  }, [opts]);

  const handleCropPageRange = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.cropRange.validate();
    if (!range) return;
    await opts.runEdit({
      command: 'crop_page_range',
      args: {
        startPage: opts.cropRange.startPage,
        endPage: opts.cropRange.endPage,
        marginTop: opts.cropMarginTop,
        marginRight: opts.cropMarginRight,
        marginBottom: opts.cropMarginBottom,
        marginLeft: opts.cropMarginLeft,
      },
      toast: (n) => `Cropped ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowCropRangeModal(false),
    });
  }, [opts]);

  const handleFlattenAllAnnotations = useCallback(async () => {
    await opts.runEdit({
      command: 'flatten_all_annotations',
      toast: (n) =>
        `Flattened ${n} annotation${n === 1 ? '' : 's'} on all pages`,
    });
  }, [opts]);

  const handleClearPdfMetadata = useCallback(async () => {
    await opts.runEdit({
      command: 'clear_pdf_metadata',
      skipReload: true,
      toast: 'Cleared document metadata',
      onSuccess: () => {
        opts.setMetadataTitle('');
        opts.setMetadataAuthor('');
        opts.setMetadataSubject('');
        opts.setMetadataKeywords('');
        opts.setMetadataCreator('');
        opts.setMetadataProducer('');
        opts.setMetadataCreationDate('');
        opts.setMetadataModDate('');
      },
    });
  }, [opts]);

  const handleSortPagesBySize = useCallback(
    async (descending: boolean) => {
      await opts.runEdit({
        command: 'sort_pages_by_size',
        args: { descending },
        reloadAt: 0,
        toast: `Sorted pages by size (${descending ? 'largest first' : 'smallest first'})`,
      });
    },
    [opts]
  );

  const openKeepRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.keepRange.reset(opts.currentPage, opts.currentPage);
    opts.setShowKeepRangeModal(true);
  }, [opts]);

  const handleKeepPageRange = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    const range = opts.keepRange.validate();
    if (!range) return;
    const keepCount = opts.keepRange.endPage - opts.keepRange.startPage + 1;
    if (keepCount >= opts.pageCount) {
      opts.showToast('Range already includes every page', 'error');
      return;
    }
    await opts.runEdit<number>({
      command: 'keep_page_range',
      args: {
        startPage: opts.keepRange.startPage,
        endPage: opts.keepRange.endPage,
      },
      reloadAt: Math.min(opts.keepRange.startPage, keepCount - 1),
      toast: (deleted) =>
        `Kept ${keepCount} page${keepCount === 1 ? '' : 's'}; removed ${deleted}`,
      onSuccess: () => opts.setShowKeepRangeModal(false),
    });
  }, [opts]);

  const openMoveRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.moveRange.reset(opts.currentPage, opts.currentPage);
    opts.setMoveRangeToIndex(opts.currentPage);
    opts.setShowMoveRangeModal(true);
  }, [opts]);

  const handleMovePageRange = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    const range = opts.moveRange.validate();
    if (!range) return;
    if (opts.moveRangeToIndex > opts.pageCount) {
      opts.showToast('Target index out of bounds', 'error');
      return;
    }
    await opts.runEdit({
      command: 'move_page_range',
      args: {
        startPage: opts.moveRange.startPage,
        endPage: opts.moveRange.endPage,
        toIndex: opts.moveRangeToIndex,
      },
      reloadAt: opts.moveRangeToIndex,
      toast: `Moved pages ${opts.moveRange.startPage + 1}–${opts.moveRange.endPage + 1} to index ${opts.moveRangeToIndex + 1}`,
      onSuccess: () => opts.setShowMoveRangeModal(false),
    });
  }, [opts]);

  const handleMovePageRangeToStart = useCallback(async () => {
    if (!opts.filePath) return;
    const range = opts.moveRange.validate();
    if (!range) return;
    await opts.runEdit({
      command: 'move_page_range_to_start',
      args: {
        startPage: opts.moveRange.startPage,
        endPage: opts.moveRange.endPage,
      },
      reloadAt: 0,
      toast: `Moved pages ${opts.moveRange.startPage + 1}–${opts.moveRange.endPage + 1} to start`,
      onSuccess: () => opts.setShowMoveRangeModal(false),
    });
  }, [opts]);

  const handleMovePageRangeToEnd = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    const range = opts.moveRange.validate();
    if (!range) return;
    await opts.runEdit({
      command: 'move_page_range_to_end',
      args: {
        startPage: opts.moveRange.startPage,
        endPage: opts.moveRange.endPage,
      },
      reloadAt:
        opts.pageCount -
        (opts.moveRange.endPage - opts.moveRange.startPage + 1),
      toast: `Moved pages ${opts.moveRange.startPage + 1}–${opts.moveRange.endPage + 1} to end`,
      onSuccess: () => opts.setShowMoveRangeModal(false),
    });
  }, [opts]);

  const openDeleteRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.deleteRange.reset(opts.currentPage, opts.currentPage);
    opts.setShowDeleteRangeModal(true);
  }, [opts]);

  const handleDeletePageRange = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    const range = opts.deleteRange.validate();
    if (!range) return;
    const deleteCount =
      opts.deleteRange.endPage - opts.deleteRange.startPage + 1;
    if (deleteCount >= opts.pageCount) {
      opts.showToast('Cannot delete every page', 'error');
      return;
    }
    await opts.withLoading(async () => {
      await invoke<number>('delete_page_range', {
        path: opts.filePath,
        startPage: opts.deleteRange.startPage,
        endPage: opts.deleteRange.endPage,
      });
      opts.markPdfEdited();
      const nextPage =
        opts.deleteRange.startPage >= opts.pageCount! - deleteCount
          ? Math.max(0, opts.pageCount! - deleteCount - 1)
          : opts.deleteRange.startPage;
      await opts.reloadOpenPdf(nextPage);
      opts.setShowDeleteRangeModal(false);
      opts.showToast(
        `Deleted ${deleteCount} page${deleteCount === 1 ? '' : 's'}`
      );
    });
  }, [opts]);

  return {
    openReverseRangeModal,
    handleReversePageRange,
    openInsertBlankPagesModal,
    handleInsertBlankPages,
    openCropRangeModal,
    handleCropPageRange,
    handleFlattenAllAnnotations,
    handleClearPdfMetadata,
    handleSortPagesBySize,
    openKeepRangeModal,
    handleKeepPageRange,
    openMoveRangeModal,
    handleMovePageRange,
    handleMovePageRangeToStart,
    handleMovePageRangeToEnd,
    openDeleteRangeModal,
    handleDeletePageRange,
  };
}

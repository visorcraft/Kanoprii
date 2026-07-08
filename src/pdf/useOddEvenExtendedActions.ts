import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { RunEdit } from './runEditTypes';

type UseOddEvenExtendedActionsOptions = {
  filePath: string;
  pageCount: number | null;
  runEdit: RunEdit;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  reloadOpenPdf: (page: number) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setShowCropModal: (open: boolean) => void;
};

export function useOddEvenExtendedActions(opts: UseOddEvenExtendedActionsOptions) {
  const handleReverseOddPages = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    await opts.withLoading(async () => {
      const reversed = await invoke<number>('reverse_odd_pages', { path: opts.filePath });
      if (reversed === 0) {
        opts.showToast('Need at least two odd pages to reverse', 'error');
        return;
      }
      opts.markPdfEdited();
      await opts.reloadOpenPdf(0);
      opts.showToast(`Reversed ${reversed} odd page${reversed === 1 ? '' : 's'}`);
    });
  }, [opts]);

  const handleReverseEvenPages = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    await opts.withLoading(async () => {
      const reversed = await invoke<number>('reverse_even_pages', { path: opts.filePath });
      if (reversed === 0) {
        opts.showToast('Need at least two even pages to reverse', 'error');
        return;
      }
      opts.markPdfEdited();
      await opts.reloadOpenPdf(0);
      opts.showToast(`Reversed ${reversed} even page${reversed === 1 ? '' : 's'}`);
    });
  }, [opts]);

  const handleMoveOddPagesToStart = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    await opts.runEdit({ command: 'move_odd_pages_to_start', reloadAt: 0, toast: 'Moved odd pages to start' });
  }, [opts]);

  const handleMoveEvenPagesToStart = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    await opts.runEdit({ command: 'move_even_pages_to_start', reloadAt: 0, toast: 'Moved even pages to start' });
  }, [opts]);

  const handleMoveOddPagesToEnd = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    await opts.runEdit({ command: 'move_odd_pages_to_end', reloadAt: 0, toast: 'Moved odd pages to end' });
  }, [opts]);

  const handleMoveEvenPagesToEnd = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
    await opts.runEdit({ command: 'move_even_pages_to_end', reloadAt: 0, toast: 'Moved even pages to end' });
  }, [opts]);

  const handleClearCropOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'clear_crop_odd_pages',
      toast: (n) => `Cleared crop on ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowCropModal(false),
    });
  }, [opts]);

  const handleClearCropEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'clear_crop_even_pages',
      toast: (n) => `Cleared crop on ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowCropModal(false),
    });
  }, [opts]);

  const handleDuplicateOddPagesBefore = useCallback(async () => {
    await opts.runEdit({
      command: 'duplicate_odd_pages_before',
      toast: (n) => `Inserted ${n} odd page cop${n === 1 ? 'y' : 'ies'} before originals`,
    });
  }, [opts]);

  const handleDuplicateEvenPagesBefore = useCallback(async () => {
    await opts.runEdit({
      command: 'duplicate_even_pages_before',
      toast: (n) => `Inserted ${n} even page cop${n === 1 ? 'y' : 'ies'} before originals`,
    });
  }, [opts]);

  const handleSortOddPagesByRotation = useCallback(
    async (descending: boolean) => {
      if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
      await opts.withLoading(async () => {
        const sorted = await invoke<number>('sort_odd_pages_by_rotation', { path: opts.filePath, descending });
        if (sorted < 2) {
          opts.showToast('Need at least two odd pages to sort by rotation', 'error');
          return;
        }
        opts.markPdfEdited();
        await opts.reloadOpenPdf(0);
        opts.showToast(
          `Sorted ${sorted} odd page${sorted === 1 ? '' : 's'} by rotation (${descending ? 'largest first' : 'smallest first'})`,
        );
      });
    },
    [opts],
  );

  const handleSortEvenPagesByRotation = useCallback(
    async (descending: boolean) => {
      if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
      await opts.withLoading(async () => {
        const sorted = await invoke<number>('sort_even_pages_by_rotation', { path: opts.filePath, descending });
        if (sorted < 2) {
          opts.showToast('Need at least two even pages to sort by rotation', 'error');
          return;
        }
        opts.markPdfEdited();
        await opts.reloadOpenPdf(0);
        opts.showToast(
          `Sorted ${sorted} even page${sorted === 1 ? '' : 's'} by rotation (${descending ? 'largest first' : 'smallest first'})`,
        );
      });
    },
    [opts],
  );

  const handleSortOddPagesBySize = useCallback(
    async (descending: boolean) => {
      if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
      await opts.withLoading(async () => {
        const sorted = await invoke<number>('sort_odd_pages_by_size', { path: opts.filePath, descending });
        if (sorted < 2) {
          opts.showToast('Need at least two odd pages to sort by size', 'error');
          return;
        }
        opts.markPdfEdited();
        await opts.reloadOpenPdf(0);
        opts.showToast(
          `Sorted ${sorted} odd page${sorted === 1 ? '' : 's'} by size (${descending ? 'largest first' : 'smallest first'})`,
        );
      });
    },
    [opts],
  );

  const handleSortEvenPagesBySize = useCallback(
    async (descending: boolean) => {
      if (!opts.filePath || opts.pageCount === null || opts.pageCount < 2) return;
      await opts.withLoading(async () => {
        const sorted = await invoke<number>('sort_even_pages_by_size', { path: opts.filePath, descending });
        if (sorted < 2) {
          opts.showToast('Need at least two even pages to sort by size', 'error');
          return;
        }
        opts.markPdfEdited();
        await opts.reloadOpenPdf(0);
        opts.showToast(
          `Sorted ${sorted} even page${sorted === 1 ? '' : 's'} by size (${descending ? 'largest first' : 'smallest first'})`,
        );
      });
    },
    [opts],
  );

  const handleSortPagesByRotation = useCallback(
    async (descending: boolean) => {
      await opts.runEdit({
        command: 'sort_pages_by_rotation',
        args: { descending },
        reloadAt: 0,
        toast: `Sorted pages by rotation (${descending ? 'largest first' : 'smallest first'})`,
      });
    },
    [opts],
  );

  return {
    handleReverseOddPages,
    handleReverseEvenPages,
    handleMoveOddPagesToStart,
    handleMoveEvenPagesToStart,
    handleMoveOddPagesToEnd,
    handleMoveEvenPagesToEnd,
    handleClearCropOddPages,
    handleClearCropEvenPages,
    handleDuplicateOddPagesBefore,
    handleDuplicateEvenPagesBefore,
    handleSortOddPagesByRotation,
    handleSortEvenPagesByRotation,
    handleSortOddPagesBySize,
    handleSortEvenPagesBySize,
    handleSortPagesByRotation,
  };
}

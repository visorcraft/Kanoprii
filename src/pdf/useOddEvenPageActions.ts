import { useCallback } from 'react';
import type { RunEdit } from './runEditTypes';

type MarginArgs = {
  marginTop: number;
  marginRight: number;
  marginBottom: number;
  marginLeft: number;
};

type UseOddEvenPageActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  cropMargins: MarginArgs;
  expandMargins: MarginArgs;
  shrinkMargins: MarginArgs;
  runEdit: RunEdit;
  setShowCropRangeModal: (open: boolean) => void;
  setShowExpandMarginsModal: (open: boolean) => void;
  setShowShrinkMarginsModal: (open: boolean) => void;
};

export function useOddEvenPageActions(opts: UseOddEvenPageActionsOptions) {
  const { filePath, pageCount, currentPage, cropMargins, expandMargins, shrinkMargins, runEdit } = opts;

  const handleRotateOddPages = useCallback(async () => {
    await runEdit({ command: 'rotate_odd_pages', toast: (n) => `Rotated ${n} odd page${n === 1 ? '' : 's'} 90° CW` });
  }, [runEdit]);

  const handleRotateEvenPages = useCallback(async () => {
    await runEdit({ command: 'rotate_even_pages', toast: (n) => `Rotated ${n} even page${n === 1 ? '' : 's'} 90° CW` });
  }, [runEdit]);

  const handleRotateOddPagesCcw = useCallback(async () => {
    await runEdit({ command: 'rotate_odd_pages_ccw', toast: (n) => `Rotated ${n} odd page${n === 1 ? '' : 's'} 90° CCW` });
  }, [runEdit]);

  const handleRotateEvenPagesCcw = useCallback(async () => {
    await runEdit({ command: 'rotate_even_pages_ccw', toast: (n) => `Rotated ${n} even page${n === 1 ? '' : 's'} 90° CCW` });
  }, [runEdit]);

  const handleResetRotationOddPages = useCallback(async () => {
    await runEdit({ command: 'reset_rotation_odd_pages', toast: (n) => `Reset rotation on ${n} odd page${n === 1 ? '' : 's'}` });
  }, [runEdit]);

  const handleResetRotationEvenPages = useCallback(async () => {
    await runEdit({ command: 'reset_rotation_even_pages', toast: (n) => `Reset rotation on ${n} even page${n === 1 ? '' : 's'}` });
  }, [runEdit]);

  const handleKeepOddPages = useCallback(async () => {
    if (!filePath || pageCount === null || pageCount < 2) return;
    await runEdit({ command: 'keep_odd_pages', reloadAt: 0, toast: (n) => `Kept odd pages; removed ${n}` });
  }, [runEdit, filePath, pageCount]);

  const handleKeepEvenPages = useCallback(async () => {
    if (!filePath || pageCount === null || pageCount < 2) return;
    await runEdit({ command: 'keep_even_pages', reloadAt: 0, toast: (n) => `Kept even pages; removed ${n}` });
  }, [runEdit, filePath, pageCount]);

  const handleDeleteOddPages = useCallback(async () => {
    if (!filePath || pageCount === null || pageCount < 2) return;
    await runEdit({ command: 'delete_odd_pages', reloadAt: 0, toast: (n) => `Deleted ${n} odd page${n === 1 ? '' : 's'}` });
  }, [runEdit, filePath, pageCount]);

  const handleDeleteEvenPages = useCallback(async () => {
    if (!filePath || pageCount === null || pageCount < 2) return;
    await runEdit({ command: 'delete_even_pages', reloadAt: 0, toast: (n) => `Deleted ${n} even page${n === 1 ? '' : 's'}` });
  }, [runEdit, filePath, pageCount]);

  const handleRotate180OddPages = useCallback(async () => {
    await runEdit({ command: 'rotate_180_odd_pages', toast: (n) => `Rotated ${n} odd page${n === 1 ? '' : 's'} 180°` });
  }, [runEdit]);

  const handleRotate180EvenPages = useCallback(async () => {
    await runEdit({ command: 'rotate_180_even_pages', toast: (n) => `Rotated ${n} even page${n === 1 ? '' : 's'} 180°` });
  }, [runEdit]);

  const handleDuplicateOddPages = useCallback(async () => {
    await runEdit({ command: 'duplicate_odd_pages', reloadAt: (pageCount ?? 1) - 1, toast: (n) => `Appended ${n} odd page cop${n === 1 ? 'y' : 'ies'}` });
  }, [runEdit, pageCount]);

  const handleDuplicateEvenPages = useCallback(async () => {
    await runEdit({ command: 'duplicate_even_pages', reloadAt: (pageCount ?? 1) - 1, toast: (n) => `Appended ${n} even page cop${n === 1 ? 'y' : 'ies'}` });
  }, [runEdit, pageCount]);

  const handleInsertBlankBetweenPages = useCallback(async () => {
    if (!filePath || pageCount === null || pageCount < 2) return;
    await runEdit({ command: 'insert_blank_between_pages', reloadAt: currentPage * 2, toast: (n) => `Inserted ${n} blank page${n === 1 ? '' : 's'} between pages` });
  }, [runEdit, filePath, pageCount, currentPage]);

  const handleFlattenOddPages = useCallback(async () => {
    await runEdit({ command: 'flatten_odd_pages', toast: (n) => `Flattened ${n} annotation${n === 1 ? '' : 's'} on odd pages` });
  }, [runEdit]);

  const handleFlattenEvenPages = useCallback(async () => {
    await runEdit({ command: 'flatten_even_pages', toast: (n) => `Flattened ${n} annotation${n === 1 ? '' : 's'} on even pages` });
  }, [runEdit]);

  const handleRotateAllPages180 = useCallback(async () => {
    await runEdit({ command: 'rotate_all_pages_180', toast: (n) => `Rotated all ${n} page${n === 1 ? '' : 's'} 180°` });
  }, [runEdit]);

  const handleCropOddPages = useCallback(async () => {
    await runEdit({
      command: 'crop_odd_pages',
      args: cropMargins,
      toast: (n) => `Cropped ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowCropRangeModal(false),
    });
  }, [runEdit, cropMargins, opts]);

  const handleCropEvenPages = useCallback(async () => {
    await runEdit({
      command: 'crop_even_pages',
      args: cropMargins,
      toast: (n) => `Cropped ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowCropRangeModal(false),
    });
  }, [runEdit, cropMargins, opts]);

  const handleExpandOddPages = useCallback(async () => {
    await runEdit({
      command: 'expand_odd_pages',
      args: expandMargins,
      toast: (n) => `Expanded margins on ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowExpandMarginsModal(false),
    });
  }, [runEdit, expandMargins, opts]);

  const handleExpandEvenPages = useCallback(async () => {
    await runEdit({
      command: 'expand_even_pages',
      args: expandMargins,
      toast: (n) => `Expanded margins on ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowExpandMarginsModal(false),
    });
  }, [runEdit, expandMargins, opts]);

  const handleShrinkOddPages = useCallback(async () => {
    await runEdit({
      command: 'shrink_odd_pages',
      args: shrinkMargins,
      toast: (n) => `Shrunk margins on ${n} odd page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowShrinkMarginsModal(false),
    });
  }, [runEdit, shrinkMargins, opts]);

  const handleShrinkEvenPages = useCallback(async () => {
    await runEdit({
      command: 'shrink_even_pages',
      args: shrinkMargins,
      toast: (n) => `Shrunk margins on ${n} even page${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowShrinkMarginsModal(false),
    });
  }, [runEdit, shrinkMargins, opts]);

  return {
    handleRotateOddPages,
    handleRotateEvenPages,
    handleRotateOddPagesCcw,
    handleRotateEvenPagesCcw,
    handleResetRotationOddPages,
    handleResetRotationEvenPages,
    handleKeepOddPages,
    handleKeepEvenPages,
    handleDeleteOddPages,
    handleDeleteEvenPages,
    handleRotate180OddPages,
    handleRotate180EvenPages,
    handleDuplicateOddPages,
    handleDuplicateEvenPages,
    handleInsertBlankBetweenPages,
    handleFlattenOddPages,
    handleFlattenEvenPages,
    handleRotateAllPages180,
    handleCropOddPages,
    handleCropEvenPages,
    handleExpandOddPages,
    handleExpandEvenPages,
    handleShrinkOddPages,
    handleShrinkEvenPages,
  };
}

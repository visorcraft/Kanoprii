import { useCallback } from 'react';
import type { RunEdit } from './runEditTypes';

type UseSinglePageEditActionsOptions = {
  filePath: string;
  currentPage: number;
  pageCount: number | null;
  runEdit: RunEdit;
  loadPdfBookmarks: (path: string) => Promise<void>;
};

export function useSinglePageEditActions({
  filePath,
  currentPage,
  pageCount,
  runEdit,
  loadPdfBookmarks,
}: UseSinglePageEditActionsOptions) {
  const handleRotatePageCcw = useCallback(async () => {
    await runEdit({ command: 'rotate_page_ccw', args: { pageIndex: currentPage }, toast: 'Page rotated 90° counter-clockwise' });
  }, [runEdit, currentPage]);

  const handleResetPageRotation = useCallback(async () => {
    await runEdit({ command: 'reset_page_rotation', args: { pageIndex: currentPage }, toast: 'Page rotation reset' });
  }, [runEdit, currentPage]);

  const handleResetAllRotations = useCallback(async () => {
    await runEdit({ command: 'reset_all_page_rotations', toast: (n) => `Reset rotation on ${n} page${n === 1 ? '' : 's'}` });
  }, [runEdit]);

  const handleReversePages = useCallback(async () => {
    if (!filePath || pageCount === null) return;
    await runEdit({ command: 'reverse_pages', reloadAt: pageCount - 1 - currentPage, toast: 'Page order reversed' });
  }, [runEdit, filePath, pageCount, currentPage]);

  const handleRotateAllPages = useCallback(async () => {
    await runEdit({ command: 'rotate_all_pages', toast: (n) => `Rotated ${n} page${n === 1 ? '' : 's'} 90°` });
  }, [runEdit]);

  const handleAddBlankPage = useCallback(async () => {
    await runEdit<number>({
      command: 'add_blank_page',
      args: { atIndex: currentPage + 1 },
      reloadAt: (newIndex) => newIndex,
      toast: (newIndex) => `Blank page inserted at position ${newIndex + 1}`,
    });
  }, [runEdit, currentPage]);

  const handleAddBlankPageBefore = useCallback(async () => {
    await runEdit<number>({
      command: 'add_blank_page',
      args: { atIndex: currentPage },
      reloadAt: (newIndex) => newIndex,
      toast: () => `Blank page inserted before page ${currentPage + 1}`,
    });
  }, [runEdit, currentPage]);

  const handleRotatePage180 = useCallback(async () => {
    await runEdit({ command: 'rotate_page_180', args: { pageIndex: currentPage }, toast: 'Page rotated 180°' });
  }, [runEdit, currentPage]);

  const handleRotateAllPagesCcw = useCallback(async () => {
    await runEdit({ command: 'rotate_all_pages_ccw', toast: (n) => `Rotated ${n} page${n === 1 ? '' : 's'} CCW` });
  }, [runEdit]);

  const handleMovePageToFirst = useCallback(async () => {
    if (!filePath || currentPage === 0) return;
    await runEdit({ command: 'move_page_to_first', args: { pageIndex: currentPage }, reloadAt: 0, toast: 'Page moved to first position' });
  }, [runEdit, filePath, currentPage]);

  const handleMovePageToLast = useCallback(async () => {
    if (!filePath || pageCount === null || currentPage >= pageCount - 1) return;
    await runEdit({
      command: 'move_page_to_last',
      args: { pageIndex: currentPage },
      reloadAt: () => (pageCount ?? 1) - 1,
      toast: 'Page moved to last position',
    });
  }, [runEdit, filePath, pageCount, currentPage]);

  const handleClearAllCrops = useCallback(async () => {
    await runEdit({ command: 'clear_all_page_crops', toast: (n) => `Cleared crop on ${n} page${n === 1 ? '' : 's'}` });
  }, [runEdit]);

  const handleClearAllBookmarks = useCallback(async () => {
    await runEdit({
      command: 'clear_pdf_bookmarks',
      afterEdit: async () => { await loadPdfBookmarks(filePath); },
      toast: (n) => `Removed ${n} bookmark${n === 1 ? '' : 's'}`,
    });
  }, [runEdit, loadPdfBookmarks, filePath]);

  const handleMovePageUp = useCallback(async () => {
    if (!filePath || currentPage === 0) return;
    await runEdit({ command: 'move_page_up', args: { pageIndex: currentPage }, reloadAt: currentPage - 1, toast: `Moved page ${currentPage + 1} up` });
  }, [runEdit, filePath, currentPage]);

  const handleMovePageDown = useCallback(async () => {
    if (!filePath || pageCount === null || currentPage >= pageCount - 1) return;
    await runEdit({ command: 'move_page_down', args: { pageIndex: currentPage }, reloadAt: currentPage + 1, toast: `Moved page ${currentPage + 1} down` });
  }, [runEdit, filePath, pageCount, currentPage]);

  return {
    handleRotatePageCcw,
    handleResetPageRotation,
    handleResetAllRotations,
    handleReversePages,
    handleRotateAllPages,
    handleAddBlankPage,
    handleAddBlankPageBefore,
    handleRotatePage180,
    handleRotateAllPagesCcw,
    handleMovePageToFirst,
    handleMovePageToLast,
    handleClearAllCrops,
    handleClearAllBookmarks,
    handleMovePageUp,
    handleMovePageDown,
  };
}

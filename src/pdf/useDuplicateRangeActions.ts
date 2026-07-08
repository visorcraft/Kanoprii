import { useCallback } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type UseDuplicateRangeActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  duplicateRange: PageRangePairController;
  runEdit: RunEdit;
  setShowDuplicateRangeModal: (open: boolean) => void;
};

export function useDuplicateRangeActions({
  filePath,
  pageCount,
  currentPage,
  duplicateRange,
  runEdit,
  setShowDuplicateRangeModal,
}: UseDuplicateRangeActionsOptions) {
  const openDuplicateRangeModal = useCallback(() => {
    if (!filePath || pageCount === null) return;
    duplicateRange.reset(currentPage, currentPage);
    setShowDuplicateRangeModal(true);
  }, [filePath, pageCount, currentPage, duplicateRange, setShowDuplicateRangeModal]);

  const handleDuplicatePageRange = useCallback(async () => {
    if (!filePath) return;
    const range = duplicateRange.validate();
    if (!range) return;
    await runEdit({
      command: 'duplicate_page_range',
      args: { startPage: duplicateRange.startPage, endPage: duplicateRange.endPage },
      reloadAt: duplicateRange.endPage + 1,
      toast: (n) => `Duplicated ${n} page${n === 1 ? '' : 's'}`,
      onSuccess: () => setShowDuplicateRangeModal(false),
    });
  }, [filePath, duplicateRange, runEdit, setShowDuplicateRangeModal]);

  const handleDuplicatePageRangeToEnd = useCallback(async () => {
    if (!filePath || pageCount === null) return;
    const range = duplicateRange.validate();
    if (!range) return;
    await runEdit<number>({
      command: 'duplicate_page_range_to_end',
      args: { startPage: duplicateRange.startPage, endPage: duplicateRange.endPage },
      reloadAt: (count) => pageCount + count - 1,
      toast: (count) => `Appended ${count} page${count === 1 ? '' : 's'} to end`,
      onSuccess: () => setShowDuplicateRangeModal(false),
    });
  }, [filePath, pageCount, duplicateRange, runEdit, setShowDuplicateRangeModal]);

  const handleDuplicatePageRangeToStart = useCallback(async () => {
    if (!filePath) return;
    const range = duplicateRange.validate();
    if (!range) return;
    await runEdit({
      command: 'duplicate_page_range_to_start',
      args: { startPage: duplicateRange.startPage, endPage: duplicateRange.endPage },
      reloadAt: 0,
      toast: (n) => `Inserted ${n} page${n === 1 ? '' : 's'} at start`,
      onSuccess: () => setShowDuplicateRangeModal(false),
    });
  }, [filePath, duplicateRange, runEdit, setShowDuplicateRangeModal]);

  const handleDuplicatePageRangeBefore = useCallback(async () => {
    if (!filePath) return;
    const range = duplicateRange.validate();
    if (!range) return;
    await runEdit({
      command: 'duplicate_page_range_before',
      args: { startPage: duplicateRange.startPage, endPage: duplicateRange.endPage },
      reloadAt: duplicateRange.startPage,
      toast: (n) => `Inserted ${n} page${n === 1 ? '' : 's'} before range`,
      onSuccess: () => setShowDuplicateRangeModal(false),
    });
  }, [filePath, duplicateRange, runEdit, setShowDuplicateRangeModal]);

  return {
    openDuplicateRangeModal,
    handleDuplicatePageRange,
    handleDuplicatePageRangeToEnd,
    handleDuplicatePageRangeToStart,
    handleDuplicatePageRangeBefore,
  };
}

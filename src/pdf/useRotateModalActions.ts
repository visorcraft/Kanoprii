import { useCallback } from 'react';
import type { PageRangeController } from '../pageRange/usePageRange';
import type { RunEdit } from './runEditTypes';

type RotateDirection = 'cw' | 'ccw';

type UseRotateModalActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  rotateRange: PageRangeController;
  runEdit: RunEdit;
  rotateDirection: RotateDirection;
  setShowRotateModal: (show: boolean) => void;
  setRotateDirection: (dir: RotateDirection) => void;
};

export function useRotateModalActions(opts: UseRotateModalActionsOptions) {
  const openRotateModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.rotateRange.reset({
      scope: 'current',
      start: opts.currentPage,
      end: opts.currentPage,
    });
    opts.setRotateDirection('cw');
    opts.setShowRotateModal(true);
  }, [opts]);

  const openRotateRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.rotateRange.reset({
      scope: 'range',
      start: opts.currentPage,
      end: opts.currentPage,
    });
    opts.setRotateDirection('cw');
    opts.setShowRotateModal(true);
  }, [opts]);

  const handleApplyRotateModal = useCallback(async () => {
    if (!opts.filePath || opts.pageCount === null) return;
    const range = opts.rotateRange.validateAndResolve();
    if (!range) return;

    const scope = opts.rotateRange.scope;
    const ccw = opts.rotateDirection === 'ccw';
    const { start, end } = range;

    if (scope === 'current') {
      await opts.runEdit({
        command: ccw ? 'rotate_page_ccw' : 'rotate_page',
        args: { pageIndex: opts.currentPage },
        toast: ccw ? 'Page rotated 90° counter-clockwise' : 'Page rotated 90°',
        onSuccess: () => opts.setShowRotateModal(false),
      });
      return;
    }

    if (scope === 'all') {
      await opts.runEdit<number>({
        command: ccw ? 'rotate_all_pages_ccw' : 'rotate_all_pages',
        toast: (n) =>
          `Rotated ${n} page${n === 1 ? '' : 's'} 90°${ccw ? ' counter-clockwise' : ''}`,
        onSuccess: () => opts.setShowRotateModal(false),
      });
      return;
    }

    await opts.runEdit<number>({
      command: ccw ? 'rotate_page_range_ccw' : 'rotate_page_range',
      args: { startPage: start, endPage: end },
      toast: (n) =>
        `Rotated ${n} page${n === 1 ? '' : 's'} 90°${ccw ? ' counter-clockwise' : ''}`,
      onSuccess: () => opts.setShowRotateModal(false),
    });
  }, [opts]);

  return {
    openRotateModal,
    openRotateRangeModal,
    handleApplyRotateModal,
  };
}

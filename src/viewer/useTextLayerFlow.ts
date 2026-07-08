import { useCallback, useEffect, useRef } from 'react';
import type { createStructuralEditRunner } from '../pdf/runStructuralEdit';
import { useTextLayerLoader } from '../pdf/useTextLayerLoader';
import { useTextSelection } from './useTextSelection';
import { useTextEditRun } from './useTextEditRun';

type UseTextLayerFlowOptions = {
  filePath: string;
  currentPage: number;
  pdfRevision: number;
  zoom: number;
  editTextRunMode: boolean;
  runEdit: ReturnType<typeof createStructuralEditRunner>;
  annotationModeActive: boolean;
};

export function useTextLayerFlow(opts: UseTextLayerFlowOptions) {
  const pageContainerRef = useRef<HTMLDivElement>(null);
  const { runs } = useTextLayerLoader(opts.filePath, opts.currentPage, opts.pdfRevision);
  const { hasSelection, readSelectionRects, clearSelection } = useTextSelection(pageContainerRef, opts.zoom);

  // Captured selection rects are page-relative; drop them when the page or
  // document changes so a stale capture cannot highlight the wrong page.
  useEffect(() => {
    clearSelection();
  }, [opts.filePath, opts.currentPage, opts.pdfRevision, clearSelection]);
  const textEdit = useTextEditRun({
    filePath: opts.filePath,
    currentPage: opts.currentPage,
    pdfRevision: opts.pdfRevision,
    editTextRunMode: opts.editTextRunMode,
    runEdit: opts.runEdit,
  });

  const highlightSelection = useCallback(() => {
    const rects = readSelectionRects();
    if (rects.length === 0) return;
    void (async () => {
      for (const rect of rects) {
        await opts.runEdit({
          command: 'add_highlight',
          args: {
            path: opts.filePath,
            pageIndex: opts.currentPage,
            x1: rect.x,
            y1: rect.y,
            x2: rect.x + rect.w,
            y2: rect.y + rect.h,
          },
          reloadAt: opts.currentPage,
        });
      }
      clearSelection();
    })();
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath, opts.currentPage, opts.runEdit, readSelectionRects, clearSelection]);

  const textLayerInteractive = !opts.annotationModeActive && !opts.editTextRunMode;

  const handleEditTextRunClick = useCallback(
    (x: number, y: number) => {
      if (!opts.editTextRunMode) return false;
      // Try line-level hit first (v2).
      const line = textEdit.hitTestLine(x, y);
      if (line) {
        const idx = textEdit.lines.findIndex((l) => l === line);
        if (idx >= 0) {
          textEdit.openLineEditor(idx, line);
          return true;
        }
      }
      // Fall back to run-level (v1).
      const run = textEdit.hitTestRun(runs, x, y);
      if (!run) return false;
      textEdit.openRunEditor(run);
      return true;
    },
    [opts.editTextRunMode, runs, textEdit],
  );

  return {
    pageContainerRef,
    textRuns: runs,
    textLayerInteractive,
    hasTextSelection: hasSelection && textLayerInteractive,
    highlightSelection,
    handleEditTextRunClick,
    textEditActiveRun: textEdit.activeRun,
    textEditActiveLine: textEdit.activeLine,
    textEditDraft: textEdit.draft,
    setTextEditDraft: textEdit.setDraft,
    applyTextEdit: textEdit.applyEdit,
    cancelTextEdit: textEdit.cancelEdit,
  };
}

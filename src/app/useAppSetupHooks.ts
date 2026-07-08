import { useAppBootstrap } from './useAppBootstrap';
import { usePageZoomInputSync } from './usePageZoomInputSync';
import { useWindowTitle } from './useWindowTitle';
import { useSourcePdfPageCounts } from './useSourcePdfPageCounts';
import type { DocumentState } from './useAppDocumentState';
import type { ModalState } from './useAppModalState';
import type { PageRangesState } from './useAppPageRanges';
import type { RefsState } from './useAppRefs';
import type { HelpState } from './useHelpChromeState';

export type UseAppSetupHooksInput = {
  doc: DocumentState;
  modal: ModalState;
  help: HelpState;
  pageRanges: PageRangesState;
  refs: Pick<RefsState, 'filePathRef'>;
  onShowTesseractReminder: () => void;
};

export function useAppSetupHooks(input: UseAppSetupHooksInput) {
  useSourcePdfPageCounts({
    insertFilePath: input.modal.insertFilePath,
    mergeFilePath: input.modal.mergeFilePath,
    insertRange: input.pageRanges.insertRange,
    mergeRange: input.pageRanges.mergeRange,
    setInsertSourcePageCount: input.modal.setInsertSourcePageCount,
    setMergeSourcePageCount: input.modal.setMergeSourcePageCount,
  });

  useAppBootstrap({
    onNativeDialogs: input.modal.setNativeDialogs,
    onOcrAvailable: input.doc.setOcrAvailable,
    onTesseractInstallGuide: input.help.setTesseractInstallGuide,
    onShowTesseractReminder: input.onShowTesseractReminder,
    onUpdaterSupported: input.help.setUpdaterSupported,
  });

  const { windowTitle } = useWindowTitle({
    filePath: input.doc.filePath,
    originalPath: input.doc.originalPath,
    isDirty: input.doc.isDirty,
    isDirtyRef: input.doc.isDirtyRef,
    filePathRef: input.refs.filePathRef,
  });

  usePageZoomInputSync({
    currentPage: input.doc.currentPage,
    setPageInput: input.doc.setPageInput,
    zoom: input.doc.zoom,
    setZoomInput: input.doc.setZoomInput,
  });

  return { windowTitle };
}

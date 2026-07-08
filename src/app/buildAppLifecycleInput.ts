import type { DocumentState } from './useAppDocumentState';
import type { ModalState } from './useAppModalState';
import type { SecurityState } from './useSecurityFormState';
import type { PanelsState } from './useDocumentPanelsState';
import type { AnnotationState } from './useAnnotationDraftState';
import type { RefsState } from './useAppRefs';
import type { HelpState } from './useHelpChromeState';
import type { PageRangesState } from './useAppPageRanges';
import type { UseAppLifecycleHooksInput } from './appLifecycleTypes';

export type BuildAppLifecycleInputArgs = {
  doc: DocumentState;
  modal: ModalState;
  security: SecurityState;
  panels: PanelsState;
  annotation: AnnotationState;
  refs: RefsState;
  pageRanges: PageRangesState;
  ocrAvailable: boolean | null;
  tesseractReminderSource: HelpState['tesseractReminderSource'];
  setTesseractReminderSource: HelpState['setTesseractReminderSource'];
  tesseractDoNotRemind: boolean;
  setTesseractDoNotRemind: HelpState['setTesseractDoNotRemind'];
  setShowTesseractModal: HelpState['setShowTesseractModal'];
  showToast: (message: string, type?: 'success' | 'error') => void;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  cancelDrawing: () => void;
};

export function buildAppLifecycleInput(args: BuildAppLifecycleInputArgs): UseAppLifecycleHooksInput {
  return {
    doc: args.doc,
    modal: args.modal,
    security: args.security,
    panels: args.panels,
    annotation: args.annotation,
    refs: args.refs,
    pageRanges: args.pageRanges,
    ocrAvailable: args.ocrAvailable,
    tesseractReminderSource: args.tesseractReminderSource,
    setTesseractReminderSource: args.setTesseractReminderSource,
    tesseractDoNotRemind: args.tesseractDoNotRemind,
    setTesseractDoNotRemind: args.setTesseractDoNotRemind,
    setShowTesseractModal: args.setShowTesseractModal,
    showToast: args.showToast,
    withLoading: args.withLoading,
    filePathRef: args.refs.filePathRef,
    cancelDrawing: args.cancelDrawing,
  };
}

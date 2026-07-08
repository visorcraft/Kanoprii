import type { DocumentState } from './useAppDocumentState';
import type { ModalState } from './useAppModalState';
import type { SecurityState } from './useSecurityFormState';
import type { PanelsState } from './useDocumentPanelsState';
import type { AnnotationState } from './useAnnotationDraftState';
import type { RefsState } from './useAppRefs';
import type { HelpState } from './useHelpChromeState';
import type { PageRangesState } from './useAppPageRanges';
import type { useAppLifecycleLoaders } from './useAppLifecycleLoaders';

export type UseAppLifecycleHooksInput = {
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
  filePathRef: RefsState['filePathRef'];
  cancelDrawing: () => void;
};

export type UseAppLifecycleDocumentInput = {
  input: UseAppLifecycleHooksInput;
  loaders: ReturnType<typeof useAppLifecycleLoaders>;
};

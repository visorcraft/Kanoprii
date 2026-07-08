import { useAppPdfActions, type UseAppPdfActionsInput } from './useAppPdfActions';
import type { AnnotationState } from './useAnnotationDraftState';
import type { DocumentState } from './useAppDocumentState';
import type { ModalState } from './useAppModalState';
import type { PageRangesState } from './useAppPageRanges';
import type { RefsState } from './useAppRefs';
import type { PanelsState } from './useDocumentPanelsState';
import type { HelpState } from './useHelpChromeState';
import type { SecurityState } from './useSecurityFormState';
import {
  annotationPdfActionFields,
  documentPdfActionFields,
  drawingPdfActionFields,
  marginPdfActionFields,
  modalPdfActionFields,
  pageRangesPdfActionFields,
  panelsPdfActionFields,
  refsPdfActionFields,
  securityPdfActionFields,
  type DrawingGestureSlice,
} from './buildAppPdfActionsFields';

type AppPdfActionsRuntime = Pick<
  UseAppPdfActionsInput,
  | 'loadFormFields'
  | 'loadPageSizes'
  | 'loadPdfBookmarks'
  | 'loadPdfSignatures'
  | 'loadThumbnails'
  | 'markPdfEdited'
  | 'markSaved'
  | 'reloadOpenPdf'
  | 'rememberBrowserDirectory'
  | 'rememberOpenedPdf'
  | 'renderPage'
  | 'setAnnotations'
  | 'shouldShowTesseractReminder'
  | 'showToast'
  | 'withLoading'
  | 'setShowTesseractModal'
  | 'setTesseractReminderSource'
>;

type AppPdfActionsRuntimeExtras = {
  openTesseractGuide: () => void;
};

export type AppPdfActionsRuntimeSlice = Omit<AppPdfActionsRuntime, 'setShowTesseractModal' | 'setTesseractReminderSource'>
  & AppPdfActionsRuntimeExtras;

export type { DrawingGestureSlice };

export type UseAppPdfActionsBindingInput = {
  doc: DocumentState;
  modal: ModalState;
  security: SecurityState;
  panels: PanelsState;
  annotation: AnnotationState;
  drawing: DrawingGestureSlice;
  pageRanges: PageRangesState;
  refs: Pick<RefsState, 'cancelDrawingRef' | 'handleSaveRef' | 'handleMarkdownViewRef' | 'imgRef'>;
  help: Pick<HelpState, 'setShowTesseractModal' | 'setTesseractReminderSource'>;
  runtime: AppPdfActionsRuntimeSlice;
};

export function useAppPdfActionsBinding(input: UseAppPdfActionsBindingInput) {
  const { modal: m, security: s, panels: p, annotation: a, doc: d, drawing: g, pageRanges: r, refs, help, runtime } = input;

  return useAppPdfActions({
    ...modalPdfActionFields(m),
    ...securityPdfActionFields(s),
    ...panelsPdfActionFields(p),
    ...annotationPdfActionFields(a),
    ...documentPdfActionFields(d),
    ...drawingPdfActionFields(g),
    ...pageRangesPdfActionFields(r),
    ...refsPdfActionFields(refs),
    ...marginPdfActionFields(m),
    extractEndPage: r.extractRange.endPage,
    extractStartPage: r.extractRange.startPage,
    pngExportEndPage: r.pngExportRange.endPage,
    pngExportScope: r.pngExportRange.scope,
    pngExportStartPage: r.pngExportRange.startPage,
    ...runtime,
    setShowTesseractModal: help.setShowTesseractModal,
    setTesseractReminderSource: help.setTesseractReminderSource,
    openTesseractGuide: runtime.openTesseractGuide,
  });
}

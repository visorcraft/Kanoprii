import { useCallback, useEffect } from 'react';
import {
  useAppPdfActions,
  type UseAppPdfActionsInput,
} from './useAppPdfActions';
import type { AnnotationState } from './useAnnotationDraftState';
import type { PdfEditState } from './usePdfEditState';
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

export type AppPdfActionsRuntimeSlice = Omit<
  AppPdfActionsRuntime,
  'setShowTesseractModal' | 'setTesseractReminderSource'
> &
  AppPdfActionsRuntimeExtras;

export type { DrawingGestureSlice };

export type UseAppPdfActionsBindingInput = {
  doc: DocumentState;
  modal: ModalState;
  security: SecurityState;
  panels: PanelsState;
  annotation: AnnotationState;
  pdfEdit: PdfEditState;
  drawing: DrawingGestureSlice;
  pageRanges: PageRangesState;
  refs: Pick<
    RefsState,
    'cancelDrawingRef' | 'handleSaveRef' | 'handleMarkdownViewRef' | 'imgRef'
  >;
  help: Pick<HelpState, 'setShowTesseractModal' | 'setTesseractReminderSource'>;
  runtime: AppPdfActionsRuntimeSlice;
};

export function useAppPdfActionsBinding(input: UseAppPdfActionsBindingInput) {
  const {
    modal: m,
    security: s,
    panels: p,
    annotation: a,
    pdfEdit,
    doc: d,
    drawing: g,
    pageRanges: r,
    refs,
    help,
    runtime,
  } = input;

  const pdfActions = useAppPdfActions({
    ...modalPdfActionFields(m),
    pageSizes: m.pageSizes,
    ...securityPdfActionFields(s),
    ...panelsPdfActionFields(p),
    ...annotationPdfActionFields(a),
    clearEditMode: pdfEdit.clearEditMode,
    setEditMode: pdfEdit.setEditMode,
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
    pdfEdit,
    sessions: d.sessions,
    activeId: d.activeId,
  });

  const activeSession = d.sessions.find((s) => s.id === d.activeId);
  const {
    pdfEditApplyText,
    pdfEditApplyParagraph,
    pdfEditDeleteText,
    pdfEditDeleteParagraph,
    pdfEditApplyImage,
    pdfEditDeleteImage,
    pdfEditReplaceImage,
    pdfEditApplyVector,
    pdfEditDeleteVector,
  } = pdfActions;

  const onApplyText = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditApplyText(activeSession);
  }, [activeSession, pdfEditApplyText]);

  const onApplyParagraph = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditApplyParagraph(activeSession);
  }, [activeSession, pdfEditApplyParagraph]);

  const onApplyImage = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditApplyImage(activeSession);
  }, [activeSession, pdfEditApplyImage]);

  const onDeleteText = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditDeleteText(activeSession);
  }, [activeSession, pdfEditDeleteText]);

  const onDeleteParagraph = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditDeleteParagraph(activeSession);
  }, [activeSession, pdfEditDeleteParagraph]);

  const onDeleteImage = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditDeleteImage(activeSession);
  }, [activeSession, pdfEditDeleteImage]);

  const onReplaceImage = useCallback(() => pdfEditReplaceImage(), [pdfEditReplaceImage]);

  const onApplyVector = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditApplyVector(activeSession);
  }, [activeSession, pdfEditApplyVector]);

  const onDeleteVector = useCallback(() => {
    if (!activeSession) return Promise.resolve();
    return pdfEditDeleteVector(activeSession);
  }, [activeSession, pdfEditDeleteVector]);

  useEffect(() => {
    pdfEdit.bindEditCallbacks({
      onApplyText,
      onApplyParagraph,
      onDeleteText,
      onDeleteParagraph,
      onApplyImage,
      onDeleteImage,
      onReplaceImage,
      onApplyVector,
      onDeleteVector,
    });
  }, [pdfEdit, onApplyText, onApplyParagraph, onDeleteText, onDeleteParagraph, onApplyImage, onDeleteImage, onReplaceImage, onApplyVector, onDeleteVector]);

  return pdfActions;
}

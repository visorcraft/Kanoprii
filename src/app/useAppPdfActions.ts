import { useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open as openNativeDialog } from '@tauri-apps/plugin-dialog';
import { useStructuralEdit } from '../pdf/useStructuralEdit';
import { useImageExportActions } from '../pdf/useImageExportActions';
import { usePdfModalOpeners } from '../pdf/usePdfModalOpeners';
import { useSinglePageEditActions } from '../pdf/useSinglePageEditActions';
import { useDuplicateRangeActions } from '../pdf/useDuplicateRangeActions';
import { usePageHeaderFooterActions } from '../pdf/usePageHeaderFooterActions';
import { useSwapReplaceInterleaveActions } from '../pdf/useSwapReplaceInterleaveActions';
import { usePageSizeActions } from '../pdf/usePageSizeActions';
import { useExportPagesActions } from '../pdf/useExportPagesActions';
import { useParityExportActions } from '../pdf/useParityExportActions';
import { useRangeModalActions } from '../pdf/useRangeModalActions';
import { useRotateModalActions } from '../pdf/useRotateModalActions';
import { useOddEvenPageActions } from '../pdf/useOddEvenPageActions';
import { useOddEvenExtendedActions } from '../pdf/useOddEvenExtendedActions';
import { useSplitExtractPrependActions } from '../pdf/useSplitExtractPrependActions';
import { usePageDecorActions } from '../pdf/usePageDecorActions';
import { useBookmarkActions } from '../pdf/useBookmarkActions';
import { usePdfFileOpsActions } from '../pdf/usePdfFileOpsActions';
import { usePageDuplicateActions } from '../pdf/usePageDuplicateActions';
import { useFormFieldActions } from '../pdf/useFormFieldActions';
import { usePdfRevisionSync } from './usePdfRevisionSync';
import { usePageInteraction } from '../viewer/usePageInteraction';
import { useTextLayerFlow } from '../viewer/useTextLayerFlow';
import { useAnnotationModesAsset } from './useAnnotationModesAsset';
import { useAnnotationModesMarkup } from './useAnnotationModesMarkup';
import { usePageTextEdits } from './usePageTextEdits';
import { useNotePasswordActions } from '../pdf/useNotePasswordActions';
import { useNativeFilePickers } from './useNativeFilePickers';
import { useSaveActions } from '../pdf/useSaveActions';
import { useMarkdownFlow } from './useMarkdownFlow';
import { useSecurityDocumentActions } from '../pdf/useSecurityDocumentActions';
import {
  useDocumentEnhancementActions,
  type UseDocumentEnhancementActionsOptions,
} from '../pdf/useDocumentEnhancementActions';
import { usePageInteractionEdit } from '../viewer/usePageInteractionEdit';
import {
  BMP_DIALOG_FILTER,
  GIF_DIALOG_FILTER,
  JPEG_DIALOG_FILTER,
  PNG_DIALOG_FILTER,
  TIFF_DIALOG_FILTER,
  VIEWER_PAGE_H,
  VIEWER_PAGE_W,
  WEBP_DIALOG_FILTER,
} from './constants';
import type { DocumentSessionData } from './documentSessionTypes';
import type { PdfEditState, Rect } from './usePdfEditState';
import type { PdfPageSize } from './types';

const IMAGE_DIALOG_FILTERS = [
  ...PNG_DIALOG_FILTER,
  ...JPEG_DIALOG_FILTER,
  ...WEBP_DIALOG_FILTER,
  ...BMP_DIALOG_FILTER,
  ...TIFF_DIALOG_FILTER,
  ...GIF_DIALOG_FILTER,
];

type HookOpts<H extends (...args: never) => unknown> = Parameters<H>[0];

type AllHookOpts = HookOpts<typeof usePdfModalOpeners> &
  HookOpts<typeof useImageExportActions> &
  HookOpts<typeof useStructuralEdit> &
  HookOpts<typeof useSinglePageEditActions> &
  HookOpts<typeof useDuplicateRangeActions> &
  HookOpts<typeof usePageHeaderFooterActions> &
  HookOpts<typeof useSwapReplaceInterleaveActions> &
  HookOpts<typeof usePageSizeActions> &
  HookOpts<typeof useExportPagesActions> &
  HookOpts<typeof useParityExportActions> &
  HookOpts<typeof useRangeModalActions> &
  HookOpts<typeof useRotateModalActions> &
  HookOpts<typeof useOddEvenPageActions> &
  HookOpts<typeof useOddEvenExtendedActions> &
  HookOpts<typeof useSplitExtractPrependActions> &
  HookOpts<typeof usePageDecorActions> &
  HookOpts<typeof useBookmarkActions> &
  HookOpts<typeof usePdfFileOpsActions> &
  HookOpts<typeof usePageDuplicateActions> &
  HookOpts<typeof useFormFieldActions> &
  HookOpts<typeof usePdfRevisionSync> &
  HookOpts<typeof usePageInteraction> &
  HookOpts<typeof useAnnotationModesAsset> &
  HookOpts<typeof useAnnotationModesMarkup> &
  HookOpts<typeof usePageTextEdits> &
  HookOpts<typeof useNotePasswordActions> &
  HookOpts<typeof useNativeFilePickers> &
  HookOpts<typeof useSaveActions> &
  HookOpts<typeof useMarkdownFlow> &
  HookOpts<typeof useSecurityDocumentActions>;

export type UseAppPdfActionsInput = Omit<
  AllHookOpts,
  | 'runEdit'
  | 'defaultExtractOutputPath'
  | 'defaultImageExportOutput'
  | 'saveAsViaNativeDialog'
  | 'exitNoteMode'
  | 'refreshAnnotations'
  | 'pdfEdit'
  | 'session'
  | 'handleEditPageClick'
  | 'hitTestImage'
> &
  Pick<
    UseDocumentEnhancementActionsOptions,
    | 'ocrAvailable'
    | 'batesRange'
    | 'batesPrefix'
    | 'batesStartNumber'
    | 'batesDigits'
    | 'batesPosition'
    | 'applyRedactionsOcrAfter'
    | 'setShowBatesNumberModal'
    | 'setShowApplyRedactionsModal'
    | 'setBatesPrefix'
    | 'setBatesStartNumber'
    | 'setBatesDigits'
    | 'setBatesPosition'
  > & {
    cancelDrawingRef: { current: () => void };
    handleSaveRef: { current: () => void | Promise<void> };
    handleMarkdownViewRef: { current: () => void | Promise<void> };
    openTesseractGuide: () => void;
    pdfEdit: PdfEditState;
    sessions: DocumentSessionData[];
    activeId: string | null;
    pageSizes: PdfPageSize[];
  };

function call<H extends (opts: never) => unknown>(
  hook: H,
  input: object
): ReturnType<H> {
  return hook(input as Parameters<H>[0]) as ReturnType<H>;
}

export type AppPdfActions = ReturnType<typeof useAppPdfActions>;

export function useAppPdfActions(input: UseAppPdfActionsInput) {
  const modalOpeners = call(usePdfModalOpeners, input);
  const imageExport = call(useImageExportActions, input);
  const runEdit = call(useStructuralEdit, input);
  const withRunEdit = { ...input, runEdit };
  const singlePage = call(useSinglePageEditActions, withRunEdit);
  const duplicateRange = call(useDuplicateRangeActions, withRunEdit);
  const headerFooter = call(usePageHeaderFooterActions, withRunEdit);
  const swapReplace = call(useSwapReplaceInterleaveActions, withRunEdit);
  const pageSize = call(usePageSizeActions, withRunEdit);
  const exportPages = call(useExportPagesActions, input);
  const parityExport = call(useParityExportActions, withRunEdit);
  const rangeModals = call(useRangeModalActions, withRunEdit);
  const rotateModalActions = call(useRotateModalActions, withRunEdit);
  const oddEven = call(useOddEvenPageActions, withRunEdit);
  const oddEvenExt = call(useOddEvenExtendedActions, withRunEdit);
  const splitExtract = call(useSplitExtractPrependActions, withRunEdit);
  const pageDecor = call(usePageDecorActions, withRunEdit);
  const bookmarkActions = call(useBookmarkActions, withRunEdit);
  const fileOps = call(usePdfFileOpsActions, input);
  const pageDuplicate = call(usePageDuplicateActions, withRunEdit);
  const formField = call(useFormFieldActions, input);
  call(usePdfRevisionSync, input);
  input.cancelDrawingRef.current = input.cancelDrawing;
  const annotationModes = {
    ...call(useAnnotationModesAsset, input),
    ...call(useAnnotationModesMarkup, input),
  };
  const textLayerFlow = useTextLayerFlow({
    filePath: input.filePath,
    currentPage: input.currentPage,
    pdfRevision: input.pdfRevision,
    zoom: input.zoom,
    editTextRunMode: input.editTextRunMode ?? false,
    runEdit,
    annotationModeActive:
      input.highlightMode ||
      input.noteMode ||
      input.drawMode ||
      input.shapeMode ||
      input.stampMode ||
      input.redactMode ||
      input.imageInsertMode ||
      input.textEditMode ||
      input.editTextRunMode ||
      input.vectorEditMode ||
      input.formAddMode ||
      input.pdfEdit.editMode,
  });
  const activeSession = useMemo(
    () => input.sessions.find((s) => s.id === input.activeId) ?? null,
    [input.sessions, input.activeId],
  );

  const pickEditImage = useCallback(async () => {
    if (!input.nativeDialogs) return window.prompt('Image path')?.trim() ?? '';
    const selected = await openNativeDialog({
      multiple: false,
      directory: false,
      filters: IMAGE_DIALOG_FILTERS,
    });
    if (selected === null) return '';
    return typeof selected === 'string' ? selected : selected[0] ?? '';
  }, [input.nativeDialogs]);

  const insertEditImage = useCallback(async () => {
    const filePath = input.filePath;
    if (!filePath) return;
    const pageIndex = input.currentPage;

    const imagePath = await pickEditImage();
    if (!imagePath) return;

    let dimensions: [number, number];
    try {
      dimensions = await invoke<[number, number]>('get_image_dimensions', { path: imagePath });
    } catch (err) {
      input.showToast(String(err), 'error');
      return;
    }

    const [imgWidth, imgHeight] = dimensions;
    if (imgWidth === 0) {
      input.showToast('Could not read image dimensions', 'error');
      return;
    }

    let defaultWidth = 200;
    let defaultHeight = (imgHeight / imgWidth) * defaultWidth;
    const scale = Math.min(1, VIEWER_PAGE_H / defaultHeight, VIEWER_PAGE_W / defaultWidth);
    defaultWidth *= scale;
    defaultHeight *= scale;

    const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
    const x = clamp((VIEWER_PAGE_W - defaultWidth) / 2, 0, VIEWER_PAGE_W - defaultWidth);
    const y = clamp((VIEWER_PAGE_H - defaultHeight) / 2, 0, VIEWER_PAGE_H - defaultHeight);

    const viewerRect: Rect = { x, y, w: defaultWidth, h: defaultHeight };

    await runEdit({
      command: 'add_page_image',
      args: {
        pageIndex,
        x: viewerRect.x,
        y: viewerRect.y,
        width: viewerRect.w,
        height: viewerRect.h,
        imagePath,
      },
      reloadAt: pageIndex,
      toast: 'Image inserted',
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [input.filePath, input.currentPage, input.showToast, pickEditImage, runEdit]);

  const replaceEditImage = useCallback(async () => {
    const draft = input.pdfEdit.imageDraft;
    if (!input.filePath || !draft) return;
    const imagePath = await pickEditImage();
    if (!imagePath) return;
    await runEdit({
      command: 'replace_page_image',
      args: {
        pageIndex: draft.pageIndex,
        imageIndex: draft.index,
        imagePath,
      },
      reloadAt: draft.pageIndex,
      toast: 'Image replaced',
    });
  }, [input.filePath, input.pdfEdit, pickEditImage, runEdit]);

  const editInteraction = usePageInteractionEdit({
    pdfEdit: input.pdfEdit,
    filePath: input.filePath,
    currentPage: input.currentPage,
    pageSizes: input.pageSizes,
    withLoading: input.withLoading,
    markPdfEdited: input.markPdfEdited,
    reloadOpenPdf: input.reloadOpenPdf,
    showToast: input.showToast,
  });

  const pageInteraction = call(usePageInteraction, {
    ...withRunEdit,
    pdfEdit: input.pdfEdit,
    session: activeSession,
    handleEditPageClick: editInteraction.handlePageClick,
    hitTestImage: editInteraction.hitTestImage,
    editTextRunMode: input.editTextRunMode ?? false,
    handleEditTextRunClick: textLayerFlow.handleEditTextRunClick,
  });
  const pageTextEdits = call(usePageTextEdits, input);
  const nativePickers = call(useNativeFilePickers, {
    ...input,
    defaultExtractOutputPath: modalOpeners.defaultExtractOutputPath,
    defaultImageExportOutput: imageExport.defaultImageExportOutput,
  });
  const saveActions = call(useSaveActions, {
    ...input,
    saveAsViaNativeDialog: nativePickers.saveAsViaNativeDialog,
  });
  const notePassword = call(useNotePasswordActions, {
    ...input,
    refreshAnnotations: pageInteraction.refreshAnnotations,
    exitNoteMode: annotationModes.exitNoteMode,
  });
  input.handleSaveRef.current = saveActions.handleSave;
  const markdownFlow = call(useMarkdownFlow, input);
  input.handleMarkdownViewRef.current = markdownFlow.handleMarkdownView;
  const securityDocs = call(useSecurityDocumentActions, withRunEdit);
  const documentEnhancement = useDocumentEnhancementActions({
    filePath: input.filePath,
    pageCount: input.pageCount,
    currentPage: input.currentPage,
    pdfRevision: input.pdfRevision,
    ocrAvailable: input.ocrAvailable,
    batesRange: input.batesRange,
    batesPrefix: input.batesPrefix,
    batesStartNumber: input.batesStartNumber,
    batesDigits: input.batesDigits,
    batesPosition: input.batesPosition,
    applyRedactionsOcrAfter: input.applyRedactionsOcrAfter,
    runEdit,
    showToast: input.showToast,
    openTesseractGuide: input.openTesseractGuide,
    setShowBatesNumberModal: input.setShowBatesNumberModal,
    setShowApplyRedactionsModal: input.setShowApplyRedactionsModal,
    setBatesPrefix: input.setBatesPrefix,
    setBatesStartNumber: input.setBatesStartNumber,
    setBatesDigits: input.setBatesDigits,
    setBatesPosition: input.setBatesPosition,
  });

  return {
    runEdit,
    ...modalOpeners,
    ...imageExport,
    ...singlePage,
    ...duplicateRange,
    ...headerFooter,
    ...swapReplace,
    ...pageSize,
    ...exportPages,
    ...parityExport,
    ...rangeModals,
    ...rotateModalActions,
    ...oddEven,
    ...oddEvenExt,
    ...splitExtract,
    ...pageDecor,
    ...bookmarkActions,
    ...fileOps,
    ...pageDuplicate,
    applyFormField: formField.applyFormField,
    ...pageInteraction,
    ...textLayerFlow,
    pdfEditApplyText: editInteraction.applyTextEdit,
    pdfEditApplyParagraph: editInteraction.applyParagraphEdit,
    pdfEditDeleteText: editInteraction.deleteText,
    pdfEditDeleteParagraph: editInteraction.deleteParagraph,
    pdfEditApplyImage: editInteraction.applyImageEdit,
    pdfEditDeleteImage: editInteraction.deleteImage,
    pdfEditReplaceImage: replaceEditImage,
    insertEditImage,
    ...annotationModes,
    ...pageTextEdits,
    ...notePassword,
    ...nativePickers,
    ...saveActions,
    ...markdownFlow,
    ...securityDocs,
    ...documentEnhancement,
  };
}

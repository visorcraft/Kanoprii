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
      input.formAddMode,
  });
  const pageInteraction = call(usePageInteraction, {
    ...withRunEdit,
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

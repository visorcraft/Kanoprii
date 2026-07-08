import type { BuildAppKeyboardActionsInput } from './buildAppKeyboardActions';
import type { AppPdfActions } from './useAppPdfActions';
import type { DocumentState } from './useAppDocumentState';
import type { AnnotationState } from './useAnnotationDraftState';

export type BuildAppKeyboardSourceArgs = {
  doc: Pick<DocumentState, 'isDirty' | 'filePath' | 'pageCount' | 'currentPage' | 'viewMode' | 'cycleTab' | 'jumpToTab'>;
  annotation: Pick<AnnotationState,
    'noteMode' | 'drawMode' | 'shapeMode' | 'stampMode' | 'redactMode' | 'imageInsertMode'
    | 'textEditMode' | 'vectorEditMode' | 'formAddMode' | 'highlightMode'>;
  history: { canUndo: boolean; canRedo: boolean; undo: () => void; redo: () => void };
  chrome: {
    anyModalOpen: boolean;
    dismissModals: () => void;
    guardUnsaved: (action: () => void) => void;
    closePdf: () => void;
    exitApp: () => void;
    openPdf: () => void;
    setShowCommandPalette: (open: boolean) => void;
    goToPage: (page: number) => void;
    handlePrint: () => void;
    openPrintDialog: () => void;
    openSearchModal: () => void;
  };
  zoom: { zoomIn: () => void; zoomOut: () => void; resetZoom: () => void };
  pdfActions: AppPdfActions;
};

export function buildAppKeyboardSource(args: BuildAppKeyboardSourceArgs): BuildAppKeyboardActionsInput {
  return {
    isDirty: args.doc.isDirty,
    canUndo: args.history.canUndo,
    canRedo: args.history.canRedo,
    filePath: args.doc.filePath,
    noteMode: args.annotation.noteMode,
    drawMode: args.annotation.drawMode,
    shapeMode: args.annotation.shapeMode,
    stampMode: args.annotation.stampMode,
    redactMode: args.annotation.redactMode,
    imageInsertMode: args.annotation.imageInsertMode,
    textEditMode: args.annotation.textEditMode,
    vectorEditMode: args.annotation.vectorEditMode,
    formAddMode: args.annotation.formAddMode,
    highlightMode: args.annotation.highlightMode,
    anyModalOpen: args.chrome.anyModalOpen,
    pageCount: args.doc.pageCount,
    currentPage: args.doc.currentPage,
    viewMode: args.doc.viewMode,
    openPdf: args.chrome.openPdf,
    setShowCommandPalette: args.chrome.setShowCommandPalette,
    dismissModals: args.chrome.dismissModals,
    guardUnsaved: args.chrome.guardUnsaved,
    closePdf: args.chrome.closePdf,
    exitApp: args.chrome.exitApp,
    handlePrint: args.chrome.handlePrint,
    openPrintDialog: args.chrome.openPrintDialog,
    openSearchModal: args.chrome.openSearchModal,
    goToPage: args.chrome.goToPage,
    zoomIn: args.zoom.zoomIn,
    zoomOut: args.zoom.zoomOut,
    resetZoom: args.zoom.resetZoom,
    undo: args.history.undo,
    redo: args.history.redo,
    cycleTab: args.doc.cycleTab,
    jumpToTab: args.doc.jumpToTab,
    exitDrawMode: args.pdfActions.exitDrawMode,
    exitFormAddMode: args.pdfActions.exitFormAddMode,
    exitHighlightMode: args.pdfActions.exitHighlightMode,
    exitImageInsertMode: args.pdfActions.exitImageInsertMode,
    exitNoteMode: args.pdfActions.exitNoteMode,
    exitRedactMode: args.pdfActions.exitRedactMode,
    exitShapeMode: args.pdfActions.exitShapeMode,
    exitStampMode: args.pdfActions.exitStampMode,
    exitTextEditMode: args.pdfActions.exitTextEditMode,
    exitVectorEditMode: args.pdfActions.exitVectorEditMode,
    handleAddBlankPage: args.pdfActions.handleAddBlankPage,
    handleDuplicatePage: args.pdfActions.handleDuplicatePage,
    handleOptimizePdf: args.pdfActions.handleOptimizePdf,
    handleReversePages: args.pdfActions.handleReversePages,
    handleRotatePage: args.pdfActions.handleRotatePage,
    handleSave: args.pdfActions.handleSave,
    handleSummarizePdf: args.pdfActions.handleSummarizePdf,
    openDeleteModal: args.pdfActions.openDeleteModal,
    openExportPngModal: args.pdfActions.openExportPngModal,
    openExtractModal: args.pdfActions.openExtractModal,
    openInsertModal: args.pdfActions.openInsertModal,
    openMergeModal: args.pdfActions.openMergeModal,
    openSaveAs: args.pdfActions.openSaveAs,
    openSignModal: args.pdfActions.openSignModal,
    openSplitModal: args.pdfActions.openSplitModal,
    toggleDrawMode: args.pdfActions.toggleDrawMode,
    toggleFormsPanel: args.pdfActions.toggleFormsPanel,
    toggleHighlightMode: args.pdfActions.toggleHighlightMode,
    toggleImageInsertMode: args.pdfActions.toggleImageInsertMode,
    toggleMarkdownView: args.pdfActions.toggleMarkdownView,
    toggleNoteMode: args.pdfActions.toggleNoteMode,
    toggleRedactMode: args.pdfActions.toggleRedactMode,
    toggleShapeMode: args.pdfActions.toggleShapeMode,
    toggleStampMode: args.pdfActions.toggleStampMode,
    toggleTextEditMode: args.pdfActions.toggleTextEditMode,
    toggleVectorEditMode: args.pdfActions.toggleVectorEditMode,
  };
}

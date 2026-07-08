import type { ViewMode } from './types';

export type AppKeyboardActions = {
  isDirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  hasOpenPdf: boolean;
  noteMode: boolean;
  drawMode: boolean;
  shapeMode: boolean;
  stampMode: boolean;
  redactMode: boolean;
  imageInsertMode: boolean;
  textEditMode: boolean;
  vectorEditMode: boolean;
  formAddMode: boolean;
  highlightMode: boolean;
  anyModalOpen: boolean;
  pageCount: number | null;
  currentPage: number;
  viewMode: ViewMode;
  openPdf: () => void;
  openCommandPalette: () => void;
  dismissModals: () => void;
  exitNoteMode: () => void;
  exitDrawMode: () => void;
  exitShapeMode: () => void;
  exitStampMode: () => void;
  exitRedactMode: () => void;
  exitImageInsertMode: () => void;
  exitTextEditMode: () => void;
  exitVectorEditMode: () => void;
  exitFormAddMode: () => void;
  exitHighlightMode: () => void;
  goToPage: (page: number) => void;
  toggleHighlightMode: () => void;
  toggleNoteMode: () => void;
  toggleDrawMode: () => void;
  toggleShapeMode: () => void;
  toggleStampMode: () => void;
  toggleRedactMode: () => void;
  toggleTextEditMode: () => void;
  toggleVectorEditMode: () => void;
  toggleImageInsertMode: () => void;
  toggleFormsPanel: () => void;
  openDeleteModal: () => void;
  openSaveAs: () => void;
  handleSave: () => void | Promise<void>;
  requestClosePdf: () => void;
  quitApp: () => void;
  handlePrint: () => void | Promise<void>;
  openPrintDialog: () => void;
  handleRotatePage: () => void | Promise<void>;
  openSearchModal: () => void;
  handleDuplicatePage: () => void | Promise<void>;
  toggleMarkdownView: () => void | Promise<void>;
  handleOptimizePdf: () => void | Promise<void>;
  handleSummarizePdf: () => void | Promise<void>;
  openSignModal: () => void;
  openInsertModal: () => void;
  openSplitModal: () => void;
  openExtractModal: () => void;
  openExportPngModal: () => void;
  handleAddBlankPage: () => void | Promise<void>;
  handleReversePages: () => void | Promise<void>;
  openMergeModal: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
  undo: () => void | Promise<void>;
  redo: () => void | Promise<void>;
  cycleTab: (delta: number) => void;
  jumpToTab: (index: number) => void;
};

export type BuildAppKeyboardActionsInput = {
  isDirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  filePath: string;
  noteMode: boolean;
  drawMode: boolean;
  shapeMode: boolean;
  stampMode: boolean;
  redactMode: boolean;
  imageInsertMode: boolean;
  textEditMode: boolean;
  vectorEditMode: boolean;
  formAddMode: boolean;
  highlightMode: boolean;
  anyModalOpen: boolean;
  pageCount: number | null;
  currentPage: number;
  viewMode: ViewMode;
  openPdf: () => void;
  setShowCommandPalette: (open: boolean) => void;
  dismissModals: () => void;
  exitNoteMode: () => void;
  exitDrawMode: () => void;
  exitShapeMode: () => void;
  exitStampMode: () => void;
  exitRedactMode: () => void;
  exitImageInsertMode: () => void;
  exitTextEditMode: () => void;
  exitVectorEditMode: () => void;
  exitFormAddMode: () => void;
  exitHighlightMode: () => void;
  goToPage: (page: number) => void;
  toggleHighlightMode: () => void;
  toggleNoteMode: () => void;
  toggleDrawMode: () => void;
  toggleShapeMode: () => void;
  toggleStampMode: () => void;
  toggleRedactMode: () => void;
  toggleTextEditMode: () => void;
  toggleVectorEditMode: () => void;
  toggleImageInsertMode: () => void;
  toggleFormsPanel: () => void;
  openDeleteModal: () => void;
  openSaveAs: () => void;
  handleSave: () => void | Promise<void>;
  guardUnsaved: (action: () => void) => void;
  closePdf: () => void;
  exitApp: () => void;
  handlePrint: () => void;
  openPrintDialog: () => void;
  handleRotatePage: () => void | Promise<void>;
  openSearchModal: () => void;
  handleDuplicatePage: () => void | Promise<void>;
  toggleMarkdownView: () => void | Promise<void>;
  handleOptimizePdf: () => void | Promise<void>;
  handleSummarizePdf: () => void | Promise<void>;
  openSignModal: () => void;
  openInsertModal: () => void;
  openSplitModal: () => void;
  openExtractModal: () => void;
  openExportPngModal: () => void;
  handleAddBlankPage: () => void | Promise<void>;
  handleReversePages: () => void | Promise<void>;
  openMergeModal: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
  undo: () => void;
  redo: () => void;
  cycleTab: (delta: number) => void;
  jumpToTab: (index: number) => void;
};

export function buildAppKeyboardActions(input: BuildAppKeyboardActionsInput): AppKeyboardActions {
  return {
    isDirty: input.isDirty,
    canUndo: input.canUndo,
    canRedo: input.canRedo,
    hasOpenPdf: !!input.filePath,
    noteMode: input.noteMode,
    drawMode: input.drawMode,
    shapeMode: input.shapeMode,
    stampMode: input.stampMode,
    redactMode: input.redactMode,
    imageInsertMode: input.imageInsertMode,
    textEditMode: input.textEditMode,
    vectorEditMode: input.vectorEditMode,
    formAddMode: input.formAddMode,
    highlightMode: input.highlightMode,
    anyModalOpen: input.anyModalOpen,
    pageCount: input.pageCount,
    currentPage: input.currentPage,
    viewMode: input.viewMode,
    openPdf: input.openPdf,
    openCommandPalette: () => input.setShowCommandPalette(true),
    dismissModals: input.dismissModals,
    exitNoteMode: input.exitNoteMode,
    exitDrawMode: input.exitDrawMode,
    exitShapeMode: input.exitShapeMode,
    exitStampMode: input.exitStampMode,
    exitRedactMode: input.exitRedactMode,
    exitImageInsertMode: input.exitImageInsertMode,
    exitTextEditMode: input.exitTextEditMode,
    exitVectorEditMode: input.exitVectorEditMode,
    exitFormAddMode: input.exitFormAddMode,
    exitHighlightMode: input.exitHighlightMode,
    goToPage: input.goToPage,
    toggleHighlightMode: input.toggleHighlightMode,
    toggleNoteMode: input.toggleNoteMode,
    toggleDrawMode: input.toggleDrawMode,
    toggleShapeMode: input.toggleShapeMode,
    toggleStampMode: input.toggleStampMode,
    toggleRedactMode: input.toggleRedactMode,
    toggleTextEditMode: input.toggleTextEditMode,
    toggleVectorEditMode: input.toggleVectorEditMode,
    toggleImageInsertMode: input.toggleImageInsertMode,
    toggleFormsPanel: input.toggleFormsPanel,
    openDeleteModal: input.openDeleteModal,
    openSaveAs: input.openSaveAs,
    handleSave: input.handleSave,
    requestClosePdf: () => input.guardUnsaved(input.closePdf),
    quitApp: () => input.guardUnsaved(input.exitApp),
    handlePrint: input.handlePrint,
    openPrintDialog: input.openPrintDialog,
    handleRotatePage: input.handleRotatePage,
    openSearchModal: input.openSearchModal,
    handleDuplicatePage: input.handleDuplicatePage,
    toggleMarkdownView: input.toggleMarkdownView,
    handleOptimizePdf: input.handleOptimizePdf,
    handleSummarizePdf: input.handleSummarizePdf,
    openSignModal: input.openSignModal,
    openInsertModal: input.openInsertModal,
    openSplitModal: input.openSplitModal,
    openExtractModal: input.openExtractModal,
    openExportPngModal: input.openExportPngModal,
    handleAddBlankPage: input.handleAddBlankPage,
    handleReversePages: input.handleReversePages,
    openMergeModal: input.openMergeModal,
    zoomIn: input.zoomIn,
    zoomOut: input.zoomOut,
    resetZoom: input.resetZoom,
    undo: input.undo,
    redo: input.redo,
    cycleTab: input.cycleTab,
    jumpToTab: input.jumpToTab,
  };
}

import type { AppKeyboardActions } from './buildAppKeyboardActions';
import type { ShortcutCommandId } from '../settings/shortcutRegistry';

export type ShortcutHandler = {
  id: ShortcutCommandId;
  enabled: (a: AppKeyboardActions) => boolean;
  run: (a: AppKeyboardActions) => void | Promise<void>;
  preventDefault?: boolean;
};

export const SHORTCUT_HANDLERS: Record<ShortcutCommandId, ShortcutHandler> = {
  'open-pdf': {
    id: 'open-pdf',
    enabled: () => true,
    run: (a) => a.openPdf(),
  },
  'command-palette': {
    id: 'command-palette',
    enabled: () => true,
    run: (a) => a.openCommandPalette(),
  },
  save: {
    id: 'save',
    enabled: (a) => a.hasOpenPdf && a.isDirty,
    run: (a) => void a.handleSave(),
  },
  'save-as': {
    id: 'save-as',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openSaveAs(),
  },
  'close-pdf': {
    id: 'close-pdf',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.requestClosePdf(),
  },
  quit: {
    id: 'quit',
    enabled: () => true,
    run: (a) => a.quitApp(),
  },
  print: {
    id: 'print',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.openPrintDialog(),
  },
  undo: {
    id: 'undo',
    enabled: (a) => a.hasOpenPdf && a.canUndo,
    run: (a) => void a.undo(),
  },
  redo: {
    id: 'redo',
    enabled: (a) => a.hasOpenPdf && a.canRedo,
    run: (a) => void a.redo(),
  },
  find: {
    id: 'find',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openSearchModal(),
  },
  'rotate-page': {
    id: 'rotate-page',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.handleRotatePage(),
  },
  'duplicate-page': {
    id: 'duplicate-page',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.handleDuplicatePage(),
  },
  'blank-page-after': {
    id: 'blank-page-after',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.handleAddBlankPage(),
  },
  'reverse-pages': {
    id: 'reverse-pages',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.handleReversePages(),
  },
  'insert-pdf': {
    id: 'insert-pdf',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openInsertModal(),
  },
  'merge-pdf': {
    id: 'merge-pdf',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openMergeModal(),
  },
  'split-pdf': {
    id: 'split-pdf',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openSplitModal(),
  },
  'extract-pages': {
    id: 'extract-pages',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openExtractModal(),
  },
  'markdown-view': {
    id: 'markdown-view',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.toggleMarkdownView(),
  },
  'optimize-pdf': {
    id: 'optimize-pdf',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.handleOptimizePdf(),
  },
  'export-images': {
    id: 'export-images',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openExportPngModal(),
  },
  summarize: {
    id: 'summarize',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => void a.handleSummarizePdf(),
  },
  'sign-pdf': {
    id: 'sign-pdf',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.openSignModal(),
  },
  'delete-page': {
    id: 'delete-page',
    enabled: (a) => a.hasOpenPdf && a.pageCount !== null && a.pageCount > 1,
    run: (a) => a.openDeleteModal(),
  },
  'toggle-highlight': {
    id: 'toggle-highlight',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleHighlightMode(),
  },
  'toggle-note': {
    id: 'toggle-note',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleNoteMode(),
  },
  'toggle-draw': {
    id: 'toggle-draw',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleDrawMode(),
  },
  'toggle-shape': {
    id: 'toggle-shape',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleShapeMode(),
  },
  'toggle-stamp': {
    id: 'toggle-stamp',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleStampMode(),
  },
  'toggle-redact': {
    id: 'toggle-redact',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleRedactMode(),
  },
  'toggle-text-edit': {
    id: 'toggle-text-edit',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleTextEditMode(),
  },
  'toggle-vector-edit': {
    id: 'toggle-vector-edit',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleVectorEditMode(),
  },
  'toggle-image-insert': {
    id: 'toggle-image-insert',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleImageInsertMode(),
  },
  'toggle-forms': {
    id: 'toggle-forms',
    enabled: (a) => a.hasOpenPdf && a.viewMode === 'pdf',
    run: (a) => a.toggleFormsPanel(),
  },
  'previous-page': {
    id: 'previous-page',
    enabled: (a) => a.hasOpenPdf && a.currentPage > 0,
    run: (a) => a.goToPage(a.currentPage - 1),
  },
  'next-page': {
    id: 'next-page',
    enabled: (a) => a.hasOpenPdf && a.pageCount !== null && a.currentPage < a.pageCount - 1,
    run: (a) => a.goToPage(a.currentPage + 1),
  },
  'first-page': {
    id: 'first-page',
    enabled: (a) => a.hasOpenPdf && a.currentPage > 0,
    run: (a) => a.goToPage(0),
  },
  'last-page': {
    id: 'last-page',
    enabled: (a) => a.hasOpenPdf && a.pageCount !== null && a.currentPage < a.pageCount - 1,
    run: (a) => a.goToPage(a.pageCount! - 1),
  },
  'zoom-in': {
    id: 'zoom-in',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.zoomIn(),
  },
  'zoom-out': {
    id: 'zoom-out',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.zoomOut(),
  },
  'zoom-reset': {
    id: 'zoom-reset',
    enabled: (a) => a.hasOpenPdf,
    run: (a) => a.resetZoom(),
  },
  'cycle-tab-next': {
    id: 'cycle-tab-next',
    enabled: () => true,
    run: (a) => a.cycleTab(1),
  },
  'cycle-tab-prev': {
    id: 'cycle-tab-prev',
    enabled: () => true,
    run: (a) => a.cycleTab(-1),
  },
  'jump-tab-1': {
    id: 'jump-tab-1',
    enabled: () => true,
    run: (a) => a.jumpToTab(0),
  },
  'jump-tab-2': {
    id: 'jump-tab-2',
    enabled: () => true,
    run: (a) => a.jumpToTab(1),
  },
  'jump-tab-3': {
    id: 'jump-tab-3',
    enabled: () => true,
    run: (a) => a.jumpToTab(2),
  },
  'jump-tab-4': {
    id: 'jump-tab-4',
    enabled: () => true,
    run: (a) => a.jumpToTab(3),
  },
  'jump-tab-5': {
    id: 'jump-tab-5',
    enabled: () => true,
    run: (a) => a.jumpToTab(4),
  },
  'jump-tab-6': {
    id: 'jump-tab-6',
    enabled: () => true,
    run: (a) => a.jumpToTab(5),
  },
  'jump-tab-7': {
    id: 'jump-tab-7',
    enabled: () => true,
    run: (a) => a.jumpToTab(6),
  },
  'jump-tab-8': {
    id: 'jump-tab-8',
    enabled: () => true,
    run: (a) => a.jumpToTab(7),
  },
  'jump-tab-9': {
    id: 'jump-tab-9',
    enabled: () => true,
    run: (a) => a.jumpToTab(8),
  },
};

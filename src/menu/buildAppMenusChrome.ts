import type { AppMenuContext, MenuAction, MenuRoot } from './types';
import { act, sep } from './menuBuilders';

export function buildViewMenu(ctx: AppMenuContext): MenuRoot {
  const pdfItems = [
    act('view-birdseye', "Bird's Eye View", ctx.setWorkspaceViewBirdseye, {
      active: ctx.workspaceView === 'birdseye',
      disabled: !ctx.hasPdf,
    }),
    act('view-pdf', 'PDF view', ctx.setViewModePdf, {
      active: ctx.workspaceView === 'tabs' && ctx.viewMode === 'pdf',
    }),
    act('view-md', 'Markdown view', () => {
      ctx.setWorkspaceViewTabs();
      void ctx.toggleMarkdownView();
    }, {
      shortcutCommandId: 'markdown-view',
      active: ctx.workspaceView === 'tabs' && ctx.viewMode === 'markdown',
      disabled: ctx.viewMode === 'webpage' && !ctx.hasPdf,
    }),
    sep(),
    act(
      'continuous-scroll',
      ctx.scrollViewMode === 'continuous' ? 'Continuous scroll (on)' : 'Continuous scroll',
      ctx.toggleContinuousScroll,
      { active: ctx.scrollViewMode === 'continuous', disabled: ctx.workspaceView !== 'tabs' || ctx.viewMode !== 'pdf' },
    ),
    act('thumbnails', ctx.showSidebar ? 'Thumbnails (on)' : 'Thumbnails', ctx.toggleSidebar, {
      active: ctx.showSidebar,
    }),
    act('bookmarks', ctx.showBookmarksPanel ? 'Bookmarks panel (on)' : 'Bookmarks panel', ctx.toggleBookmarksPanel, {
      active: ctx.showBookmarksPanel,
    }),
    act(
      'annotations-panel',
      ctx.showAnnotationsPanel ? 'Annotations panel (on)' : 'Annotations panel',
      ctx.toggleAnnotationsPanel,
      { active: ctx.showAnnotationsPanel },
    ),
    act('forms', ctx.showFormsPanel ? 'Forms panel (on)' : 'Forms panel', ctx.toggleFormsPanel, {
      shortcutCommandId: 'toggle-forms',
      active: ctx.showFormsPanel,
    }),
    act(
      'show-hidden-layers',
      ctx.showHiddenLayers ? 'Hidden watermarks/layers (on)' : 'Hidden watermarks/layers',
      ctx.toggleShowHiddenLayers,
      { active: ctx.showHiddenLayers },
    ),
    act(
      'pdfua-panel',
      ctx.showPdfUaPanel ? 'PDF/UA Check (on)' : 'PDF/UA Check',
      ctx.togglePdfUaPanel,
      { active: ctx.showPdfUaPanel },
    ),
  ];
  return {
    id: 'view',
    label: 'View',
    items: pdfItems,
  };
}

export function buildHelpMenu(ctx: AppMenuContext): MenuRoot {
  return {
    id: 'help',
    label: 'Help',
    items: [
      act('cmd-palette', 'Command palette…', ctx.openCommandPalette, { shortcutCommandId: 'command-palette' }),
      ...(ctx.tesseractInstalled
        ? []
        : [act('tesseract', 'Install Tesseract (scan OCR)…', ctx.openTesseractGuide)]),
      act('settings', 'Settings…', () => ctx.openSettings(null)),
      act('shortcuts', 'Keyboard shortcuts…', () => ctx.openSettings('shortcuts')),
      act('licenses', 'Licenses…', ctx.openLicenses),
      act('credits', 'Credits…', ctx.openCredits),
      act('about', 'About Kanoprii…', ctx.openAbout),
      act('check-updates', 'Check for Updates…', ctx.openUpdateModal),
    ],
  };
}

export function buildQuickAccessActions(ctx: AppMenuContext): MenuAction[] {
  const off = !ctx.hasPdf;
  const noToolActive =
    !ctx.highlightMode && !ctx.noteMode && !ctx.drawMode && !ctx.shapeMode &&
    !ctx.stampMode && !ctx.redactMode && !ctx.imageInsertMode &&
    !ctx.textEditMode && !ctx.editTextRunMode && !ctx.vectorEditMode &&
    !ctx.formAddMode && !ctx.editMode;
  return [
    act('qa-save', ctx.isDirty ? 'Save •' : 'Save', ctx.handleSave, {
      shortcutCommandId: 'save',
      disabled: off || !ctx.isDirty,
    }),
    act('qa-undo', 'Undo', ctx.undo, { shortcutCommandId: 'undo', disabled: off || !ctx.canUndo }),
    act('qa-redo', 'Redo', ctx.redo, { shortcutCommandId: 'redo', disabled: off || !ctx.canRedo }),
    act('qa-select', 'Select', () => {
      if (ctx.highlightMode) ctx.toggleHighlightMode();
      if (ctx.noteMode) ctx.toggleNoteMode();
      if (ctx.drawMode) ctx.toggleDrawMode();
      if (ctx.shapeMode) ctx.toggleShapeMode();
      if (ctx.stampMode) ctx.toggleStampMode();
      if (ctx.redactMode) ctx.toggleRedactMode();
      if (ctx.imageInsertMode) ctx.toggleImageInsertMode();
      if (ctx.textEditMode) ctx.toggleTextEditMode();
      if (ctx.editTextRunMode) ctx.toggleEditTextRunMode();
      if (ctx.vectorEditMode) ctx.toggleVectorEditMode();
      if (ctx.formAddMode) ctx.exitFormAddMode();
      if (ctx.editMode) ctx.toggleEditMode();
    }, { disabled: off, active: ctx.hasPdf && noToolActive }),
    act('qa-find', 'Find', ctx.openSearchModal, { shortcutCommandId: 'find', disabled: off }),
    act('qa-rotate', 'Rotate', ctx.openRotateModal, { shortcutCommandId: 'rotate-page', disabled: off }),
    act('qa-dup', 'Duplicate', ctx.handleDuplicatePage, { shortcutCommandId: 'duplicate-page', disabled: off }),
  ];
}

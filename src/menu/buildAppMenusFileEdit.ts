import type { AppMenuContext, MenuRoot } from './types';
import { act, sep, sub } from './menuBuilders';

export function buildFileEditMenus(ctx: AppMenuContext): { fileMenu: MenuRoot; editMenu: MenuRoot } {
  const fileMenu: MenuRoot = {
    id: 'file',
    label: 'File',
    items: [
      act('open', 'Open Document…', ctx.openPdf, { shortcutCommandId: 'open-pdf' }),
      ...(ctx.hasPdf
        ? [
            sep(),
            act('save', ctx.isDirty ? 'Save •' : 'Save', ctx.handleSave, {
              shortcutCommandId: 'save',
              disabled: !ctx.isDirty,
            }),
            act('save-as', 'Save As…', ctx.openSaveAs, { shortcutCommandId: 'save-as' }),
            sep(),
            act('print', 'Print…', ctx.openPrintDialog, { shortcutCommandId: 'print' }),
            sub('Export', [
              act('export-image', 'Pages as images…', ctx.openExportPngModal, { shortcutCommandId: 'export-images' }),
              act('export-page', 'Current page as PDF…', ctx.openExportPagePdfModal),
              act('export-pages', 'Each page as PDF…', ctx.openExportPagesPdfModal),
            ]),
            sep(),
            act('protect', 'Export password-protected copy…', ctx.openProtectModal),
            act('decrypt', 'Save decrypted copy…', ctx.openDecryptModal),
            sep(),
            act('close', 'Close', ctx.requestClosePdf, { shortcutCommandId: 'close-pdf' }),
          ]
        : []),
      ...(!ctx.hasPdf && ctx.hasDocument
        ? [sep(), act('close', 'Close', ctx.requestClosePdf, { shortcutCommandId: 'close-pdf' })]
        : []),
      sep(),
      act('quit', 'Quit', ctx.quitApp, { shortcutCommandId: 'quit' }),
    ],
  };

  const editMenu: MenuRoot = {
    id: 'edit',
    label: 'Edit',
    disabled: !ctx.hasPdf,
    items: [
      act('undo', 'Undo', ctx.undo, { shortcutCommandId: 'undo', disabled: !ctx.canUndo }),
      act('redo', 'Redo', ctx.redo, { shortcutCommandId: 'redo', disabled: !ctx.canRedo }),
      sep(),
      act('pdf-edit', ctx.editMode ? 'Edit PDF content (on)' : 'Edit PDF content', ctx.toggleEditMode, {
        shortcutCommandId: 'toggle-pdf-edit',
        active: ctx.editMode,
      }),
      act('edit-text', ctx.editTextRunMode ? 'Edit text (on)' : 'Edit text', ctx.toggleEditTextRunMode, {
        active: ctx.editTextRunMode,
      }),
      act('page-text', ctx.textEditMode ? 'Add page text (on)' : 'Add page text', ctx.toggleTextEditMode, {
        shortcutCommandId: 'toggle-text-edit',
        active: ctx.textEditMode,
      }),
      act('insert-image', ctx.imageInsertMode ? 'Insert image (on)' : 'Insert image', ctx.toggleImageInsertMode, {
        shortcutCommandId: 'toggle-image-insert',
        active: ctx.imageInsertMode,
      }),
      act('vector', ctx.vectorEditMode ? 'Edit vector (on)' : 'Edit vector', ctx.toggleVectorEditMode, {
        shortcutCommandId: 'toggle-vector-edit',
        active: ctx.vectorEditMode,
      }),
      act('edits', 'Manage page edits…', ctx.openPageEditsModal),
      sep(),
      act('find', 'Find text…', ctx.openSearchModal, { shortcutCommandId: 'find' }),
    ],
  };

  return { fileMenu, editMenu };
}

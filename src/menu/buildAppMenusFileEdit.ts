import type { AppMenuContext, MenuRoot } from './types';
import { act, sep, sub } from './menuBuilders';

export function buildFileEditMenus(ctx: AppMenuContext): { fileMenu: MenuRoot; editMenu: MenuRoot } {
  const fileMenu: MenuRoot = {
    id: 'file',
    label: 'File',
    items: [
      act('open', 'Open PDF…', ctx.openPdf, { shortcutCommandId: 'open-pdf' }),
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
      act('find', 'Find text…', ctx.openSearchModal, { shortcutCommandId: 'find' }),
    ],
  };

  return { fileMenu, editMenu };
}

import type { AppMenuContext, MenuRoot } from './types';
import { act } from './menuBuilders';

export function buildAnnotMenu(ctx: AppMenuContext): MenuRoot {
  return {
    id: 'annotate',
    label: 'Annotate',
    disabled: !ctx.hasPdf,
    items: [
      act('highlight', ctx.highlightMode ? 'Highlight (on)' : 'Highlight', ctx.toggleHighlightMode, {
        shortcutCommandId: 'toggle-highlight',
        active: ctx.highlightMode,
      }),
      act('highlight-selection', 'Highlight Selection', ctx.highlightSelection, {
        disabled: !ctx.hasTextSelection,
      }),
      act('note', ctx.noteMode ? 'Sticky note (on)' : 'Sticky note', ctx.toggleNoteMode, {
        shortcutCommandId: 'toggle-note',
        active: ctx.noteMode,
      }),
      act('draw', ctx.drawMode ? 'Draw (on)' : 'Draw', ctx.toggleDrawMode, { shortcutCommandId: 'toggle-draw', active: ctx.drawMode }),
      act('shape', ctx.shapeMode ? 'Shape (on)' : 'Shape', ctx.toggleShapeMode, {
        shortcutCommandId: 'toggle-shape',
        active: ctx.shapeMode,
      }),
      act('stamp', ctx.stampMode ? 'Stamp (on)' : 'Stamp', ctx.toggleStampMode, {
        shortcutCommandId: 'toggle-stamp',
        active: ctx.stampMode,
      }),
      act('redact', ctx.redactMode ? 'Redact (on)' : 'Redact', ctx.toggleRedactMode, {
        shortcutCommandId: 'toggle-redact',
        active: ctx.redactMode,
      }),
    ],
  };
}

export function buildSecurityMenu(ctx: AppMenuContext): MenuRoot {
  return {
    id: 'security',
    label: 'Security',
    disabled: !ctx.hasPdf,
    items: [
      act('sign', 'Digitally sign…', ctx.openSignModal, { shortcutCommandId: 'sign-pdf' }),
      act('signatures', ctx.showSignaturesPanel ? 'Signatures panel (on)' : 'Signatures panel', ctx.toggleSignaturesPanel, {
        active: ctx.showSignaturesPanel,
      }),
    ],
  };
}

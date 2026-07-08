import type { ShortcutCommandId } from '../settings/shortcutRegistry';
import { shortcutToDisplay } from '../settings/shortcutKeys';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';

export type KeyboardShortcutRow = { keys: string; action: string };

const GROUPED_ROWS: { label: string; commandIds: ShortcutCommandId[] }[] = [
  { label: 'Open PDF', commandIds: ['open-pdf'] },
  { label: 'Command palette', commandIds: ['command-palette'] },
  { label: 'Save / Save As', commandIds: ['save', 'save-as'] },
  { label: 'Close PDF', commandIds: ['close-pdf'] },
  { label: 'Print', commandIds: ['print'] },
  { label: 'Undo / Redo', commandIds: ['undo', 'redo'] },
  { label: 'Find text', commandIds: ['find'] },
  { label: 'Rotate page', commandIds: ['rotate-page'] },
  { label: 'Duplicate page', commandIds: ['duplicate-page'] },
  { label: 'Blank page after', commandIds: ['blank-page-after'] },
  { label: 'Reverse pages', commandIds: ['reverse-pages'] },
  { label: 'Insert PDF', commandIds: ['insert-pdf'] },
  { label: 'Merge PDF', commandIds: ['merge-pdf'] },
  { label: 'Split PDF', commandIds: ['split-pdf'] },
  { label: 'Extract pages', commandIds: ['extract-pages'] },
  { label: 'Markdown view', commandIds: ['markdown-view'] },
  { label: 'Optimize PDF', commandIds: ['optimize-pdf'] },
  { label: 'Export images', commandIds: ['export-images'] },
  { label: 'Summarize', commandIds: ['summarize'] },
  { label: 'Sign PDF', commandIds: ['sign-pdf'] },
  { label: 'Delete page', commandIds: ['delete-page'] },
  { label: 'Highlight / Note / Draw / Shape / Stamp / Redact', commandIds: ['toggle-highlight', 'toggle-note', 'toggle-draw', 'toggle-shape', 'toggle-stamp', 'toggle-redact'] },
  { label: 'Page text / Vector / Insert image / Forms', commandIds: ['toggle-text-edit', 'toggle-vector-edit', 'toggle-image-insert', 'toggle-forms'] },
  { label: 'Previous / next page', commandIds: ['previous-page', 'next-page'] },
  { label: 'First / last page', commandIds: ['first-page', 'last-page'] },
  { label: 'Zoom in / out / reset', commandIds: ['zoom-in', 'zoom-out', 'zoom-reset'] },
  { label: 'Next / previous tab', commandIds: ['cycle-tab-next', 'cycle-tab-prev'] },
  { label: 'Jump to tab 1-9', commandIds: ['jump-tab-1', 'jump-tab-2', 'jump-tab-3', 'jump-tab-4', 'jump-tab-5', 'jump-tab-6', 'jump-tab-7', 'jump-tab-8', 'jump-tab-9'] },
];

export function buildKeyboardShortcuts(bindings: ShortcutBindings): KeyboardShortcutRow[] {
  return GROUPED_ROWS.map(({ label, commandIds }) => {
    const keys = commandIds
      .flatMap((id) => bindings[id] ?? [])
      .map((s) => shortcutToDisplay(s))
      .join(' / ');
    return { keys: keys || '-', action: label };
  });
}

export function getShortcutsForCommand(bindings: ShortcutBindings, commandId: ShortcutCommandId): string[] {
  return bindings[commandId] ?? [];
}

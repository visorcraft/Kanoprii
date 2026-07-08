export type ShortcutCommandId =
  | 'open-pdf'
  | 'quit'
  | 'save'
  | 'save-as'
  | 'close-pdf'
  | 'print'
  | 'undo'
  | 'redo'
  | 'find'
  | 'command-palette'
  | 'rotate-page'
  | 'duplicate-page'
  | 'blank-page-after'
  | 'reverse-pages'
  | 'insert-pdf'
  | 'merge-pdf'
  | 'split-pdf'
  | 'extract-pages'
  | 'markdown-view'
  | 'optimize-pdf'
  | 'export-images'
  | 'summarize'
  | 'sign-pdf'
  | 'delete-page'
  | 'toggle-highlight'
  | 'toggle-note'
  | 'toggle-draw'
  | 'toggle-shape'
  | 'toggle-stamp'
  | 'toggle-redact'
  | 'toggle-text-edit'
  | 'toggle-vector-edit'
  | 'toggle-image-insert'
  | 'toggle-forms'
  | 'previous-page'
  | 'next-page'
  | 'first-page'
  | 'last-page'
  | 'zoom-in'
  | 'zoom-out'
  | 'zoom-reset'
  | 'cycle-tab-next'
  | 'cycle-tab-prev'
  | 'jump-tab-1'
  | 'jump-tab-2'
  | 'jump-tab-3'
  | 'jump-tab-4'
  | 'jump-tab-5'
  | 'jump-tab-6'
  | 'jump-tab-7'
  | 'jump-tab-8'
  | 'jump-tab-9';

export type ShortcutCategory =
  | 'Global'
  | 'File'
  | 'Edit'
  | 'Pages'
  | 'View'
  | 'Document'
  | 'Annotation'
  | 'Navigation'
  | 'Tabs';

export type ShortcutBinding = {
  commandId: ShortcutCommandId;
  category: ShortcutCategory;
  label: string;
  defaultShortcuts: string[];
};

export const SHORTCUT_REGISTRY: ShortcutBinding[] = [
  { commandId: 'open-pdf', category: 'Global', label: 'Open PDF', defaultShortcuts: ['Ctrl+O'] },
  { commandId: 'command-palette', category: 'Global', label: 'Command palette', defaultShortcuts: ['Ctrl+Shift+P'] },
  { commandId: 'quit', category: 'Global', label: 'Quit', defaultShortcuts: ['Ctrl+Q'] },

  { commandId: 'save', category: 'File', label: 'Save', defaultShortcuts: ['Ctrl+S'] },
  { commandId: 'save-as', category: 'File', label: 'Save As', defaultShortcuts: ['Ctrl+Shift+S'] },
  { commandId: 'close-pdf', category: 'File', label: 'Close PDF', defaultShortcuts: ['Ctrl+W'] },
  { commandId: 'print', category: 'File', label: 'Print', defaultShortcuts: ['Ctrl+P'] },

  { commandId: 'undo', category: 'Edit', label: 'Undo', defaultShortcuts: ['Ctrl+Z'] },
  { commandId: 'redo', category: 'Edit', label: 'Redo', defaultShortcuts: ['Ctrl+Y', 'Ctrl+Shift+Z'] },
  { commandId: 'find', category: 'Edit', label: 'Find text', defaultShortcuts: ['Ctrl+F'] },

  { commandId: 'rotate-page', category: 'Pages', label: 'Rotate page', defaultShortcuts: ['Ctrl+R'] },
  { commandId: 'duplicate-page', category: 'Pages', label: 'Duplicate page', defaultShortcuts: ['Ctrl+Shift+D'] },
  { commandId: 'blank-page-after', category: 'Pages', label: 'Blank page after', defaultShortcuts: ['Ctrl+Shift+N'] },
  { commandId: 'reverse-pages', category: 'Pages', label: 'Reverse pages', defaultShortcuts: ['Ctrl+Shift+Y'] },
  { commandId: 'insert-pdf', category: 'Pages', label: 'Insert PDF', defaultShortcuts: ['Ctrl+Shift+I'] },
  { commandId: 'merge-pdf', category: 'Pages', label: 'Merge PDF', defaultShortcuts: ['Ctrl+Shift+G'] },
  { commandId: 'split-pdf', category: 'Pages', label: 'Split PDF', defaultShortcuts: ['Ctrl+Shift+K'] },
  { commandId: 'extract-pages', category: 'Pages', label: 'Extract pages', defaultShortcuts: ['Ctrl+Shift+J'] },
  { commandId: 'delete-page', category: 'Pages', label: 'Delete page', defaultShortcuts: ['Delete'] },

  { commandId: 'markdown-view', category: 'View', label: 'Markdown view', defaultShortcuts: ['Ctrl+Shift+M'] },

  { commandId: 'optimize-pdf', category: 'Document', label: 'Optimize PDF', defaultShortcuts: ['Ctrl+Shift+O'] },
  { commandId: 'export-images', category: 'Document', label: 'Export images', defaultShortcuts: ['Ctrl+Shift+B'] },
  { commandId: 'summarize', category: 'Document', label: 'Summarize', defaultShortcuts: ['Ctrl+Shift+E'] },
  { commandId: 'sign-pdf', category: 'Document', label: 'Sign PDF', defaultShortcuts: ['Ctrl+Shift+U'] },

  { commandId: 'toggle-highlight', category: 'Annotation', label: 'Highlight', defaultShortcuts: ['H'] },
  { commandId: 'toggle-note', category: 'Annotation', label: 'Note', defaultShortcuts: ['N'] },
  { commandId: 'toggle-draw', category: 'Annotation', label: 'Draw', defaultShortcuts: ['D'] },
  { commandId: 'toggle-shape', category: 'Annotation', label: 'Shape', defaultShortcuts: ['S'] },
  { commandId: 'toggle-stamp', category: 'Annotation', label: 'Stamp', defaultShortcuts: ['T'] },
  { commandId: 'toggle-redact', category: 'Annotation', label: 'Redact', defaultShortcuts: ['X'] },
  { commandId: 'toggle-text-edit', category: 'Annotation', label: 'Page text', defaultShortcuts: ['E'] },
  { commandId: 'toggle-vector-edit', category: 'Annotation', label: 'Vector edit', defaultShortcuts: ['G'] },
  { commandId: 'toggle-image-insert', category: 'Annotation', label: 'Insert image', defaultShortcuts: ['I'] },
  { commandId: 'toggle-forms', category: 'Annotation', label: 'Forms', defaultShortcuts: ['F'] },

  { commandId: 'previous-page', category: 'Navigation', label: 'Previous page', defaultShortcuts: ['ArrowLeft', 'PageUp'] },
  { commandId: 'next-page', category: 'Navigation', label: 'Next page', defaultShortcuts: ['ArrowRight', 'PageDown'] },
  { commandId: 'first-page', category: 'Navigation', label: 'First page', defaultShortcuts: ['Home'] },
  { commandId: 'last-page', category: 'Navigation', label: 'Last page', defaultShortcuts: ['End'] },
  { commandId: 'zoom-in', category: 'View', label: 'Zoom in', defaultShortcuts: ['Ctrl+=', 'Ctrl+Plus', 'Ctrl+Shift+Plus'] },
  { commandId: 'zoom-out', category: 'View', label: 'Zoom out', defaultShortcuts: ['Ctrl+-'] },
  { commandId: 'zoom-reset', category: 'View', label: 'Reset zoom', defaultShortcuts: ['Ctrl+0'] },

  { commandId: 'cycle-tab-next', category: 'Tabs', label: 'Next tab', defaultShortcuts: ['Ctrl+Tab'] },
  { commandId: 'cycle-tab-prev', category: 'Tabs', label: 'Previous tab', defaultShortcuts: ['Ctrl+Shift+Tab'] },
  { commandId: 'jump-tab-1', category: 'Tabs', label: 'Jump to tab 1', defaultShortcuts: ['Ctrl+1'] },
  { commandId: 'jump-tab-2', category: 'Tabs', label: 'Jump to tab 2', defaultShortcuts: ['Ctrl+2'] },
  { commandId: 'jump-tab-3', category: 'Tabs', label: 'Jump to tab 3', defaultShortcuts: ['Ctrl+3'] },
  { commandId: 'jump-tab-4', category: 'Tabs', label: 'Jump to tab 4', defaultShortcuts: ['Ctrl+4'] },
  { commandId: 'jump-tab-5', category: 'Tabs', label: 'Jump to tab 5', defaultShortcuts: ['Ctrl+5'] },
  { commandId: 'jump-tab-6', category: 'Tabs', label: 'Jump to tab 6', defaultShortcuts: ['Ctrl+6'] },
  { commandId: 'jump-tab-7', category: 'Tabs', label: 'Jump to tab 7', defaultShortcuts: ['Ctrl+7'] },
  { commandId: 'jump-tab-8', category: 'Tabs', label: 'Jump to tab 8', defaultShortcuts: ['Ctrl+8'] },
  { commandId: 'jump-tab-9', category: 'Tabs', label: 'Jump to tab 9', defaultShortcuts: ['Ctrl+9'] },
];

export const SHORTCUT_COMMAND_MAP: Record<ShortcutCommandId, ShortcutBinding> = Object.fromEntries(
  SHORTCUT_REGISTRY.map((binding) => [binding.commandId, binding]),
) as Record<ShortcutCommandId, ShortcutBinding>;

export function getDefaultShortcuts(): Record<ShortcutCommandId, string[]> {
  return Object.fromEntries(
    SHORTCUT_REGISTRY.map((binding) => [binding.commandId, [...binding.defaultShortcuts]]),
  ) as Record<ShortcutCommandId, string[]>;
}

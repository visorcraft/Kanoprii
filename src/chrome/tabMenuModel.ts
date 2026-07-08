// Pure model for the tab right-click context menu: the item tree and the
// action interfaces. Kept free of React so the structure stays easy to reason
// about (and unit-testable) independent of rendering.

export type TabMenuItem =
  | { kind: 'divider' }
  | { kind: 'item'; id: string; label: string; disabled?: boolean; onSelect: () => void }
  | { kind: 'submenu'; id: string; label: string; items: TabMenuItem[] };

/** Zero-arg callbacks + enablement flags for a single (already-resolved) tab. */
export interface TabMenuActions {
  hasFile: boolean;
  canCloseOthers: boolean;
  canCloseRight: boolean;
  rename: () => void;
  closeTab: () => void;
  closeOthers: () => void;
  closeRight: () => void;
  newWindow: () => void;
  moveFirst: () => void;
  moveLast: () => void;
  copyPath: () => void;
  openFolder: () => void;
  print: () => void;
  properties: () => void;
}

/** Build the exact menu structure from the spec (order + dividers + submenu). */
export function buildTabMenuItems(a: TabMenuActions): TabMenuItem[] {
  return [
    { kind: 'item', id: 'rename', label: 'Rename file', disabled: !a.hasFile, onSelect: a.rename },
    { kind: 'divider' },
    { kind: 'item', id: 'close', label: 'Close tab', onSelect: a.closeTab },
    { kind: 'item', id: 'close-others', label: 'Close other tabs', disabled: !a.canCloseOthers, onSelect: a.closeOthers },
    { kind: 'item', id: 'close-right', label: 'Close tabs to the right', disabled: !a.canCloseRight, onSelect: a.closeRight },
    { kind: 'divider' },
    {
      kind: 'submenu',
      id: 'move',
      label: 'Move tab to',
      items: [
        { kind: 'item', id: 'move-new-window', label: 'New window', disabled: !a.hasFile, onSelect: a.newWindow },
        { kind: 'item', id: 'move-first', label: 'First tab', onSelect: a.moveFirst },
        { kind: 'item', id: 'move-last', label: 'Last tab', onSelect: a.moveLast },
      ],
    },
    { kind: 'divider' },
    { kind: 'item', id: 'copy-path', label: 'Copy filepath', disabled: !a.hasFile, onSelect: a.copyPath },
    { kind: 'item', id: 'open-folder', label: 'Open containing folder', disabled: !a.hasFile, onSelect: a.openFolder },
    { kind: 'divider' },
    { kind: 'item', id: 'print', label: 'Print', disabled: !a.hasFile, onSelect: a.print },
    { kind: 'item', id: 'properties', label: 'Document properties', disabled: !a.hasFile, onSelect: a.properties },
  ];
}

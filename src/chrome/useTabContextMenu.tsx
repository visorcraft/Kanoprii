import { useCallback, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DocumentTabInfo } from '../app/documentSessionTypes';
import { buildTabMenuItems, type TabMenuActions } from './tabMenuModel';
import { TabContextMenu } from './TabContextMenu';
import { ConfirmCloseTabsModal } from '../modals/ConfirmCloseTabsModal';
import { RenameFileModal } from '../modals/RenameFileModal';

/** Functions the tab menu drives; `tabs` carries enablement + paths. */
export interface TabMenuWiring {
  tabs: DocumentTabInfo[];
  selectTab: (id: string) => void;
  requestCloseTab: (id: string) => void;
  finalizeClose: (id: string) => void | Promise<void>;
  moveTabToFirst: (id: string) => void;
  moveTabToLast: (id: string) => void;
  moveToNewWindow: (id: string) => void;
  updateSession: (id: string, patch: { originalPath: string }) => void;
  openPrint: () => void;
  openProperties: (filePath: string) => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
}

/** The wiring minus `tabs` - threaded down to the chrome as a single prop. */
export type TabMenuApi = Omit<TabMenuWiring, 'tabs'>;

/** Strip directory and a trailing `.pdf` so the rename field shows the base name. */
function baseName(path: string): string {
  const file = path.split(/[\\/]/).pop() ?? path;
  return file.replace(/\.pdf$/i, '');
}

type MenuState = { x: number; y: number; targetId: string };
type RenameState = { id: string; originalPath: string; currentName: string };

/**
 * Owns the tab context menu plus the two follow-up modals it can trigger
 * (batch-close confirm, rename). Returns the right-click handler for TabBar and
 * an `overlay` element to render once near the tab bar.
 */
export function useTabContextMenu(api: TabMenuWiring) {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [confirmClose, setConfirmClose] = useState<{ ids: string[] } | null>(null);
  const [rename, setRename] = useState<RenameState | null>(null);

  const onTabContextMenu = useCallback((id: string, x: number, y: number) => {
    setMenu({ x, y, targetId: id });
  }, []);

  const closeMenu = useCallback(() => {
    const targetId = menu?.targetId;
    const menuRoot = document.querySelector<HTMLElement>('.tab-context-menu');
    setMenu(null);
    if (!targetId) return;

    setTimeout(() => {
      // Don't steal focus if an action already moved it elsewhere (e.g. a modal).
      if (document.activeElement !== document.body) return;
      // If the menu root is somehow still mounted, leave focus alone.
      if (menuRoot && document.contains(menuRoot)) return;

      const tab = document.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(targetId)}"]`);
      tab?.focus();
    }, 0);
  }, [menu]);

  const closeSet = useCallback(
    (ids: string[]) => {
      const targets = api.tabs.filter((t) => ids.includes(t.id));
      const dirty = targets.filter((t) => t.dirty).map((t) => t.id);
      targets.filter((t) => !t.dirty).forEach((t) => void api.finalizeClose(t.id));
      if (dirty.length > 0) setConfirmClose({ ids: dirty });
    },
    [api],
  );

  const submitRename = useCallback(
    async (target: RenameState, newName: string) => {
      try {
        const newPath = await invoke<string>('rename_document', {
          original_path: target.originalPath,
          new_file_name: newName,
        });
        api.updateSession(target.id, { originalPath: newPath });
        api.showToast('File renamed');
        setRename(null);
      } catch (e) {
        api.showToast(String(e), 'error');
      }
    },
    [api],
  );

  const parts: ReactNode[] = [];

  if (menu) {
    const idx = api.tabs.findIndex((t) => t.id === menu.targetId);
    const target = api.tabs[idx];
    if (target) {
      const otherIds = api.tabs.filter((t) => t.id !== target.id).map((t) => t.id);
      const rightIds = api.tabs.slice(idx + 1).map((t) => t.id);
      const actions: TabMenuActions = {
        hasFile: !!target.originalPath,
        canCloseOthers: otherIds.length > 0,
        canCloseRight: rightIds.length > 0,
        rename: () => setRename({ id: target.id, originalPath: target.originalPath, currentName: baseName(target.originalPath) }),
        closeTab: () => api.requestCloseTab(target.id),
        closeOthers: () => closeSet(otherIds),
        closeRight: () => closeSet(rightIds),
        newWindow: () => api.moveToNewWindow(target.id),
        moveFirst: () => api.moveTabToFirst(target.id),
        moveLast: () => api.moveTabToLast(target.id),
        copyPath: () => {
          void navigator.clipboard.writeText(target.originalPath);
          api.showToast('Filepath copied');
        },
        openFolder: () => {
          void invoke('reveal_in_file_manager', { path: target.originalPath }).catch((e) => api.showToast(String(e), 'error'));
        },
        print: () => {
          api.selectTab(target.id);
          api.openPrint();
        },
        properties: () => {
          api.selectTab(target.id);
          api.openProperties(target.filePath);
        },
      };
      parts.push(<TabContextMenu key="menu" items={buildTabMenuItems(actions)} x={menu.x} y={menu.y} onClose={closeMenu} />);
    }
  }

  if (confirmClose) {
    parts.push(
      <ConfirmCloseTabsModal
        key="confirm"
        count={confirmClose.ids.length}
        onConfirm={() => {
          confirmClose.ids.forEach((id) => void api.finalizeClose(id));
          setConfirmClose(null);
        }}
        onCancel={() => setConfirmClose(null)}
      />,
    );
  }

  if (rename) {
    parts.push(
      <RenameFileModal
        key="rename"
        currentName={rename.currentName}
        onClose={() => setRename(null)}
        onSubmit={(newName) => void submitRename(rename, newName)}
      />,
    );
  }

  const overlay = parts.length > 0 ? <>{parts}</> : null;
  return { onTabContextMenu, overlay };
}

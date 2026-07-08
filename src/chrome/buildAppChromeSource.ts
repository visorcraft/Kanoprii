import type { BuildChromeContextInput } from './buildChromeContext';
import type { AppMenus } from '../menu/types';
import type { TabMenuApi } from './useTabContextMenu';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { WorkspaceViewMode } from '../app/types';

export type BuildAppChromeSourceInput = {
  menus: AppMenus;
  showCommandPalette: boolean;
  showShortcutsHelp: boolean;
  showLicenses: boolean;
  showCredits: boolean;
  showAbout: boolean;
  setShowCommandPalette: (open: boolean) => void;
  setShowShortcutsHelp: (open: boolean) => void;
  setShowLicenses: (open: boolean) => void;
  setShowCredits: (open: boolean) => void;
  setShowAbout: (open: boolean) => void;
  modeExtras: BuildChromeContextInput['modeExtras'];
  tabs: BuildChromeContextInput['tabs'];
  activeTabId: BuildChromeContextInput['activeTabId'];
  onSelectTab: BuildChromeContextInput['onSelectTab'];
  onCloseTab: BuildChromeContextInput['onCloseTab'];
  tabMenuApi: TabMenuApi;
  documentChromeVisible: boolean;
  workspaceView: WorkspaceViewMode;
  shortcutBindings: ShortcutBindings;
};

export function buildAppChromeSource(input: BuildAppChromeSourceInput): BuildChromeContextInput {
  return {
    menus: input.menus,
    showCommandPalette: input.showCommandPalette,
    showShortcutsHelp: input.showShortcutsHelp,
    showLicenses: input.showLicenses,
    showCredits: input.showCredits,
    showAbout: input.showAbout,
    onCloseCommandPalette: () => input.setShowCommandPalette(false),
    onCloseShortcutsHelp: () => input.setShowShortcutsHelp(false),
    onCloseLicenses: () => input.setShowLicenses(false),
    onCloseCredits: () => input.setShowCredits(false),
    onCloseAbout: () => input.setShowAbout(false),
    modeExtras: input.modeExtras,
    tabs: input.tabs,
    activeTabId: input.activeTabId,
    onSelectTab: input.onSelectTab,
    onCloseTab: input.onCloseTab,
    tabMenuApi: input.tabMenuApi,
    documentChromeVisible: input.documentChromeVisible,
    workspaceView: input.workspaceView,
    shortcutBindings: input.shortcutBindings,
  };
}

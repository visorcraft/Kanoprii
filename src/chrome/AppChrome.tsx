import type { ReactNode } from 'react';
import { MenuChrome } from '../menu/MenuChrome';
import type { AppMenus } from '../menu/types';
import type { DocumentTabInfo } from '../app/documentSessionTypes';
import type { TabMenuApi } from './useTabContextMenu';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { WorkspaceViewMode } from '../app/types';

type AppChromeProps = {
  menus: AppMenus;
  showCommandPalette: boolean;
  showShortcutsHelp: boolean;
  showLicenses: boolean;
  showCredits: boolean;
  showAbout: boolean;
  onCloseCommandPalette: () => void;
  onCloseShortcutsHelp: () => void;
  onCloseLicenses: () => void;
  onCloseCredits: () => void;
  onCloseAbout: () => void;
  modeExtras: ReactNode;
  tabs: DocumentTabInfo[];
  activeTabId: string | null;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  tabMenuApi: TabMenuApi;
  documentChromeVisible: boolean;
  workspaceView: WorkspaceViewMode;
  shortcutBindings: ShortcutBindings;
};

export function AppChrome({
  menus,
  showCommandPalette,
  showShortcutsHelp,
  showLicenses,
  showCredits,
  showAbout,
  onCloseCommandPalette,
  onCloseShortcutsHelp,
  onCloseLicenses,
  onCloseCredits,
  onCloseAbout,
  modeExtras,
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  tabMenuApi,
  documentChromeVisible,
  workspaceView,
  shortcutBindings,
}: AppChromeProps) {
  return (
    <div className="app-chrome">
      <MenuChrome
        menus={menus.menus}
        quickAccess={menus.quickAccess}
        allActions={menus.allActions}
        showCommandPalette={showCommandPalette}
        showShortcutsHelp={showShortcutsHelp}
        showLicenses={showLicenses}
        showCredits={showCredits}
        showAbout={showAbout}
        onCloseCommandPalette={onCloseCommandPalette}
        onCloseShortcutsHelp={onCloseShortcutsHelp}
        onCloseLicenses={onCloseLicenses}
        onCloseCredits={onCloseCredits}
        onCloseAbout={onCloseAbout}
        modeExtras={modeExtras}
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={onSelectTab}
        onCloseTab={onCloseTab}
        tabMenuApi={tabMenuApi}
        documentChromeVisible={documentChromeVisible}
        workspaceView={workspaceView}
        shortcutBindings={shortcutBindings}
      />
    </div>
  );
}

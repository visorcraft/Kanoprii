import { MenuChrome } from '../menu/MenuChrome';
import type { AppMenus } from '../menu/types';
import type { RibbonTabExtras } from '../viewer/buildRibbonTabExtras';
import type { DocumentTabInfo } from '../app/documentSessionTypes';
import type { TabMenuApi } from './useTabContextMenu';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { WorkspaceViewMode } from '../app/types';

type AppChromeProps = {
  menus: AppMenus;
  ribbonExtras: RibbonTabExtras;
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
  ribbonExtras,
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
        menus={menus}
        ribbonExtras={ribbonExtras}
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

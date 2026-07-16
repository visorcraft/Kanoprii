import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { AboutModal } from '../about/AboutModal';
import { CreditsModal } from '../credits/CreditsModal';
import { LicensesModal } from '../licenses/LicensesModal';
import { TabBar } from '../chrome/TabBar';
import { Ribbon } from '../chrome/Ribbon';
import { buildRibbonTabs } from '../chrome/ribbonModel';
import { useTabContextMenu, type TabMenuApi } from '../chrome/useTabContextMenu';
import type { DocumentTabInfo } from '../app/documentSessionTypes';
import type { AppMenus, FlatMenuAction } from './types';
import { runAction } from './MenuDropdownItem';
import { buildKeyboardShortcuts } from './buildMenuShortcuts';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { WorkspaceViewMode } from '../app/types';
import { useEscapeClose } from '../legal/useEscapeClose';
import { FocusTrap } from '../ui/FocusTrap';
import type { RibbonTabExtras } from '../viewer/buildRibbonTabExtras';

type MenuChromeProps = {
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

function CommandPalette({
  actions,
  onClose,
}: {
  actions: FlatMenuAction[];
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [highlight, setHighlight] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return actions.filter((a) => !a.disabled).slice(0, 40);
    return actions
      .filter(
        (a) => !a.disabled && `${a.path} ${a.label}`.toLowerCase().includes(q)
      )
      .slice(0, 50);
  }, [actions, query]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setHighlight(0);
  }, [query]);

  const pick = useCallback(
    (action: FlatMenuAction) => {
      runAction(action);
      onClose();
    },
    [onClose]
  );

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setHighlight((prev) =>
        Math.min(prev + 1, Math.max(0, filtered.length - 1))
      );
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      setHighlight((prev) => Math.max(prev - 1, 0));
      return;
    }
    if (event.key === 'Enter' && filtered[highlight]) {
      event.preventDefault();
      pick(filtered[highlight]);
    }
  };

  return (
    <div className="command-palette-backdrop" onClick={onClose}>
      <FocusTrap>
        <div
          className="command-palette"
          onClick={(e) => e.stopPropagation()}
          role="dialog"
          aria-label="Command palette"
        >
          <input
            ref={inputRef}
            className="command-palette-input"
            type="text"
            placeholder="Search commands…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
          />
          <ul className="command-palette-list">
            {filtered.length === 0 ? (
              <li className="command-palette-empty">No matching commands</li>
            ) : (
              filtered.map((action, index) => (
                <li key={action.id}>
                  <button
                    type="button"
                    className={`command-palette-item${index === highlight ? ' highlighted' : ''}`}
                    onMouseEnter={() => setHighlight(index)}
                    onClick={() => pick(action)}
                  >
                    <span className="command-palette-path">{action.path}</span>
                    {action.shortcut && (
                      <span className="command-palette-shortcut">
                        {action.shortcut}
                      </span>
                    )}
                  </button>
                </li>
              ))
            )}
          </ul>
        </div>
      </FocusTrap>
    </div>
  );
}

function ShortcutsModal({
  bindings,
  onClose,
}: {
  bindings: ShortcutBindings;
  onClose: () => void;
}) {
  const shortcuts = useMemo(() => buildKeyboardShortcuts(bindings), [bindings]);
  useEscapeClose(onClose, true);
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <FocusTrap>
        <div
          className="modal shortcuts-modal"
          onClick={(e) => e.stopPropagation()}
        >
          <h3>Keyboard shortcuts</h3>
          <table className="shortcuts-table">
            <tbody>
              {shortcuts.map((row) => (
                <tr key={row.keys}>
                  <th>{row.keys}</th>
                  <td>{row.action}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="modal-actions">
            <button type="button" className="btn btn-active" onClick={onClose}>
              Close
            </button>
          </div>
        </div>
      </FocusTrap>
    </div>
  );
}

export function MenuChrome({
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
}: MenuChromeProps) {
  const { onTabContextMenu, overlay: tabMenuOverlay } = useTabContextMenu({ tabs, ...tabMenuApi });
  const showTabChrome = documentChromeVisible && workspaceView === 'tabs';
  const ribbonTabs = useMemo(
    () =>
      buildRibbonTabs({
        menus: menus.menus,
        quickAccess: menus.quickAccess,
        editTabContent: ribbonExtras.editTab,
        annotateOptions: ribbonExtras.annotateOptions,
      }),
    [menus, ribbonExtras],
  );
  return (
    <>
      <div className="menu-chrome">
        <Ribbon tabs={ribbonTabs} />
        {showTabChrome && (
          <>
            <TabBar
              tabs={tabs}
              activeId={activeTabId}
              onSelect={onSelectTab}
              onClose={onCloseTab}
              onTabContextMenu={onTabContextMenu}
            />
            {tabMenuOverlay}
          </>
        )}
      </div>
      {showCommandPalette && (
        <CommandPalette actions={menus.allActions} onClose={onCloseCommandPalette} />
      )}
      {showShortcutsHelp && (
        <ShortcutsModal
          bindings={shortcutBindings}
          onClose={onCloseShortcutsHelp}
        />
      )}

      {showLicenses && <LicensesModal onClose={onCloseLicenses} />}
      {showCredits && <CreditsModal onClose={onCloseCredits} />}
      {showAbout && <AboutModal onClose={onCloseAbout} />}
    </>
  );
}

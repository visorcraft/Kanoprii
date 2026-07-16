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
import { useTabContextMenu, type TabMenuApi } from '../chrome/useTabContextMenu';
import type { DocumentTabInfo } from '../app/documentSessionTypes';
import type { FlatMenuAction, MenuAction, MenuRoot } from './types';
import { MenuDropdownItem, runAction } from './MenuDropdownItem';
import { buildKeyboardShortcuts } from './buildMenuShortcuts';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { WorkspaceViewMode } from '../app/types';
import { useEscapeClose } from '../legal/useEscapeClose';
import { FocusTrap } from '../ui/FocusTrap';

type MenuChromeProps = {
  menus: MenuRoot[];
  quickAccess: MenuAction[];
  allActions: FlatMenuAction[];
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
  modeExtras?: React.ReactNode;
  tabs: DocumentTabInfo[];
  activeTabId: string | null;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  tabMenuApi: TabMenuApi;
  documentChromeVisible: boolean;
  workspaceView: WorkspaceViewMode;
  shortcutBindings: ShortcutBindings;
};

function MenuBar({ menus }: { menus: MenuRoot[] }) {
  const [openId, setOpenId] = useState<string | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    document.body.classList.toggle('kanoprii-menu-open', openId !== null);
    return () => document.body.classList.remove('kanoprii-menu-open');
  }, [openId]);

  useEffect(() => {
    if (!openId) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Element;
      if (!barRef.current?.contains(target) && !target.closest('.menu-dropdown-nested')) setOpenId(null);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpenId(null);
    };
    window.addEventListener('mousedown', onPointerDown);
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('mousedown', onPointerDown);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [openId]);

  return (
    <nav className="menu-bar" ref={barRef} aria-label="Application menu">
      {menus.map((menu) => (
        <div
          key={menu.id}
          className="menu-bar-entry"
          onMouseEnter={() => {
            if (openId !== null && openId !== menu.id && !menu.disabled) {
              setOpenId(menu.id);
            }
          }}
        >
          <button
            type="button"
            className={`menu-bar-trigger${openId === menu.id ? ' open' : ''}`}
            disabled={menu.disabled}
            data-testid={`menu-${menu.id}`}
            aria-haspopup="menu"
            aria-expanded={openId === menu.id}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() =>
              setOpenId((prev) => (prev === menu.id ? null : menu.id))
            }
          >
            {menu.label}
          </button>
          {openId === menu.id && !menu.disabled && (
            <div className="menu-dropdown" role="menu">
              {menu.items.map((entry, index) => (
                <MenuDropdownItem
                  key={`${menu.id}-${index}`}
                  entry={entry}
                  onClose={() => setOpenId(null)}
                />
              ))}
            </div>
          )}
        </div>
      ))}
    </nav>
  );
}

function QuickToolbar({ items }: { items: MenuAction[] }) {
  if (items.length === 0) return null;
  return (
    <div className="quick-toolbar" role="toolbar" aria-label="Quick access">
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          className={`btn${item.active ? ' btn-active' : ''}`}
          disabled={item.disabled}
          title={
            item.shortcut ? `${item.label} (${item.shortcut})` : item.label
          }
          data-testid={
            item.id === 'qa-save'
              ? 'save-pdf'
              : item.id === 'qa-rotate'
                ? 'rotate-page'
                : item.id === 'qa-undo'
                  ? 'undo-btn'
                  : item.id === 'qa-find'
                    ? 'find-btn'
                    : undefined
          }
          onClick={() => runAction(item)}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}

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
  quickAccess,
  allActions,
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
}: MenuChromeProps) {
  const { onTabContextMenu, overlay: tabMenuOverlay } = useTabContextMenu({ tabs, ...tabMenuApi });
  const showTabChrome = documentChromeVisible && workspaceView === 'tabs';
  return (
    <>
      <div className="menu-chrome">
        <MenuBar menus={menus} />
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
            {(quickAccess.length > 0 || modeExtras) && (
              <div className="quick-toolbar-row">
                <QuickToolbar items={quickAccess} />
                {modeExtras}
              </div>
            )}
          </>
        )}
      </div>
      {showCommandPalette && (
        <CommandPalette actions={allActions} onClose={onCloseCommandPalette} />
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

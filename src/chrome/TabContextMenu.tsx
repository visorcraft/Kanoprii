import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { TabMenuItem } from './tabMenuModel';

type TabContextMenuProps = {
  items: TabMenuItem[];
  x: number;
  y: number;
  onClose: () => void;
};

type NavItem = {
  kind: 'item';
  id: string;
  label: string;
  disabled?: boolean;
  onSelect: () => void;
  parentId: string | null;
};

type NavSubmenu = {
  kind: 'submenu';
  id: string;
  label: string;
  parentId: string | null;
  childIds: string[];
};

type NavEntry = NavItem | NavSubmenu;

function buildNav(items: TabMenuItem[]): NavEntry[] {
  const entries: NavEntry[] = [];
  function walk(list: TabMenuItem[], parentId: string | null): string[] {
    const ids: string[] = [];
    for (const it of list) {
      if (it.kind === 'divider') continue;
      if (it.kind === 'item') {
        ids.push(it.id);
        entries.push({ kind: 'item', id: it.id, label: it.label, disabled: it.disabled, onSelect: it.onSelect, parentId });
      } else {
        ids.push(it.id);
        const headerIdx = entries.length;
        entries.push({ kind: 'submenu', id: it.id, label: it.label, parentId, childIds: [] });
        const childIds = walk(it.items, it.id);
        (entries[headerIdx] as NavSubmenu).childIds = childIds;
      }
    }
    return ids;
  }
  walk(items, null);
  return entries;
}

function isVisible(entry: NavEntry, openSub: string | null): boolean {
  if (entry.parentId === null) return true;
  return entry.parentId === openSub;
}

function isSelectable(entry: NavEntry): boolean {
  return entry.kind === 'submenu' || !entry.disabled;
}

function findFirst(nav: NavEntry[], openSub: string | null): string | null {
  for (const e of nav) {
    if (isVisible(e, openSub) && isSelectable(e)) return e.id;
  }
  return null;
}

function findLast(nav: NavEntry[], openSub: string | null): string | null {
  for (let i = nav.length - 1; i >= 0; i--) {
    const e = nav[i];
    if (isVisible(e, openSub) && isSelectable(e)) return e.id;
  }
  return null;
}

function findNext(nav: NavEntry[], currentId: string, openSub: string | null): string {
  const idx = nav.findIndex((e) => e.id === currentId);
  if (idx === -1) return currentId;
  for (let offset = 1; offset <= nav.length; offset++) {
    const e = nav[(idx + offset) % nav.length];
    if (isVisible(e, openSub) && isSelectable(e)) return e.id;
  }
  return currentId;
}

function findPrev(nav: NavEntry[], currentId: string, openSub: string | null): string {
  const idx = nav.findIndex((e) => e.id === currentId);
  if (idx === -1) return currentId;
  for (let offset = 1; offset <= nav.length; offset++) {
    const e = nav[(idx - offset + nav.length) % nav.length];
    if (isVisible(e, openSub) && isSelectable(e)) return e.id;
  }
  return currentId;
}

function firstSelectableChildId(nav: NavEntry[], submenuId: string): string | null {
  const header = nav.find((e): e is NavSubmenu => e.kind === 'submenu' && e.id === submenuId);
  if (!header) return null;
  for (const childId of header.childIds) {
    const child = nav.find((e) => e.id === childId);
    if (child && isSelectable(child)) return child.id;
  }
  return null;
}

/** Themed, viewport-clamped popup menu with full keyboard support. */
export function TabContextMenu({ items, x, y, onClose }: TabContextMenuProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const instanceId = useId();
  const makeDomId = (id: string) => `${instanceId}--${id}`;
  const [pos, setPos] = useState({ left: x, top: y });
  const [openSub, setOpenSub] = useState<string | null>(null);
  const nav = useMemo(() => buildNav(items), [items]);
  const [highlightedId, setHighlightedId] = useState<string | null>(() => findFirst(nav, null));

  // Focus the menu as soon as it is mounted.
  useLayoutEffect(() => {
    rootRef.current?.focus();
  }, []);

  // Dismiss on outside click or window blur.
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener('mousedown', onDown, true);
    window.addEventListener('blur', onClose);
    return () => {
      window.removeEventListener('mousedown', onDown, true);
      window.removeEventListener('blur', onClose);
    };
  }, [onClose]);

  // Flip horizontally/vertically so the menu stays on-screen.
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const left = x + rect.width > window.innerWidth ? Math.max(4, x - rect.width) : x;
    const top = y + rect.height > window.innerHeight ? Math.max(4, y - rect.height) : y;
    setPos({ left, top });
  }, [x, y, items]);

  // Keep the highlighted item visible when the open submenu changes.
  useEffect(() => {
    const entry = highlightedId ? nav.find((e) => e.id === highlightedId) : null;
    if (!entry || !isVisible(entry, openSub)) {
      if (entry?.parentId) {
        const parent = nav.find((e) => e.id === entry.parentId);
        if (parent && isVisible(parent, openSub)) {
          setHighlightedId(parent.id);
          return;
        }
      }
      const first = findFirst(nav, openSub);
      if (first) setHighlightedId(first);
    }
  }, [openSub, nav, highlightedId]);

  const activate = (disabled: boolean | undefined, onSelect: () => void) => {
    if (disabled) return;
    onSelect();
    onClose();
  };

  const openSubmenu = (submenuId: string) => {
    setOpenSub(submenuId);
    const childId = firstSelectableChildId(nav, submenuId);
    if (childId) setHighlightedId(childId);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!highlightedId) return;
    const current = nav.find((e) => e.id === highlightedId);
    if (!current) return;

    switch (e.key) {
      case 'ArrowDown': {
        e.preventDefault();
        setHighlightedId(findNext(nav, highlightedId, openSub));
        break;
      }
      case 'ArrowUp': {
        e.preventDefault();
        setHighlightedId(findPrev(nav, highlightedId, openSub));
        break;
      }
      case 'ArrowRight': {
        if (current.kind === 'submenu') {
          e.preventDefault();
          openSubmenu(current.id);
        }
        break;
      }
      case 'ArrowLeft': {
        if (current.parentId) {
          e.preventDefault();
          setOpenSub(null);
          setHighlightedId(current.parentId);
        }
        break;
      }
      case 'Enter':
      case ' ': {
        e.preventDefault();
        if (current.kind === 'submenu') {
          openSubmenu(current.id);
        } else if (!current.disabled) {
          current.onSelect();
          onClose();
        }
        break;
      }
      case 'Escape': {
        e.preventDefault();
        onClose();
        break;
      }
      case 'Tab': {
        e.preventDefault();
        if (e.shiftKey) {
          setHighlightedId(findPrev(nav, highlightedId, openSub));
        } else {
          setHighlightedId(findNext(nav, highlightedId, openSub));
        }
        break;
      }
      case 'Home': {
        e.preventDefault();
        const first = findFirst(nav, openSub);
        if (first) setHighlightedId(first);
        break;
      }
      case 'End': {
        e.preventDefault();
        const last = findLast(nav, openSub);
        if (last) setHighlightedId(last);
        break;
      }
    }
  };

  const renderItems = (list: TabMenuItem[]) => {
    return list.map((it, i) => {
      if (it.kind === 'divider') {
        return <div key={`d${i}`} className="tcm-divider" role="separator" />;
      }

      const highlighted = highlightedId === it.id;
      const baseClass = 'tcm-item';
      const className =
        baseClass +
        (highlighted ? ' highlighted' : '') +
        (it.kind === 'submenu' ? ' tcm-has-sub' : '');

      if (it.kind === 'submenu') {
        return (
          <div
            key={it.id}
            className="tcm-submenu-wrap"
            onMouseEnter={() => {
              setOpenSub(it.id);
              setHighlightedId(it.id);
            }}
            onMouseLeave={() => setOpenSub((cur) => (cur === it.id ? null : cur))}
          >
            <button
              type="button"
              className={className}
              role="menuitem"
              tabIndex={-1}
              id={makeDomId(it.id)}
              data-menuitem-id={it.id}
              aria-haspopup="true"
              aria-expanded={openSub === it.id}
              onClick={() => openSubmenu(it.id)}
            >
              <span>{it.label}</span>
              <span className="tcm-arrow" aria-hidden>
                ›
              </span>
            </button>
            {openSub === it.id && (
              <div className="tcm-submenu" role="menu">
                {renderItems(it.items)}
              </div>
            )}
          </div>
        );
      }

      return (
        <button
          key={it.id}
          type="button"
          className={className}
          role="menuitem"
          tabIndex={-1}
          id={makeDomId(it.id)}
          data-menuitem-id={it.id}
          disabled={it.disabled}
          onMouseEnter={() => {
            setOpenSub(null);
            setHighlightedId(it.id);
          }}
          onClick={() => activate(it.disabled, it.onSelect)}
        >
          {it.label}
        </button>
      );
    });
  };

  return (
    <div
      ref={rootRef}
      className="tab-context-menu"
      style={{ left: pos.left, top: pos.top }}
      role="menu"
      tabIndex={-1}
      aria-label="Tab options"
      aria-activedescendant={highlightedId ? makeDomId(highlightedId) : undefined}
      onKeyDown={onKeyDown}
    >
      {renderItems(items)}
    </div>
  );
}

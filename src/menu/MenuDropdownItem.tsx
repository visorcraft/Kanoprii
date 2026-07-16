import { useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { MenuAction, MenuEntry } from './types';

export function runAction(action: MenuAction) {
  if (action.disabled) return;
  void action.run();
}

export function MenuDropdownItem({
  entry,
  onClose,
}: {
  entry: MenuEntry;
  onClose: () => void;
}) {
  const [subOpen, setSubOpen] = useState(false);
  const itemRef = useRef<HTMLDivElement>(null);
  const submenuRef = useRef<HTMLDivElement>(null);
  const [submenuPosition, setSubmenuPosition] = useState({ left: 0, top: 0 });

  useLayoutEffect(() => {
    if (!subOpen || !itemRef.current || !submenuRef.current) return;
    const item = itemRef.current.getBoundingClientRect();
    const submenu = submenuRef.current.getBoundingClientRect();
    setSubmenuPosition({
      left: item.right + submenu.width + 2 > window.innerWidth
        ? item.left - submenu.width - 2
        : item.right + 2,
      top: Math.max(4, Math.min(item.top - 6, window.innerHeight - submenu.height - 4)),
    });
  }, [subOpen]);

  if ('separator' in entry) {
    return <div className="menu-separator" role="separator" />;
  }

  if ('items' in entry && !('id' in entry)) {
    return (
      <div
        ref={itemRef}
        className="menu-item menu-item-submenu"
        data-testid={`submenu-${entry.label.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`}
        onMouseEnter={() => setSubOpen(true)}
        onMouseLeave={(event) => {
          if (!submenuRef.current?.contains(event.relatedTarget as Node)) setSubOpen(false);
        }}
      >
        <span className="menu-item-label">{entry.label}</span>
        <span className="menu-item-chevron">›</span>
        {subOpen && createPortal(
          <div
            ref={submenuRef}
            className="menu-dropdown menu-dropdown-nested"
            style={submenuPosition}
            onMouseLeave={(event) => {
              const related = event.relatedTarget as Element | null;
              if (!itemRef.current?.contains(related) && !related?.closest('.menu-dropdown-nested')) setSubOpen(false);
            }}
          >
            {entry.items.map((child, index) => (
              <MenuDropdownItem
                key={`${entry.label}-${index}`}
                entry={child}
                onClose={onClose}
              />
            ))}
          </div>,
          document.body,
        )}
      </div>
    );
  }

  const action = entry as MenuAction;
  return (
    <button
      type="button"
      className={`menu-item${action.danger ? ' danger' : ''}${action.active ? ' active' : ''}`}
      disabled={action.disabled}
      data-testid={action.id === 'open' ? 'open-pdf' : action.id}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => {
        runAction(action);
        onClose();
      }}
    >
      <span className="menu-item-label">{action.label}</span>
      {action.shortcut && (
        <span className="menu-item-shortcut">{action.shortcut}</span>
      )}
    </button>
  );
}

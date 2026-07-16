import type { MenuEntry } from '../menu/types';
import { MenuDropdownItem } from '../menu/MenuDropdownItem';
import { RibbonIcon, type RibbonIconName } from './icons';

export function RibbonDropdown({
  id,
  label,
  icon,
  items,
  danger = false,
  disabled = false,
  open,
  onToggle,
  onClose,
  variant = 'button',
}: {
  id: string;
  label: string;
  icon?: RibbonIconName;
  items: MenuEntry[];
  danger?: boolean;
  disabled?: boolean;
  open: boolean;
  onToggle: () => void;
  onClose: () => void;
  variant?: 'button' | 'tab';
}) {
  const slug = label.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  return (
    <div className={`ribbon-dropdown-wrap${open ? ' open' : ''}`}>
      <button
        type="button"
        className={
          variant === 'tab'
            ? `ribbon-tab ribbon-tab-menu${open ? ' open' : ''}`
            : `ribbon-btn ribbon-btn-dropdown${open ? ' open' : ''}${danger ? ' danger' : ''}`
        }
        disabled={disabled}
        data-testid={variant === 'tab' ? `menu-${id}` : `submenu-${slug}`}
        aria-haspopup="menu"
        aria-expanded={open}
        onMouseDown={(e) => e.preventDefault()}
        onClick={onToggle}
      >
        {icon && variant === 'button' && <RibbonIcon name={icon} />}
        <span className={variant === 'tab' ? undefined : 'ribbon-btn-label'}>{label}</span>
        {variant === 'button' && <span className="ribbon-dropdown-caret" aria-hidden="true">▾</span>}
      </button>
      {open && !disabled && (
        <div className="menu-dropdown ribbon-dropdown" role="menu">
          {items.map((entry, index) => (
            <MenuDropdownItem key={`${id}-${index}`} entry={entry} onClose={onClose} />
          ))}
        </div>
      )}
    </div>
  );
}

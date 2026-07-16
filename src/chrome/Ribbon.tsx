import { useEffect, useRef, useState } from 'react';
import { runAction } from '../menu/MenuDropdownItem';
import { RibbonIcon } from './icons';
import { RibbonDropdown } from './RibbonDropdown';
import type { RibbonControl, RibbonGroup, RibbonTabDef } from './ribbonModel';

const COLLAPSED_KEY = 'kanoprii-ribbon-collapsed';

/** Legacy e2e testids preserved from the old QuickToolbar / menu open action. */
function buttonTestId(actionId: string): string {
  switch (actionId) {
    case 'open': return 'open-pdf';
    case 'qa-save': return 'save-pdf';
    case 'qa-undo': return 'undo-btn';
    case 'qa-find': return 'find-btn';
    case 'qa-rotate': return 'rotate-page';
    default: return actionId;
  }
}

function RibbonButtonView({ control }: { control: Extract<RibbonControl, { kind: 'button' }> }) {
  const { action } = control;
  const label = action.label.replace(/ \(on\)$/, '');
  return (
    <button
      type="button"
      className={`ribbon-btn${action.active ? ' active' : ''}${action.danger ? ' danger' : ''}`}
      disabled={action.disabled}
      aria-pressed={action.active ?? undefined}
      title={action.shortcut ? `${label} (${action.shortcut})` : label}
      data-testid={buttonTestId(action.id)}
      onClick={() => runAction(action)}
    >
      <RibbonIcon name={control.icon} />
      <span className="ribbon-btn-label">{label}</span>
    </button>
  );
}

function RibbonGroupView({
  group,
  openDropdown,
  onToggleDropdown,
  onCloseDropdown,
}: {
  group: RibbonGroup;
  openDropdown: string | null;
  onToggleDropdown: (id: string) => void;
  onCloseDropdown: () => void;
}) {
  return (
    <div className="ribbon-group" role="group" aria-label={group.label}>
      <div className="ribbon-group-controls">
        {group.controls.map((control) =>
          control.kind === 'button' ? (
            <RibbonButtonView key={control.action.id} control={control} />
          ) : (
            <RibbonDropdown
              key={control.id}
              id={control.id}
              label={control.label}
              icon={control.icon}
              items={control.items}
              danger={control.danger}
              open={openDropdown === control.id}
              onToggle={() => onToggleDropdown(control.id)}
              onClose={onCloseDropdown}
            />
          ),
        )}
      </div>
      <span className="ribbon-group-label">{group.label}</span>
    </div>
  );
}

export function Ribbon({ tabs }: { tabs: RibbonTabDef[] }) {
  const [activeTab, setActiveTab] = useState('home');
  const [openDropdown, setOpenDropdown] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem(COLLAPSED_KEY) === '1';
    } catch {
      return false;
    }
  });
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    document.body.classList.toggle('kanoprii-menu-open', openDropdown !== null);
    return () => document.body.classList.remove('kanoprii-menu-open');
  }, [openDropdown]);

  useEffect(() => {
    if (!openDropdown) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Element;
      if (!rootRef.current?.contains(target) && !target.closest('.menu-dropdown-nested')) {
        setOpenDropdown(null);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpenDropdown(null);
    };
    window.addEventListener('mousedown', onPointerDown);
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('mousedown', onPointerDown);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [openDropdown]);

  const toggleCollapsed = () => {
    setCollapsed((prev) => {
      const next = !prev;
      try {
        window.localStorage.setItem(COLLAPSED_KEY, next ? '1' : '0');
      } catch {
        /* localStorage unavailable */
      }
      return next;
    });
  };

  const selectTab = (tab: RibbonTabDef) => {
    if (tab.disabled) return;
    setOpenDropdown(null);
    setActiveTab(tab.id);
    if (collapsed) toggleCollapsed();
  };

  /** Arrow-key focus movement across the ribbon body's controls. */
  const onBodyKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) return;
    const focusables = Array.from(
      event.currentTarget.querySelectorAll<HTMLElement>(
        'button:not([disabled]), select:not([disabled]), input:not([disabled])',
      ),
    );
    if (focusables.length === 0) return;
    const index = focusables.indexOf(document.activeElement as HTMLElement);
    if (index === -1) return;
    const delta = event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 1;
    event.stopPropagation();
    event.preventDefault();
    focusables[(index + delta + focusables.length) % focusables.length].focus();
  };

  const active = tabs.find((t) => t.id === activeTab && t.kind === 'tab' && !t.disabled) as
    | Extract<RibbonTabDef, { kind: 'tab' }>
    | undefined;

  /**
   * Partition so the tablist contains only role="tab" buttons
   * (aria-required-children): leading menu tabs, body tabs, trailing menu tabs.
   */
  type MenuTab = Extract<RibbonTabDef, { kind: 'menu' }>;
  type BodyTab = Extract<RibbonTabDef, { kind: 'tab' }>;
  const firstTabIndex = tabs.findIndex((t) => t.kind === 'tab');
  const splitAt = firstTabIndex === -1 ? tabs.length : firstTabIndex;
  const leadingMenus = tabs.slice(0, splitAt).filter((t): t is MenuTab => t.kind === 'menu');
  const bodyTabs = tabs.filter((t): t is BodyTab => t.kind === 'tab');
  const trailingMenus = tabs.slice(splitAt).filter((t): t is MenuTab => t.kind === 'menu');

  const renderMenuTab = (tab: MenuTab) => (
    <RibbonDropdown
      key={tab.id}
      variant="tab"
      id={tab.id}
      label={tab.label}
      items={tab.items}
      disabled={tab.disabled}
      open={openDropdown === tab.id}
      onToggle={() => setOpenDropdown((prev) => (prev === tab.id ? null : tab.id))}
      onClose={() => setOpenDropdown(null)}
    />
  );

  return (
    <div className="ribbon" ref={rootRef}>
      <nav className="ribbon-tabs" aria-label="Ribbon">
        {leadingMenus.map(renderMenuTab)}
        <div className="ribbon-tablist" role="tablist" aria-label="Ribbon">
          {bodyTabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              id={`ribbon-tab-${tab.id}`}
              aria-controls="ribbon-panel"
              aria-selected={activeTab === tab.id && !collapsed}
              className={`ribbon-tab${activeTab === tab.id && !collapsed ? ' active' : ''}`}
              disabled={tab.disabled}
              data-testid={`menu-${tab.id}`}
              onClick={() => selectTab(tab)}
            >
              {tab.label}
            </button>
          ))}
        </div>
        {trailingMenus.map(renderMenuTab)}
        <span className="ribbon-tabs-spacer" />
        <button
          type="button"
          className="ribbon-collapse"
          onClick={toggleCollapsed}
          aria-label={collapsed ? 'Expand ribbon' : 'Collapse ribbon'}
          aria-expanded={!collapsed}
          title={collapsed ? 'Expand ribbon' : 'Collapse ribbon'}
          data-testid="ribbon-collapse"
        >
          <RibbonIcon name={collapsed ? 'chevron-down' : 'chevron-up'} className="ribbon-collapse-icon" />
        </button>
      </nav>
      {!collapsed && active && (
        <div
          className="ribbon-body"
          role="tabpanel"
          id="ribbon-panel"
          aria-labelledby={active ? `ribbon-tab-${active.id}` : undefined}
          onKeyDown={onBodyKeyDown}
        >
          {active.groups.filter((group) => group.controls.length > 0).map((group) => (
            <RibbonGroupView
              key={group.id}
              group={group}
              openDropdown={openDropdown}
              onToggleDropdown={(id) => setOpenDropdown((prev) => (prev === id ? null : id))}
              onCloseDropdown={() => setOpenDropdown(null)}
            />
          ))}
          {active.extras}
        </div>
      )}
    </div>
  );
}

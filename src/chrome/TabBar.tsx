import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { DocumentTabInfo } from '../app/documentSessionTypes';

type TabBarProps = {
  tabs: DocumentTabInfo[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onTabContextMenu?: (id: string, x: number, y: number) => void;
};

const SCROLL_EDGE_EPSILON = 2;

export function TabBar({ tabs, activeId, onSelect, onClose, onTabContextMenu }: TabBarProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);
  const [focusedTabId, setFocusedTabId] = useState<string | null>(activeId);
  const lastActiveIdRef = useRef(activeId);

  // Sync roving focus to the active tab when activation changes, and fall back
  // if the focused tab is removed from the tab list.
  useEffect(() => {
    const activeChanged = activeId !== lastActiveIdRef.current;
    lastActiveIdRef.current = activeId;

    setFocusedTabId((prev) => {
      if (activeChanged && activeId && tabs.some((t) => t.id === activeId)) {
        return activeId;
      }
      if (prev && tabs.some((t) => t.id === prev)) {
        return prev;
      }
      if (activeId && tabs.some((t) => t.id === activeId)) {
        return activeId;
      }
      return tabs[0]?.id ?? null;
    });
  }, [activeId, tabs]);

  // The scroll arrows are absolute overlays (see styles.css), so they never
  // take layout space - the scroll viewport width is constant whether or not an
  // arrow is showing. That keeps these thresholds (and the scroll targets below)
  // stable, so a single click always lands exactly on the intended tab.
  const checkScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const scrollLeft = el.scrollLeft;
    setCanScrollLeft(scrollLeft > SCROLL_EDGE_EPSILON);
    setCanScrollRight(scrollLeft + el.clientWidth < el.scrollWidth - SCROLL_EDGE_EPSILON);
  }, []);

  useLayoutEffect(() => {
    checkScroll();
  }, [checkScroll, tabs]);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    el.addEventListener('scroll', checkScroll);
    window.addEventListener('resize', checkScroll);
    return () => {
      el.removeEventListener('scroll', checkScroll);
      window.removeEventListener('resize', checkScroll);
    };
  }, [checkScroll]);

  // Bring a tab fully into view. Snap to the true edge when the target is the
  // first/last tab so that arrow collapses and the tab is fully revealed.
  const scrollTabIntoView = useCallback((tab: HTMLElement) => {
    const el = scrollRef.current;
    if (!el) return;
    const items = Array.from(el.querySelectorAll<HTMLElement>('.tab-item'));
    if (items.length === 0) return;

    let target: number;
    if (tab === items[items.length - 1]) {
      target = el.scrollWidth; // clamps to max → right arrow collapses
    } else if (tab === items[0]) {
      target = 0;
    } else {
      const right = tab.offsetLeft + tab.offsetWidth;
      if (right > el.scrollLeft + el.clientWidth) target = right - el.clientWidth;
      else if (tab.offsetLeft < el.scrollLeft) target = tab.offsetLeft;
      else return; // already fully visible
    }
    el.scrollTo({ left: target, behavior: 'smooth' });
  }, []);

  // Keep the active tab fully visible whenever it changes.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el || !activeId) return;
    const active = el.querySelector<HTMLElement>('.tab-item.active');
    if (active) scrollTabIntoView(active);
  }, [activeId, tabs, scrollTabIntoView]);

  // Move DOM focus to the roving tabindex target and reveal it.
  const focusTabElement = useCallback((id: string | null) => {
    const el = scrollRef.current;
    if (!el || !id) return;
    const tab = el.querySelector<HTMLElement>(`.tab-item[data-tab-id="${id}"]`);
    if (tab) {
      tab.focus();
      scrollTabIntoView(tab);
    }
  }, [scrollTabIntoView]);

  useEffect(() => {
    focusTabElement(focusedTabId);
  }, [focusedTabId, focusTabElement]);

  const scrollToNext = useCallback(
    (direction: 'left' | 'right') => {
      const el = scrollRef.current;
      if (!el) return;
      const items = Array.from(el.querySelectorAll<HTMLElement>('.tab-item'));
      if (items.length === 0) return;
      const visibleRight = el.scrollLeft + el.clientWidth;
      const target =
        direction === 'right'
          ? items.find((item) => item.offsetLeft + item.offsetWidth > visibleRight + 1)
          : [...items].reverse().find((item) => item.offsetLeft < el.scrollLeft - 1);
      if (target) scrollTabIntoView(target);
    },
    [scrollTabIntoView],
  );

  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>, tab: DocumentTabInfo) => {
      if (!tabs.length) return;
      const index = tabs.findIndex((t) => t.id === tab.id);
      if (index === -1) return;

      const moveFocus = (nextId: string) => {
        e.preventDefault();
        setFocusedTabId(nextId);
      };

      switch (e.key) {
        case 'ArrowLeft':
          moveFocus(tabs[(index - 1 + tabs.length) % tabs.length].id);
          break;
        case 'ArrowRight':
          moveFocus(tabs[(index + 1) % tabs.length].id);
          break;
        case 'Home':
          moveFocus(tabs[0].id);
          break;
        case 'End':
          moveFocus(tabs[tabs.length - 1].id);
          break;
        case 'Delete':
          e.preventDefault();
          onClose(tab.id);
          break;
        case 'Enter':
        case ' ':
          e.preventDefault();
          onSelect(tab.id);
          break;
        case 'F10':
          if (e.shiftKey && onTabContextMenu) {
            e.preventDefault();
            const el = scrollRef.current?.querySelector<HTMLElement>(`.tab-item[data-tab-id="${tab.id}"]`);
            if (el) {
              const rect = el.getBoundingClientRect();
              onTabContextMenu(tab.id, rect.left + rect.width / 2, rect.top + rect.height / 2);
            }
          }
          break;
        case 'ContextMenu':
          if (onTabContextMenu) {
            e.preventDefault();
            const el = scrollRef.current?.querySelector<HTMLElement>(`.tab-item[data-tab-id="${tab.id}"]`);
            if (el) {
              const rect = el.getBoundingClientRect();
              onTabContextMenu(tab.id, rect.left + rect.width / 2, rect.top + rect.height / 2);
            }
          }
          break;
      }
    },
    [tabs, onClose, onSelect, onTabContextMenu]
  );

  if (tabs.length <= 1) return null;

  return (
    <div className="tab-bar" role="tablist">
      {canScrollLeft && (
        <button
          type="button"
          className="tab-scroll-btn tab-scroll-btn-left"
          aria-label="Scroll tabs left"
          onClick={() => scrollToNext('left')}
        >
          ‹
        </button>
      )}
      <div className="tab-bar-scroll" ref={scrollRef}>
        {tabs.map((tab) => {
          const active = tab.id === activeId;
          const focused = tab.id === focusedTabId;
          return (
            <div
              key={tab.id}
              role="tab"
              aria-selected={active}
              aria-label={tab.label}
              tabIndex={focused ? 0 : -1}
              data-tab-id={tab.id}
              data-testid={`doc-tab-${tab.label}`}
              data-working-path={import.meta.env.VITE_WDIO === '1' ? tab.filePath || undefined : undefined}
              className={`tab-item${active ? ' active' : ''}`}
              onClick={() => onSelect(tab.id)}
              onKeyDown={(e) => handleTabKeyDown(e, tab)}
              onContextMenu={
                onTabContextMenu
                  ? (e) => {
                      e.preventDefault();
                      onTabContextMenu(tab.id, e.clientX, e.clientY);
                    }
                  : undefined
              }
              onMouseDown={(e) => {
                if (e.button === 1) {
                  e.preventDefault();
                  onClose(tab.id);
                }
              }}
            >
              {tab.dirty && <span className="tab-dirty" aria-hidden />}
              <span className="tab-label">{tab.label}</span>
              <button
                type="button"
                className="tab-close"
                tabIndex={-1}
                aria-label={`Close ${tab.label}`}
                data-testid={`doc-tab-close-${tab.label}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onClose(tab.id);
                }}
              >
                ×
              </button>
            </div>
          );
        })}
      </div>
      {canScrollRight && (
        <button
          type="button"
          className="tab-scroll-btn tab-scroll-btn-right"
          aria-label="Scroll tabs right"
          onClick={() => scrollToNext('right')}
        >
          ›
        </button>
      )}
    </div>
  );
}

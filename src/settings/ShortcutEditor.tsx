import { useMemo, useState } from 'react';
import { SHORTCUT_COMMAND_MAP, SHORTCUT_REGISTRY, type ShortcutCategory, type ShortcutCommandId } from './shortcutRegistry';
import { normalizeShortcut, shortcutToDisplay } from './shortcutKeys';
import { ShortcutCapture } from './ShortcutCapture';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';

type ShortcutEditorProps = {
  bindings: ShortcutBindings;
  setBinding: (commandId: ShortcutCommandId, shortcuts: string[]) => void;
  resetBinding: (commandId: ShortcutCommandId) => void;
};

const CATEGORY_ORDER: ShortcutCategory[] = [
  'Global',
  'File',
  'Edit',
  'Pages',
  'View',
  'Document',
  'Annotation',
  'Navigation',
  'Tabs',
];

function findConflicts(bindings: ShortcutBindings): Map<ShortcutCommandId, string> {
  const seen = new Map<string, ShortcutCommandId>();
  const conflicts = new Map<ShortcutCommandId, string>();
  for (const [commandId, shortcuts] of Object.entries(bindings) as [ShortcutCommandId, string[]][]) {
    for (const shortcut of shortcuts) {
      const normalized = normalizeShortcut(shortcut);
      if (!normalized) continue;
      const existing = seen.get(normalized);
      if (existing && existing !== commandId) {
        conflicts.set(commandId, existing);
        conflicts.set(existing, commandId);
      } else {
        seen.set(normalized, commandId);
      }
    }
  }
  return conflicts;
}

export function ShortcutEditor({ bindings, setBinding, resetBinding }: ShortcutEditorProps) {
  const [query, setQuery] = useState('');

  const groups = useMemo(() => {
    const map = new Map<ShortcutCategory, typeof SHORTCUT_REGISTRY>();
    for (const binding of SHORTCUT_REGISTRY) {
      if (query) {
        const q = query.toLowerCase();
        const match =
          binding.label.toLowerCase().includes(q) ||
          binding.commandId.toLowerCase().includes(q) ||
          binding.category.toLowerCase().includes(q);
        if (!match) continue;
      }
      const list = map.get(binding.category) ?? [];
      list.push(binding);
      map.set(binding.category, list);
    }
    return CATEGORY_ORDER.map((category) => [category, map.get(category) ?? []] as const).filter(
      ([, list]) => list.length > 0,
    );
  }, [query]);

  const conflicts = useMemo(() => findConflicts(bindings), [bindings]);

  return (
    <div className="shortcut-editor">
      <div className="shortcut-editor-search">
        <input
          type="search"
          className="shortcut-editor-search-input"
          placeholder="Search shortcuts..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Search shortcuts"
        />
      </div>
      {groups.map(([category, items]) => (
        <div key={category} className="shortcut-group">
          <h3 className="shortcut-group-title">{category}</h3>
          <ul className="shortcut-group-list">
            {items.map((item) => {
              const commandId = item.commandId;
              const current = bindings[commandId] ?? item.defaultShortcuts;
              const conflictCommandId = conflicts.get(commandId);
              const conflictLabel = conflictCommandId
                ? SHORTCUT_COMMAND_MAP[conflictCommandId as ShortcutCommandId]?.label ?? conflictCommandId
                : null;

              return (
                <li key={commandId} className="shortcut-row">
                  <span className="shortcut-row-label">{item.label}</span>
                  <div className="shortcut-row-bindings">
                    {current.map((shortcut, i) => (
                      <span key={`${shortcut}-${i}`} className="shortcut-chip">
                        {shortcutToDisplay(shortcut)}
                      </span>
                    ))}
                  </div>
                  <div className="shortcut-row-actions">
                    <ShortcutCapture
                      commandId={commandId}
                      bindings={bindings}
                      onCapture={(shortcut) => setBinding(commandId, [shortcut])}
                    />
                    <button
                      type="button"
                      className="shortcut-row-reset"
                      onClick={() => resetBinding(commandId)}
                      aria-label={`Reset ${item.label} shortcut`}
                    >
                      Reset
                    </button>
                  </div>
                  {conflictLabel && (
                    <span className="shortcut-row-conflict">
                      Conflicts with {conflictLabel}
                    </span>
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      ))}
      {groups.length === 0 && (
        <p className="shortcut-editor-empty">No shortcuts match your search.</p>
      )}
    </div>
  );
}

import { useCallback, useMemo, useState } from 'react';
import { getDefaultShortcuts, type ShortcutCommandId } from '../settings/shortcutRegistry';
import { loadShortcutOverrides, normalizeShortcut, saveShortcutOverrides } from '../settings/shortcutKeys';

export type ShortcutBindings = Record<ShortcutCommandId, string[]>;

export type ShortcutBindingsState = {
  bindings: ShortcutBindings;
  setBinding: (commandId: ShortcutCommandId, shortcuts: string[]) => void;
  resetBinding: (commandId: ShortcutCommandId) => void;
  resetAllBindings: () => void;
};

export function useShortcutBindingsState() {
  const [overrides, setOverrides] = useState<Partial<ShortcutBindings>>(() => loadShortcutOverrides());

  const bindings = useMemo<ShortcutBindings>(() => {
    const defaults = getDefaultShortcuts();
    return { ...defaults, ...overrides };
  }, [overrides]);

  const setBinding = useCallback((commandId: ShortcutCommandId, shortcuts: string[]) => {
    setOverrides((prev) => {
      const defaults = getDefaultShortcuts();
      const defaultShortcuts = defaults[commandId];
      const next = { ...prev };
      const normalized = shortcuts
        .map((s) => normalizeShortcut(s))
        .filter((s): s is string => s !== null);
      const sameAsDefault =
        normalized.length === defaultShortcuts.length &&
        normalized.every((s, i) => s === defaultShortcuts[i]);
      if (sameAsDefault) {
        delete next[commandId];
      } else {
        next[commandId] = normalized;
      }
      saveShortcutOverrides(next);
      return next;
    });
  }, []);

  const resetBinding = useCallback((commandId: ShortcutCommandId) => {
    setOverrides((prev) => {
      const next = { ...prev };
      delete next[commandId];
      saveShortcutOverrides(next);
      return next;
    });
  }, []);

  const resetAllBindings = useCallback(() => {
    setOverrides({});
    saveShortcutOverrides({});
  }, []);

  return { bindings, overrides, setBinding, resetBinding, resetAllBindings };
}

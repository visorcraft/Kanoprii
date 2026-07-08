import { useEffect, type MutableRefObject } from 'react';
import type { AppKeyboardActions } from './buildAppKeyboardActions';
import { SHORTCUT_HANDLERS } from './appShortcutHandlers';
import { type ShortcutCommandId } from '../settings/shortcutRegistry';
import { eventToShortcut, normalizeShortcut } from '../settings/shortcutKeys';
import type { ShortcutBindings } from './useShortcutBindingsState';

export type { AppKeyboardActions } from './buildAppKeyboardActions';

function isTextInput(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

function isShortcutCapture(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && target.dataset.shortcutCapture === 'true';
}

function buildShortcutIndex(bindings: Record<ShortcutCommandId, string[]>): Map<string, ShortcutCommandId> {
  const index = new Map<string, ShortcutCommandId>();
  for (const [commandId, shortcuts] of Object.entries(bindings) as [ShortcutCommandId, string[]][]) {
    for (const shortcut of shortcuts) {
      const normalized = normalizeShortcut(shortcut);
      if (normalized) {
        index.set(normalized, commandId);
      }
    }
  }
  return index;
}

const GLOBAL_COMMANDS = new Set<ShortcutCommandId>(['open-pdf', 'command-palette', 'quit']);

export function useAppKeyboard(
  actionsRef: MutableRefObject<AppKeyboardActions>,
  shortcutBindings: ShortcutBindings,
  activeSurface: 'document' | 'settings' = 'document',
) {
  useEffect(() => {
    const shortcutIndex = buildShortcutIndex(shortcutBindings);

    const onKeyDown = (e: KeyboardEvent) => {
      const a = actionsRef.current;

      if (e.key === 'Escape') {
        if (activeSurface === 'settings') {
          if (a.anyModalOpen) {
            a.dismissModals();
          }
          return;
        }
        if (a.noteMode && a.hasOpenPdf) { a.exitNoteMode(); return; }
        if (a.drawMode && a.hasOpenPdf) { a.exitDrawMode(); return; }
        if (a.shapeMode && a.hasOpenPdf) { a.exitShapeMode(); return; }
        if (a.stampMode && a.hasOpenPdf) { a.exitStampMode(); return; }
        if (a.redactMode && a.hasOpenPdf) { a.exitRedactMode(); return; }
        if (a.imageInsertMode && a.hasOpenPdf) { a.exitImageInsertMode(); return; }
        if (a.textEditMode && a.hasOpenPdf) { a.exitTextEditMode(); return; }
        if (a.vectorEditMode && a.hasOpenPdf) { a.exitVectorEditMode(); return; }
        if (a.formAddMode && a.hasOpenPdf) { a.exitFormAddMode(); return; }
        if (a.highlightMode && a.hasOpenPdf) { a.exitHighlightMode(); return; }
        if (a.anyModalOpen) { a.dismissModals(); return; }
        return;
      }

      if (isTextInput(e.target) && !isShortcutCapture(e.target)) return;

      const shortcut = eventToShortcut(e);
      if (!shortcut) return;

      const commandId = shortcutIndex.get(shortcut);
      if (!commandId) return;

      if (activeSurface === 'settings' && !GLOBAL_COMMANDS.has(commandId)) {
        return;
      }

      const handler = SHORTCUT_HANDLERS[commandId];
      if (!handler) return;

      if (!handler.enabled(a)) return;

      e.preventDefault();
      void handler.run(a);
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [actionsRef, shortcutBindings, activeSurface]);
}

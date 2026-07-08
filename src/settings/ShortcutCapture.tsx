import { useCallback, useEffect, useRef, useState } from 'react';
import {
  eventToShortcut,
  isReservedShortcut,
  isShortcutConflict,
  shortcutToDisplay,
  type Shortcut,
} from './shortcutKeys';
import { SHORTCUT_COMMAND_MAP, type ShortcutCommandId } from './shortcutRegistry';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';

type ShortcutCaptureProps = {
  commandId: ShortcutCommandId;
  bindings: ShortcutBindings;
  onCapture: (shortcut: Shortcut) => void;
};

type CaptureState =
  | { kind: 'idle' }
  | { kind: 'capturing'; captured: Shortcut | null; error: string | null };

function conflictLabel(commandId: ShortcutCommandId): string {
  return SHORTCUT_COMMAND_MAP[commandId]?.label ?? commandId;
}

export function ShortcutCapture({ commandId, bindings, onCapture }: ShortcutCaptureProps) {
  const ref = useRef<HTMLButtonElement>(null);
  const [state, setState] = useState<CaptureState>({ kind: 'idle' });

  const reset = useCallback(() => setState({ kind: 'idle' }), []);

  useEffect(() => {
    if (state.kind !== 'capturing') return;

    const element = ref.current;
    if (!element) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        reset();
        element.blur();
        return;
      }

      if (e.key === 'Enter') {
        if (state.captured && !state.error) {
          onCapture(state.captured);
        }
        reset();
        element.blur();
        return;
      }

      const captured = eventToShortcut(e);
      if (!captured) {
        setState({ kind: 'capturing', captured: null, error: 'Invalid shortcut' });
        return;
      }

      let error: string | null = null;
      if (isReservedShortcut(captured)) {
        error = 'Reserved shortcut';
      } else {
        const conflict = isShortcutConflict(bindings, commandId, captured);
        if (conflict) {
          error = `Conflicts with ${conflictLabel(conflict.commandId)}`;
        }
      }
      setState({ kind: 'capturing', captured, error });
    };

    element.addEventListener('keydown', handleKeyDown);
    return () => element.removeEventListener('keydown', handleKeyDown);
  }, [state, bindings, commandId, onCapture, reset]);

  const displayText =
    state.kind === 'capturing'
      ? state.captured
        ? shortcutToDisplay(state.captured)
        : 'Press keys...'
      : 'Click to bind';

  const ariaLabel =
    state.kind === 'idle'
      ? 'Click to bind shortcut'
      : state.error
        ? `Capture error: ${state.error}`
        : state.captured
          ? `Captured: ${shortcutToDisplay(state.captured)}, press Enter to save`
          : 'Press keys to capture shortcut';

  return (
    <button
      ref={ref}
      type="button"
      className={`shortcut-capture ${state.kind === 'capturing' ? 'shortcut-capture-active' : ''} ${state.kind === 'capturing' && state.error ? 'shortcut-capture-error' : ''}`}
      data-shortcut-capture="true"
      onFocus={() => setState({ kind: 'capturing', captured: null, error: null })}
      onBlur={reset}
      aria-label={ariaLabel}
    >
      <span className="shortcut-capture-text">{displayText}</span>
      {state.kind === 'capturing' && state.error && (
        <span className="shortcut-capture-error-text" aria-live="polite">
          {state.error}
        </span>
      )}
    </button>
  );
}

import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useRef, type MutableRefObject } from 'react';
import type { DocumentSessionData } from './documentSessionTypes';
import { isTauriRuntime } from './tauriRuntime';

type UseWindowCloseGuardOptions = {
  dirtySessions: DocumentSessionData[];
  anyDirtyRef: MutableRefObject<boolean>;
  pendingNavRef: MutableRefObject<(() => void) | null>;
  setShowUnsavedModal: (open: boolean) => void;
  focusSession: (id: string) => void;
};

export function useWindowCloseGuard({
  dirtySessions,
  anyDirtyRef,
  pendingNavRef,
  setShowUnsavedModal,
  focusSession,
}: UseWindowCloseGuardOptions) {
  const dirtySessionsRef = useRef(dirtySessions);
  dirtySessionsRef.current = dirtySessions;

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const w = getCurrentWindow();
    const unlisten = w.onCloseRequested((event) => {
      if (!anyDirtyRef.current) return;
      event.preventDefault();
      const queue = [...dirtySessionsRef.current];
      const promptNext = () => {
        const next = queue.shift();
        if (!next) {
          pendingNavRef.current = () => w.destroy();
          setShowUnsavedModal(true);
          return;
        }
        focusSession(next.id);
        pendingNavRef.current = () => {
          if (queue.length > 0) promptNext();
          else w.destroy();
        };
        setShowUnsavedModal(true);
      };
      promptNext();
    });
    return () => { void unlisten.then((f) => f()); };
  }, [anyDirtyRef, focusSession, pendingNavRef, setShowUnsavedModal]);
}

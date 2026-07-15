import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, type MutableRefObject } from 'react';
import type { DocumentSessionData } from './documentSessionTypes';
import { isTauriRuntime } from './tauriRuntime';

type UseWindowCloseGuardOptions = {
  sessions: DocumentSessionData[];
  dirtySessions: DocumentSessionData[];
  anyDirtyRef: MutableRefObject<boolean>;
  pendingNavRef: MutableRefObject<(() => void) | null>;
  setShowUnsavedModal: (open: boolean) => void;
  focusSession: (id: string) => void;
};

export function useWindowCloseGuard({
  sessions,
  dirtySessions,
  anyDirtyRef,
  pendingNavRef,
  setShowUnsavedModal,
  focusSession,
}: UseWindowCloseGuardOptions) {
  const dirtySessionsRef = useRef(dirtySessions);
  dirtySessionsRef.current = dirtySessions;
  const sessionsRef = useRef(sessions);
  sessionsRef.current = sessions;

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const w = getCurrentWindow();
    // Discard auto-materialized sibling PDFs (markdown/HTML -> PDF view) so
    // they don't accumulate next to the user's source across runs. Always
    // runs before close (dirty or not); failures are best-effort/silent.
    const discardGeneratedSiblings = async () => {
      const tasks = sessionsRef.current
        .filter((s) => s.generatedPdfPath && s.sourcePath)
        .map((s) =>
          invoke('discard_document_pdf', { generated: s.generatedPdfPath, source: s.sourcePath }).catch(() => {}),
        );
      await Promise.all(tasks);
    };
    const unlisten = w.onCloseRequested((event) => {
      event.preventDefault();
      void (async () => {
        await discardGeneratedSiblings();
        if (!anyDirtyRef.current) {
          w.destroy();
          return;
        }
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
      })();
    });
    return () => { void unlisten.then((f) => f()); };
  }, [anyDirtyRef, focusSession, pendingNavRef, setShowUnsavedModal]);
}

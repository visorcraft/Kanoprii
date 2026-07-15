import { useMemo, useRef, useState } from 'react';
import { useDocumentSessions } from './useDocumentSessions';

export function useAppDocumentState() {
  const sessions = useDocumentSessions();
  const [loading, setLoading] = useState(false);
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);
  const [ocrAvailable, setOcrAvailable] = useState<boolean | null>(null);
  // False during cold-start (session restore + launch-path drain). The
  // markdown/HTML -> PDF generation effect gates on this so its slow async
  // work does not race with a concurrent launch-path PDF open (which could
  // otherwise lose the file/cache slot to the older md-derived render).
  const [coldStartReady, setColdStartReady] = useState(false);

  const anyDirtyRef = useRef(false);
  anyDirtyRef.current = sessions.dirtySessions.length > 0;

  return useMemo(
    () => ({
      ...sessions,
      loading,
      setLoading,
      toast,
      setToast,
      ocrAvailable,
      setOcrAvailable,
      anyDirtyRef,
      coldStartReady,
      setColdStartReady,
    }),
    [sessions, loading, toast, ocrAvailable, coldStartReady],
  );
}

/** Canonical alias for this hook's state shape. */
export type DocumentState = ReturnType<typeof useAppDocumentState>;

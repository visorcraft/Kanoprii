import { useMemo, useRef, useState } from 'react';
import { useDocumentSessions } from './useDocumentSessions';

export function useAppDocumentState() {
  const sessions = useDocumentSessions();
  const [loading, setLoading] = useState(false);
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);
  const [ocrAvailable, setOcrAvailable] = useState<boolean | null>(null);

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
    }),
    [sessions, loading, toast, ocrAvailable],
  );
}

/** Canonical alias for this hook's state shape. */
export type DocumentState = ReturnType<typeof useAppDocumentState>;

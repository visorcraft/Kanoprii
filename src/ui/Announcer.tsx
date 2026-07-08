import type { ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AnnouncerContext, type AnnouncePriority } from './useAnnouncer';

type Announcement = {
  id: number;
  message: string;
  priority: AnnouncePriority;
};

export function AnnouncerProvider({ children }: { children: ReactNode }) {
  const [announcements, setAnnouncements] = useState<Announcement[]>([]);
  const idRef = useRef(0);
  const timeoutsRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  const announce = useCallback(
    (message: string, priority: AnnouncePriority = 'polite') => {
      const id = idRef.current++;
      setAnnouncements((prev) => [...prev, { id, message, priority }]);
      const timeoutMs = Math.max(1000, message.length * 50);
      const timeout = setTimeout(() => {
        setAnnouncements((prev) => prev.filter((a) => a.id !== id));
        timeoutsRef.current.delete(timeout);
      }, timeoutMs);
      timeoutsRef.current.add(timeout);
    },
    []
  );

  useEffect(() => {
    return () => {
      timeoutsRef.current.forEach(clearTimeout);
      // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
      timeoutsRef.current.clear();
    };
  }, []);

  const value = useMemo(() => ({ announce }), [announce]);

  return (
    <AnnouncerContext.Provider value={value}>
      {children}
      <div
        aria-live="polite"
        aria-atomic="false"
        aria-relevant="additions"
        className="sr-only"
      >
        {announcements
          .filter((a) => a.priority === 'polite')
          .map((a) => (
            <span key={a.id}>{a.message}</span>
          ))}
      </div>
      <div
        aria-live="assertive"
        aria-atomic="false"
        aria-relevant="additions"
        className="sr-only"
      >
        {announcements
          .filter((a) => a.priority === 'assertive')
          .map((a) => (
            <span key={a.id}>{a.message}</span>
          ))}
      </div>
    </AnnouncerContext.Provider>
  );
}

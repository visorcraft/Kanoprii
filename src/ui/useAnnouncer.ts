import { createContext, useContext } from 'react';

export type AnnouncePriority = 'polite' | 'assertive';

export type AnnouncerContextValue = {
  announce: (message: string, priority?: AnnouncePriority) => void;
};

export const AnnouncerContext = createContext<AnnouncerContextValue>({
  announce: () => {},
});

export function useAnnouncer() {
  return useContext(AnnouncerContext);
}

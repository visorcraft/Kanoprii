import { useRef } from 'react';
import type { AppKeyboardActions } from './useAppKeyboard';

export function useAppRefs() {
  const filePathRef = useRef('');
  const handleMarkdownViewRef = useRef<() => void | Promise<void>>(async () => {});
  const loadPdfBookmarksRef = useRef<(path: string) => void>(() => {});
  const loadPageSizesRef = useRef<(path: string) => void>(() => {});
  const cancelDrawingRef = useRef<() => void>(() => {});
  const keyboardActionsRef = useRef<AppKeyboardActions>({} as AppKeyboardActions);
  const imgRef = useRef<HTMLImageElement>(null);
  const handleSaveRef = useRef<() => void | Promise<void>>(async () => {});

  return {
    filePathRef,
    handleMarkdownViewRef,
    loadPdfBookmarksRef,
    loadPageSizesRef,
    cancelDrawingRef,
    keyboardActionsRef,
    imgRef,
    handleSaveRef,
  };
}

/** Canonical alias for this hook's state shape. */
export type RefsState = ReturnType<typeof useAppRefs>;

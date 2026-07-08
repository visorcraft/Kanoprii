import { useRef, useState } from 'react';
import type { UnsavedChoice } from '../modals/UnsavedChangesModal';

type UseUnsavedGuardOptions = {
  isDirty: boolean;
  setIsDirty: (dirty: boolean) => void;
  onSave: () => void | Promise<void>;
};

export function useUnsavedGuard({ isDirty, setIsDirty, onSave }: UseUnsavedGuardOptions) {
  const pendingNavRef = useRef<null | (() => void | Promise<void>)>(null);
  const [showUnsavedModal, setShowUnsavedModal] = useState(false);

  const guardUnsaved = (action: () => void | Promise<void>, dirtyOverride?: boolean) => {
    const dirty = dirtyOverride ?? isDirty;
    if (dirty) {
      pendingNavRef.current = action;
      setShowUnsavedModal(true);
    } else {
      void action();
    }
  };

  const resolveUnsaved = async (choice: UnsavedChoice) => {
    if (choice === 'cancel') {
      pendingNavRef.current = null;
      setShowUnsavedModal(false);
      return;
    }
    if (choice === 'save') await onSave();
    else setIsDirty(false);
    setShowUnsavedModal(false);
    const action = pendingNavRef.current;
    pendingNavRef.current = null;
    if (action) await action();
  };

  return {
    showUnsavedModal,
    setShowUnsavedModal,
    pendingNavRef,
    guardUnsaved,
    resolveUnsaved,
  };
}

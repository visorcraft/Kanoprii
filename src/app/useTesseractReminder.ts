import { useCallback, type MutableRefObject } from 'react';
import { dismissTesseractReminder, isTesseractReminderDismissed } from './utils';

type TesseractReminderSource = 'launch' | 'markdown' | null;

type UseTesseractReminderOptions = {
  ocrAvailable: boolean | null;
  tesseractReminderSource: TesseractReminderSource;
  setTesseractReminderSource: (source: TesseractReminderSource) => void;
  tesseractDoNotRemind: boolean;
  setTesseractDoNotRemind: (value: boolean) => void;
  setShowTesseractModal: (open: boolean) => void;
  handleMarkdownViewRef: MutableRefObject<() => void | Promise<void>>;
};

export function useTesseractReminder({
  ocrAvailable,
  tesseractReminderSource,
  setTesseractReminderSource,
  tesseractDoNotRemind,
  setTesseractDoNotRemind,
  setShowTesseractModal,
  handleMarkdownViewRef,
}: UseTesseractReminderOptions) {
  const shouldShowTesseractReminder = useCallback(
    () => ocrAvailable === false && !isTesseractReminderDismissed(),
    [ocrAvailable],
  );

  const closeTesseractReminderModal = useCallback(() => {
    const source = tesseractReminderSource;
    if (tesseractDoNotRemind) dismissTesseractReminder();
    setShowTesseractModal(false);
    setTesseractDoNotRemind(false);
    setTesseractReminderSource(null);
    if (source === 'markdown') {
      void handleMarkdownViewRef.current();
    }
  }, [
    handleMarkdownViewRef,
    setShowTesseractModal,
    setTesseractDoNotRemind,
    setTesseractReminderSource,
    tesseractDoNotRemind,
    tesseractReminderSource,
  ]);

  const showLaunchTesseractReminder = useCallback(() => {
    setTesseractReminderSource('launch');
    setShowTesseractModal(true);
  }, [setShowTesseractModal, setTesseractReminderSource]);

  const openTesseractGuide = useCallback(() => {
    setTesseractReminderSource('launch');
    setShowTesseractModal(true);
  }, [setShowTesseractModal, setTesseractReminderSource]);

  return {
    shouldShowTesseractReminder,
    closeTesseractReminderModal,
    showLaunchTesseractReminder,
    openTesseractGuide,
  };
}

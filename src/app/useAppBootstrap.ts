import { invoke } from '@tauri-apps/api/core';
import { useEffect } from 'react';
import { DEFAULT_TESSERACT_GUIDE } from './constants';
import { isTesseractReminderDismissed } from './utils';
import type { TesseractInstallGuide } from '../modals/TesseractReminderModal';

type AppBootstrapOptions = {
  onNativeDialogs: (enabled: boolean) => void;
  onOcrAvailable: (available: boolean) => void;
  onTesseractInstallGuide: (guide: TesseractInstallGuide) => void;
  onShowTesseractReminder: () => void;
  onUpdaterSupported?: (supported: boolean) => void;
};

/** Load native-dialog capability, OCR availability, and Tesseract guide on startup. */
export function useAppBootstrap({
  onNativeDialogs,
  onOcrAvailable,
  onTesseractInstallGuide,
  onShowTesseractReminder,
  onUpdaterSupported,
}: AppBootstrapOptions) {
  useEffect(() => {
    void (async () => {
      const [dialogs, available, guide, updater] = await Promise.all([
        invoke<boolean>('native_file_dialogs_enabled').catch(() => false),
        invoke<boolean>('ocr_available').catch(() => true),
        invoke<TesseractInstallGuide>('tesseract_install_guide').catch(() => null),
        invoke<boolean>('updater_supported').catch(() => false),
      ]);
      onNativeDialogs(dialogs);
      onOcrAvailable(available);
      onTesseractInstallGuide(guide ?? DEFAULT_TESSERACT_GUIDE);
      onUpdaterSupported?.(updater);
      if (!available && !isTesseractReminderDismissed()) {
        onShowTesseractReminder();
      }
    })();
  }, [onNativeDialogs, onOcrAvailable, onTesseractInstallGuide, onShowTesseractReminder, onUpdaterSupported]);
}

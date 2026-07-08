import type { TesseractInstallGuide } from '../modals/TesseractReminderModal';
import { DEFAULT_TESSERACT_GUIDE } from './constants';
import { useState } from 'react';

export function useHelpChromeState() {
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [showShortcutsHelp, setShowShortcutsHelp] = useState(false);
  const [showLicenses, setShowLicenses] = useState(false);
  const [showCredits, setShowCredits] = useState(false);
  const [showAbout, setShowAbout] = useState(false);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [updaterSupported, setUpdaterSupported] = useState(true);
  const [showTesseractModal, setShowTesseractModal] = useState(false);
  const [tesseractInstallGuide, setTesseractInstallGuide] = useState<TesseractInstallGuide>(DEFAULT_TESSERACT_GUIDE);
  const [tesseractDoNotRemind, setTesseractDoNotRemind] = useState(false);
  const [tesseractReminderSource, setTesseractReminderSource] = useState<'launch' | 'markdown' | null>(null);
  return { showCommandPalette, showShortcutsHelp, showLicenses, showCredits, showAbout, showUpdateModal, updaterSupported, showTesseractModal, tesseractInstallGuide, tesseractDoNotRemind, tesseractReminderSource, setShowCommandPalette, setShowShortcutsHelp, setShowLicenses, setShowCredits, setShowAbout, setShowUpdateModal, setUpdaterSupported, setShowTesseractModal, setTesseractInstallGuide, setTesseractDoNotRemind, setTesseractReminderSource };
}

/** Canonical alias for this hook's state shape. */
export type HelpState = ReturnType<typeof useHelpChromeState>;

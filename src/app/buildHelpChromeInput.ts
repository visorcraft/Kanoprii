import type { HelpState } from './useHelpChromeState';

export type HelpChromeInput = Pick<
  HelpState,
  | 'showCommandPalette'
  | 'showShortcutsHelp'
  | 'showLicenses'
  | 'showCredits'
  | 'showAbout'
  | 'setShowCommandPalette'
  | 'setShowShortcutsHelp'
  | 'setShowLicenses'
  | 'setShowCredits'
  | 'setShowAbout'
>;

export type HelpChromeMenuInput = Pick<
  HelpState,
  'setShowShortcutsHelp' | 'setShowLicenses' | 'setShowCredits' | 'setShowAbout' | 'setShowCommandPalette'
>;

export function buildHelpChromeInput(help: HelpState): HelpChromeInput {
  return {
    showCommandPalette: help.showCommandPalette,
    showShortcutsHelp: help.showShortcutsHelp,
    showLicenses: help.showLicenses,
    showCredits: help.showCredits,
    showAbout: help.showAbout,
    setShowCommandPalette: help.setShowCommandPalette,
    setShowShortcutsHelp: help.setShowShortcutsHelp,
    setShowLicenses: help.setShowLicenses,
    setShowCredits: help.setShowCredits,
    setShowAbout: help.setShowAbout,
  };
}

export function buildHelpChromeMenuInput(help: HelpState): HelpChromeMenuInput {
  return {
    setShowShortcutsHelp: help.setShowShortcutsHelp,
    setShowLicenses: help.setShowLicenses,
    setShowCredits: help.setShowCredits,
    setShowAbout: help.setShowAbout,
    setShowCommandPalette: help.setShowCommandPalette,
  };
}

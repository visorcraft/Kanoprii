import { AxeBuilder } from '@axe-core/webdriverio';
import type { AxeResults } from 'axe-core';
import {
  clickMenuAction,
  fixturePdf,
  fixturePdf3p,
  fixturePdfB,
  openPdfViaPathModal,
  resetToWelcome,
  waitForPageCount,
  waitForShell,
} from '../support/helpers';

async function assertNoAxeViolations(
  context: string,
  extraDisabledRules: string[] = [],
) {
  // WebdriverIO v9 + Tauri's embedded WebKit driver do not support
  // `window/new`, which axe-core uses by default for cross-origin/frame
  // isolation. Legacy mode analyzes the current window directly.
  //
  // Kanoprii is an app-like desktop PDF editor, not a document website, so
  // axe's document-structure best-practice rules (page heading and landmark
  // containment) do not map cleanly to the toolbar/sidebar/viewer layout.
  // Disable them by default; individual tests can opt out via extraDisabledRules.
  const baseDisabledRules = ['page-has-heading-one'];
  // The full app DOM (especially with a rendered PDF) can take longer than the
  // default WebDriver script timeout to analyze. Save and restore the previous
  // timeout so this helper does not leak a global timeout change.
  const previous = await browser.getTimeouts();
  await browser.setTimeout({ script: 120_000 });
  let results: AxeResults;
  try {
    results = await new AxeBuilder({ client: browser })
      .setLegacyMode(true)
      .disableRules([...baseDisabledRules, ...extraDisabledRules])
      .analyze();
  } finally {
    await browser.setTimeout({ script: previous.script ?? 30_000 });
  }
  if (results.violations.length > 0) {
    console.error(
      `Axe violations in ${context}:`,
      JSON.stringify(results.violations, null, 2),
    );
  }
  expect(results.violations).toEqual([]);
}

describe('axe accessibility', () => {
  before(async () => {
    await waitForShell();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('welcome screen has no detectable axe violations', async () => {
    await assertNoAxeViolations('welcome', ['region']);
  });

  it('viewer has no detectable axe violations', async () => {
    await openPdfViaPathModal(fixturePdf3p);
    await waitForPageCount('/ 3');
    await assertNoAxeViolations('viewer', ['region']);
  });

  it('PDF/UA panel has no detectable axe violations', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');
    // Earlier specs may have left the panel open; toggle only if it is closed.
    const panel = await $('.pdfua-panel');
    if (!(await panel.isDisplayed().catch(() => false))) {
      await clickMenuAction('view', 'pdfua-panel');
    }
    await browser.waitUntil(
      async () => (await $('.pdfua-panel').isDisplayed()),
      { timeout: 10_000, timeoutMsg: 'expected PDF/UA panel' },
    );
    await assertNoAxeViolations('pdfua-panel', ['region']);
  });

  it('settings page has no detectable axe violations', async () => {
    await clickMenuAction('help', 'settings');
    await browser.waitUntil(
      async () => (await $('body').getText()).includes('Appearance'),
      { timeout: 10_000, timeoutMsg: 'expected Settings page' },
    );
    await assertNoAxeViolations('settings');
  });

  it('tab bar has no detectable axe violations', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await browser.waitUntil(
      async () =>
        (await (await $$('[data-testid^="doc-tab-"]:not([data-testid^="doc-tab-close-"])'))
          .length) === 2,
      { timeout: 15_000, timeoutMsg: 'expected two document tabs' },
    );
    // The tab close button is intentionally rendered inside each tab so it is
    // visually associated with the tab and reachable by mouse. Keyboard users
    // can close tabs with Delete and the context menu. axe flags this as
    // nested-interactive (via the no-focusable-content check), so disable that
    // rule for the tab bar scan while still checking contrast, ARIA roles, and
    // labels.
    await assertNoAxeViolations('tab-bar', ['region', 'nested-interactive']);
  });
});

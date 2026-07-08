import {
  countDocTabs,
  getZoomInput,
  openPdfViaPathModal,
  resetToWelcome,
  selectTab,
  setZoomPercent,
  waitForPageCount,
  waitForShell,
} from '../support/helpers';

describe('session restore setup', () => {
  before(async () => {
    await waitForShell();
    await resetToWelcome();
  });

  it('opens two documents with independent zoom for restore test', async () => {
    await openPdfViaPathModal(await import('../support/helpers').then((m) => m.fixturePdf));
    await waitForPageCount('/ 1');
    await setZoomPercent('150');
    await openPdfViaPathModal(await import('../support/helpers').then((m) => m.fixturePdfB));
    await waitForPageCount('/ 1');
    await browser.waitUntil(
      async () => (await countDocTabs()) === 2,
      { timeout: 15_000, timeoutMsg: 'expected two document tabs' },
    );
    expect(await getZoomInput()).toBe('100');
    await selectTab('sample');
    expect(await getZoomInput()).toBe('150');
    // Allow the debounced session-state save to flush before WDIO kills the app.
    await browser.pause(500);
  });
});

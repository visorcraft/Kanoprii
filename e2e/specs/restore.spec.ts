import {
  countDocTabs,
  getZoomInput,
  selectTab,
  waitForPageCount,
  waitForPdfOpen,
  waitForShell,
} from '../support/helpers';

describe('session restore', () => {
  before(async () => {
    await waitForShell();
  });

  it('restores previous tabs with zoom after app restart', async () => {
    await browser.waitUntil(
      async () => (await countDocTabs()) === 2,
      { timeout: 15_000, timeoutMsg: 'expected two restored document tabs' },
    );
    await waitForPdfOpen();
    await waitForPageCount('/ 1');
    // Active tab should be the first one (sample) with 150% zoom.
    expect(await getZoomInput()).toBe('150');
    await selectTab('sample-b');
    expect(await getZoomInput()).toBe('100');
  });
});

import {
  cancelUnsavedIfPrompted,
  countDocTabs,
  captureTabWorkingPaths,
  clickMenuAction,
  clickQuickAction,
  closeTab,
  discardUnsavedIfPrompted,
  findText,
  fixturePdf,
  fixturePdfB,
  focusTabByLabel,
  getSaveLabel,
  getZoomInput,
  openPdfViaPathModal,
  pathExists,
  resetToWelcome,
  rotateCurrentPage,
  selectTab,
  setZoomPercent,
  waitForPageCount,
  waitForPdfOpen,
  waitForSearchResults,
  waitForShell,
  waitForWelcome,
} from '../support/helpers';

describe('multi-document tabs', () => {
  before(async () => {
    await waitForShell();
    await resetToWelcome();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('opens two documents with independent zoom', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');
    await setZoomPercent('150');
    await openPdfViaPathModal(fixturePdfB);
    await waitForPageCount('/ 1');
    await browser.waitUntil(
      async () => (await countDocTabs()) === 2,
      { timeout: 15_000, timeoutMsg: 'expected two document tabs' },
    );
    expect(await getZoomInput()).toBe('100');
    await selectTab('sample');
    expect(await getZoomInput()).toBe('150');
    await selectTab('sample-b');
    expect(await getZoomInput()).toBe('100');
  });

  it('keeps edits scoped to the active tab', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await selectTab('sample-b');
    await rotateCurrentPage();
    await browser.waitUntil(async () => (await getSaveLabel()) === 'Save •', {
      timeout: 15_000,
      timeoutMsg: 'expected dirty tab B',
    });
    await selectTab('sample');
    expect(await getSaveLabel()).toBe('Save');
    await selectTab('sample-b');
    expect(await getSaveLabel()).toBe('Save •');
  });

  it('isolates undo history per tab', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await selectTab('sample');
    await rotateCurrentPage();
    await selectTab('sample-b');
    await rotateCurrentPage();
    await selectTab('sample');
    await browser.waitUntil(async () => (await $('[data-testid="undo-btn"]').isEnabled()), {
      timeout: 10_000,
      timeoutMsg: 'expected undo enabled in tab A',
    });
    await clickQuickAction('undo-btn');
    await browser.waitUntil(async () => (await getSaveLabel()) === 'Save', {
      timeout: 15_000,
      timeoutMsg: 'expected tab A clean after undo',
    });
    await selectTab('sample-b');
    expect(await getSaveLabel()).toBe('Save •');
  });

  it('restores per-tab find state', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await selectTab('sample');
    await findText('Hello');
    await waitForSearchResults(1);
    await browser.keys('Escape');
    await selectTab('sample-b');
    await findText('ZZZNOTFOUND');
    await browser.waitUntil(
      async () => !(await $('[data-testid="search-results"]').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected no matches in tab B' },
    );
    await browser.keys('Escape');
    await selectTab('sample');
    await findText('Hello');
    await waitForSearchResults(1);
  });

  it('prompts before closing a dirty tab', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await selectTab('sample-b');
    await rotateCurrentPage();
    await closeTab('sample-b');
    await (await $('[data-testid="unsaved-cancel"]')).waitForDisplayed({ timeout: 10_000 });
    await cancelUnsavedIfPrompted();
    expect(await countDocTabs()).toBe(2);
    await closeTab('sample-b');
    await discardUnsavedIfPrompted();
    await browser.waitUntil(
      async () => (await countDocTabs()) === 0,
      { timeout: 10_000, timeoutMsg: 'expected tab bar hidden after one tab remains' },
    );
  });

  it('does not duplicate an already-open document', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');
    expect(await countDocTabs()).toBe(0);
  });

  it('navigates tabs with arrow keys and closes with Delete', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await browser.waitUntil(
      async () => (await countDocTabs()) === 2,
      { timeout: 15_000, timeoutMsg: 'expected two document tabs' },
    );

    await focusTabByLabel('sample-b');
    const focusedTabDataTestid = () =>
      browser.execute(() =>
        document.activeElement?.closest('.tab-item')?.getAttribute('data-testid'),
      );
    await browser.waitUntil(
      async () => (await focusedTabDataTestid()) === 'doc-tab-sample-b',
      { timeout: 5_000, timeoutMsg: 'expected focus on second tab' },
    );

    await browser.keys('ArrowLeft');
    expect(await focusedTabDataTestid()).toBe('doc-tab-sample');

    await browser.keys('ArrowRight');
    expect(await focusedTabDataTestid()).toBe('doc-tab-sample-b');

    // The app renders the tab bar only when two or more tabs are open, so the
    // DOM count becomes 0 after closing one of two tabs. Dispatch Delete directly
    // because WebKit GTK's WebDriver does not reliably send it to non-inputs.
    await browser.execute(() => {
      const active = document.activeElement as HTMLElement | null;
      if (!active) throw new Error('no focused tab');
      active.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Delete', bubbles: true, cancelable: true }),
      );
    });
    await browser.waitUntil(
      async () => (await countDocTabs()) === 0,
      { timeout: 10_000, timeoutMsg: 'expected tab bar to hide after Delete' },
    );
    await (await $('[data-testid="save-pdf"]')).waitForDisplayed({ timeout: 10_000 });
  });

  it('opens the tab context menu from the keyboard and closes a tab', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    await browser.waitUntil(
      async () => (await countDocTabs()) === 2,
      { timeout: 15_000, timeoutMsg: 'expected two document tabs' },
    );

    await focusTabByLabel('sample');
    // Dispatch Shift+F10 directly; WebKit GTK's WebDriver does not reliably
    // send function-key combinations to non-input elements.
    await browser.execute(() => {
      const active = document.activeElement as HTMLElement | null;
      if (!active) throw new Error('no focused tab');
      active.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true, cancelable: true }),
      );
    });
    const menu = await $('.tab-context-menu');
    await menu.waitForDisplayed({ timeout: 10_000 });

    // Ensure the menu root has focus, then navigate with the keyboard.
    await browser.execute(() => {
      const menuRoot = document.querySelector('.tab-context-menu') as HTMLElement | null;
      if (!menuRoot) throw new Error('context menu missing');
      menuRoot.focus();
    });
    await browser.keys('ArrowDown');
    await browser.waitUntil(
      async () => (await (await $$('.tcm-item.highlighted')).length) === 1,
      { timeout: 5_000, timeoutMsg: 'expected a highlighted context-menu item' },
    );
    await browser.keys('Enter');
    await browser.waitUntil(
      async () => (await countDocTabs()) === 0,
      { timeout: 10_000, timeoutMsg: 'expected tab bar to hide after Close tab' },
    );
    await (await $('[data-testid="save-pdf"]')).waitForDisplayed({ timeout: 10_000 });
  });

  it('cleans up working copies after all tabs close', async () => {
    await openPdfViaPathModal(fixturePdf);
    await openPdfViaPathModal(fixturePdfB);
    const paths = await captureTabWorkingPaths();
    expect(paths.length).toBe(2);
    await closeTab('sample-b');
    await discardUnsavedIfPrompted();
    if ((await countDocTabs()) > 0) {
      await closeTab('sample');
    } else {
      await clickMenuAction('file', 'close');
    }
    await discardUnsavedIfPrompted();
    await waitForWelcome();
    for (const p of paths) {
      expect(await pathExists(p)).toBe(false);
    }
  });
});

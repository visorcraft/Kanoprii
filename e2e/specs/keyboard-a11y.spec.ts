import {
  clickMenuAction,
  fixturePdf,
  fixturePdf3p,
  openPdfViaPathModal,
  resetToWelcome,
  waitForPageCount,
  waitForShell,
} from '../support/helpers';

async function getPageInputValue(): Promise<string> {
  return browser.execute(() => {
    const input = document.querySelector('input[aria-label="Current page"]') as HTMLInputElement | null;
    return input?.value ?? '';
  });
}

describe('keyboard accessibility', () => {
  before(async () => {
    await waitForShell();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('navigates pages with keyboard', async () => {
    await openPdfViaPathModal(fixturePdf3p);
    await waitForPageCount('/ 3');

    // Ensure no text input is focused so the global shortcut fires.
    await browser.execute(() => {
      const active = document.activeElement as HTMLElement | null;
      active?.blur();
    });

    await browser.keys('ArrowRight');
    await browser.pause(250);
    await expect(await getPageInputValue()).toBe('2');

    await browser.keys('ArrowRight');
    await browser.pause(250);
    await expect(await getPageInputValue()).toBe('3');

    await browser.keys('ArrowLeft');
    await browser.pause(250);
    await expect(await getPageInputValue()).toBe('2');

    await browser.keys('ArrowLeft');
    await browser.pause(250);
    await expect(await getPageInputValue()).toBe('1');
  });

  it('focuses and activates thumbnails with keyboard', async () => {
    await openPdfViaPathModal(fixturePdf3p);
    await waitForPageCount('/ 3');

    await browser.execute(() => {
      const list = document.querySelector('.thumbnail-list') as HTMLElement | null;
      const thumb = list?.querySelector<HTMLElement>('.thumbnail');
      thumb?.focus();
    });

    await browser.keys('ArrowDown');
    await browser.pause(250);
    const label = await browser.execute(() => {
      const active = document.activeElement as HTMLElement | null;
      return active?.getAttribute('aria-label') ?? '';
    });
    expect(label).toBe('Page 2');

    await browser.keys('Enter');
    await browser.pause(250);
    await expect(await getPageInputValue()).toBe('2');
  });

  it('opens PDF/UA Check panel from View menu', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');

    await clickMenuAction('view', 'pdfua-panel');

    await browser.waitUntil(
      async () => (await $('.pdfua-panel').isDisplayed()),
      { timeout: 10_000, timeoutMsg: 'expected PDF/UA panel' },
    );

    const panelText = await $('.pdfua-panel').getText();
    expect(panelText).toContain('PDF/UA Check');
  });
});

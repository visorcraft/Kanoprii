import {
  waitForShell,
  clickMenuAction,
  openPdfViaPathModal,
  fixturePdf,
} from '../support/helpers';

describe('print dialog', () => {
  beforeEach(async () => {
    await browser.execute(() => window.location.reload());
    await waitForShell();
  });

  it('opens the print dialog from the File menu', async () => {
    await openPdfViaPathModal(fixturePdf);
    await clickMenuAction('file', 'print');
    const dialog = await $('[data-testid="print-dialog"]');
    await dialog.waitForDisplayed({ timeout: 10_000 });
    await browser.waitUntil(
      async () => browser.execute(() => document.body.classList.contains('kanoprii-modal-open')),
      { timeout: 5_000, timeoutMsg: 'expected modal-open scrollbar guard' },
    );
    expect(await dialog.getText()).toContain('Print');
    await browser.keys('Escape');
    await browser.waitUntil(
      async () => browser.execute(() => !document.body.classList.contains('kanoprii-modal-open')),
      { timeout: 5_000, timeoutMsg: 'expected modal-open scrollbar guard to clear' },
    );
  });
});

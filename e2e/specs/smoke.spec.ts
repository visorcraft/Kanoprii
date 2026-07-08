import {
  fixturePdf,
  openFileMenu,
  openPdfViaPathModal,
  resetToWelcome,
  rotateCurrentPage,
  waitForPageCount,
  waitForShell,
} from '../support/helpers';

describe('Kanoprii shell', () => {
  before(async () => {
    await waitForShell();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('shows Open PDF under the File menu on launch', async () => {
    await openFileMenu();
    const openItem = await $('[data-testid="open-pdf"]');
    await expect(openItem).toBeDisplayed();
    await expect(openItem).toHaveText(expect.stringContaining('Open PDF'));
  });

  it('opens a PDF via the path modal and shows page controls', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');
  });

  it('marks the document dirty after rotate', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');

    const saveBtn = await $('[data-testid="save-pdf"]');
    await expect(saveBtn).toHaveText('Save');

    await rotateCurrentPage();

    await browser.waitUntil(
      async () => (await saveBtn.getText()) === 'Save •',
      { timeout: 15_000, timeoutMsg: 'expected dirty save label after rotate' },
    );
  });

});

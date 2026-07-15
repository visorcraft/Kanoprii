import { browser } from '@wdio/globals';
import {
  fixturePdf,
  openPdfViaPathModal,
  resetToWelcome,
  waitForPageRendered,
  waitForPdfOpen,
  waitForShell,
} from '../support/helpers';

describe('PDF Edit Mode', () => {
  before(async () => {
    await waitForShell();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('enters edit text mode, edits an existing text line, and applies', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();

    // Enter "Edit text" mode via the Annotate menu.
    const annotateTrigger = await $('[data-testid="menu-annotate"]');
    await annotateTrigger.click();
    const editAction = await $('[data-testid="edit-text"]');
    await editAction.waitForDisplayed({ timeout: 5_000 });
    await editAction.click();

    // Wait for the page container to show the edit cursor.
    const pageContainer = await $('[data-testid="page-container"]');
    await browser.waitUntil(
      async () => (await pageContainer.getAttribute('class')).includes('text-edit-cursor'),
      { timeout: 5_000, timeoutMsg: 'expected page container to enter text-edit-cursor mode' },
    );

    // Click on an existing text line. The text layer is pointer-events:none in
    // edit mode, so dispatch the click on the page container at the span's
    // screen location so the hit-test finds the line.
    await browser.execute(() => {
      const container = document.querySelector('[data-testid="page-container"]') as HTMLElement | null;
      const span = Array.from(document.querySelectorAll('.text-layer span')).find((el) =>
        el.textContent?.includes('Hello'),
      ) as HTMLElement | null;
      const img = document.querySelector('.page-image') as HTMLElement | null;
      if (!container || !span || !img) throw new Error('missing page container, text span, or page image');

      const rect = span.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;

      container.dispatchEvent(
        new MouseEvent('click', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
    });

    // The overlay should appear.
    const input = await $('.text-edit-overlay-input');
    await input.waitForDisplayed({ timeout: 10_000 });

    // Replace the text and apply (the overlay commits on Enter).
    await input.setValue('Hello edit');
    await browser.keys('Enter');

    await browser.waitUntil(
      async () => !(await $('.text-edit-overlay-input').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected text edit overlay to disappear after apply' },
    );
  });
});

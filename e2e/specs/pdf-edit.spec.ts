import { browser, expect } from '@wdio/globals';
import {
  clickMenuAction,
  fixturePdf,
  fixturePdfParagraph,
  openPdfViaPathModal,
  resetToWelcome,
  waitForPageRendered,
  waitForPdfOpen,
  waitForShell,
} from '../support/helpers';

async function enterPdfEditMode() {
  const toolbarBtn = await $('[aria-label="Edit mode"]');
  if (await toolbarBtn.isDisplayed().catch(() => false)) {
    await toolbarBtn.click();
    return;
  }
  await clickMenuAction('annotate', 'pdf-edit');
}

describe('PDF Edit Mode', () => {
  before(async () => {
    await waitForShell();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('enters edit mode, edits an existing text line, and applies', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();

    await enterPdfEditMode();
    const editBtn = await $('[aria-label="Edit mode"]');
    await editBtn.waitForDisplayed({ timeout: 10_000 });
    await expect(editBtn).toHaveAttribute('aria-pressed', 'true');

    const { cx, cy } = await browser.execute(() => {
      const span = Array.from(document.querySelectorAll('.text-layer span')).find((el) =>
        el.textContent?.includes('Hello'),
      ) as HTMLElement | null;
      if (!span) throw new Error('missing text layer span containing "Hello"');
      const rect = span.getBoundingClientRect();
      return { cx: Math.round(rect.left + rect.width / 2), cy: Math.round(rect.top + rect.height / 2) };
    });

    await browser
      .action('pointer')
      .move({ x: cx, y: cy })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });

    await textarea.setValue('Hello edit');
    const applyBtn = await $('.edit-toolbar-apply');
    await applyBtn.click();

    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected rich-text edit overlay to disappear after apply' },
    );
  });

  it('enters edit mode, edits a multi-line paragraph, and applies', async () => {
    await openPdfViaPathModal(fixturePdfParagraph);
    await waitForPdfOpen();
    await waitForPageRendered();

    await enterPdfEditMode();
    const editBtn = await $('[aria-label="Edit mode"]');
    await editBtn.waitForDisplayed({ timeout: 10_000 });

    const { cx, cy } = await browser.execute(() => {
      const span = Array.from(document.querySelectorAll('.text-layer span')).find((el) =>
        el.textContent?.includes('First line'),
      ) as HTMLElement | null;
      if (!span) throw new Error('missing text layer span containing "First line"');
      const rect = span.getBoundingClientRect();
      return { cx: Math.round(rect.left + rect.width / 2), cy: Math.round(rect.top + rect.height / 2) };
    });

    await browser
      .action('pointer')
      .move({ x: cx, y: cy })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();

    const selectionOverlay = await $('.paragraph-selection-overlay');
    await selectionOverlay.waitForDisplayed({ timeout: 10_000 });

    await selectionOverlay.click();
    await browser.keys('Enter');

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });

    // Verify the textarea contains the paragraph text (both lines joined by newline).
    const value = await textarea.getValue();
    expect(value).toContain('First line of paragraph');
    expect(value).toContain('Second line of paragraph');

    await textarea.setValue('Edited paragraph text');

    const applyBtn = await $('.edit-toolbar-apply');
    await applyBtn.click();

    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected rich-text edit overlay to disappear after paragraph apply' },
    );
  });
});

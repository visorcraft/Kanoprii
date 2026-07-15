import { browser, expect } from '@wdio/globals';
import {
  clickMenuAction,
  fixturePdf,
  fixturePdfImage,
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

async function getCenterOfTextSpan(text: string): Promise<{ cx: number; cy: number }> {
  return browser.execute((t) => {
    const span = Array.from(document.querySelectorAll('.text-layer span')).find((el) =>
      el.textContent?.includes(t),
    ) as HTMLElement | null;
    if (!span) throw new Error(`missing text layer span containing "${t}"`);
    const rect = span.getBoundingClientRect();
    return { cx: Math.round(rect.left + rect.width / 2), cy: Math.round(rect.top + rect.height / 2) };
  }, text);
}

describe('PDF Edit Mode', () => {
  before(async () => {
    await waitForShell();
  });

  beforeEach(async () => {
    await resetToWelcome();
  });

  it('enters edit mode, resizes the rich-text edit box, and applies', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();

    await enterPdfEditMode();
    const editBtn = await $('[aria-label="Edit mode"]');
    await editBtn.waitForDisplayed({ timeout: 10_000 });

    const { cx, cy } = await getCenterOfTextSpan('Hello');

    await browser
      .action('pointer')
      .move({ x: cx, y: cy })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });

    const handle = await $('.rich-text-resize-handle-br');
    await handle.waitForDisplayed({ timeout: 10_000 });

    const before = await browser.execute(() => {
      const el = document.querySelector('.rich-text-edit-overlay') as HTMLElement | null;
      if (!el) throw new Error('missing overlay');
      const rect = el.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    });

    const { hx, hy } = await browser.execute(() => {
      const el = document.querySelector('.rich-text-resize-handle-br') as HTMLElement | null;
      if (!el) throw new Error('missing resize handle');
      const rect = el.getBoundingClientRect();
      return { hx: Math.round(rect.left + rect.width / 2), hy: Math.round(rect.top + rect.height / 2) };
    });

    await browser
      .action('pointer')
      .move({ x: hx, y: hy })
      .down({ button: 0 })
      .move({ x: hx + 60, y: hy + 40 })
      .up({ button: 0 })
      .perform();

    const after = await browser.execute(() => {
      const el = document.querySelector('.rich-text-edit-overlay') as HTMLElement | null;
      if (!el) throw new Error('missing overlay');
      const rect = el.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    });

    expect(after.width).toBeGreaterThan(before.width);
    expect(after.height).toBeGreaterThan(before.height);

    const applyBtn = await $('.edit-toolbar-apply');
    await applyBtn.click();

    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected rich-text edit overlay to disappear after resize apply' },
    );
  });

  it('enters edit mode, edits an existing text line, and applies', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();

    await enterPdfEditMode();
    const editBtn = await $('[aria-label="Edit mode"]');
    await editBtn.waitForDisplayed({ timeout: 10_000 });
    await expect(editBtn).toHaveAttribute('aria-pressed', 'true');

    const { cx, cy } = await getCenterOfTextSpan('Hello');

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

    const { cx, cy } = await getCenterOfTextSpan('First line');

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

  it('enters edit mode, selects an image, rotates it, and applies', async () => {
    await openPdfViaPathModal(fixturePdfImage);
    await waitForPdfOpen();
    await waitForPageRendered();

    await enterPdfEditMode();
    const editBtn = await $('[aria-label="Edit mode"]');
    await editBtn.waitForDisplayed({ timeout: 10_000 });

    // Click near the center of the page to select the full-page image.
    const { cx, cy } = await browser.execute(() => {
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!img) throw new Error('missing page image');
      const rect = img.getBoundingClientRect();
      return { cx: Math.round(rect.left + rect.width / 2), cy: Math.round(rect.top + rect.height / 2) };
    });

    await browser
      .action('pointer')
      .move({ x: cx, y: cy })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();

    const overlay = await $('.image-selection-overlay');
    await overlay.waitForDisplayed({ timeout: 10_000 });

    const rotator = await $('.image-selection-handle-rotate');
    await rotator.waitForDisplayed({ timeout: 10_000 });

    // Drag the rotation handle from the top-center of the overlay to the right.
    const { rx, ry } = await browser.execute(() => {
      const el = document.querySelector('.image-selection-overlay') as HTMLElement | null;
      if (!el) throw new Error('missing image selection overlay');
      const rect = el.getBoundingClientRect();
      return { rx: Math.round(rect.left + rect.width / 2), ry: Math.round(rect.top) - 12 };
    });

    await browser
      .action('pointer')
      .move({ x: rx, y: ry })
      .down({ button: 0 })
      .move({ x: rx + 120, y: ry })
      .up({ button: 0 })
      .perform();

    // Apply the image rotation with Enter (the overlay keeps focus during drag).
    await browser.keys('Enter');

    await browser.waitUntil(
      async () => !(await $('.image-selection-overlay').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected image selection overlay to disappear after rotate apply' },
    );

    const errorToast = await $('[role="alert"]');
    expect(await errorToast.isDisplayed().catch(() => false)).toBe(false);
  });
});

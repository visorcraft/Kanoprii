import { browser, expect } from '@wdio/globals';
import {
  clickMenuAction,
  fixturePdf,
  fixturePdfImage,
  fixturePdfParagraph,
  fixturePng,
  openPdfViaPathModal,
  resetToWelcome,
  rotateCurrentPage,
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
  await clickMenuAction('edit', 'pdf-edit');
}

// The edit tools live in the Edit ribbon tab; select it before using them directly.
async function selectEditRibbonTab() {
  const tab = await $('[data-testid="menu-edit"]');
  await tab.waitForDisplayed({ timeout: 10_000 });
  await tab.click();
}

// Undo lives on the Home ribbon tab; select it before asserting undo state.
async function selectHomeRibbonTab() {
  const tab = await $('[data-testid="menu-home"]');
  await tab.waitForDisplayed({ timeout: 10_000 });
  await tab.click();
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

  it('shows a dedicated edit toolbar and keeps editing commands out of Annotate', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await selectEditRibbonTab();

    const toolbar = await $('[data-testid="pdf-edit-toolbar"]');
    await toolbar.waitForDisplayed({ timeout: 5_000 });
    expect(await toolbar.getAttribute('class')).toContain('edit-ribbon-tab');
    expect(await toolbar.$$('.pdf-edit-tool-group')).toHaveLength(4);
    expect(await toolbar.$$('.pdf-edit-tool-icon')).toHaveLength(6);
    for (const label of ['Edit Text', 'Add Text', 'Add Image', 'Edit Objects', 'Edit Vector', 'Manage']) {
      expect(await toolbar.$(`button=${label}`).isExisting()).toBe(true);
    }

    await $('[data-testid="menu-annotate"]').click();
    expect(await $('[data-testid="pdf-edit"]').isDisplayed().catch(() => false)).toBe(false);
    expect(await $('[data-testid="edit-text"]').isDisplayed().catch(() => false)).toBe(false);
  });

  it('keeps Add Text active, readable, roomy, and isolated from app shortcuts', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    const addText = await $('button=Add Text');
    const editObjects = await $('[aria-label="Edit mode"]');
    await addText.click();
    await expect(addText).toHaveAttribute('aria-pressed', 'true');
    await expect(editObjects).toHaveAttribute('aria-pressed', 'false');

    const { cx, cy } = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      return { cx: Math.round(page.left + page.width * 0.45), cy: Math.round(page.top + page.height * 0.35) };
    });
    await browser.action('pointer').move({ x: cx, y: cy }).down({ button: 0 }).up({ button: 0 }).perform();

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });
    const typed = 'test Approved text stays visible\nwhile typing and shortcut letters never steal focus';
    await browser.keys('test Approved text stays visible while typing and shortcut letters never steal focus');
    await browser.execute((value) => {
      const input = document.querySelector('.rich-text-edit-textarea') as HTMLTextAreaElement;
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')!.set!.call(input, value);
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }, typed);
    await expect(textarea).toHaveValue(typed);
    await expect(addText).toHaveAttribute('aria-pressed', 'true');
    await expect(editObjects).toHaveAttribute('aria-pressed', 'false');
    await browser.waitUntil(
      () => browser.execute(() => {
        const input = document.querySelector('.rich-text-edit-textarea') as HTMLTextAreaElement;
        return input.getBoundingClientRect().height > 44 && input.scrollHeight <= input.clientHeight + 1;
      }),
      { timeout: 5_000, timeoutMsg: 'expected the Add Text box to grow for wrapped text' },
    );

    const layout = await browser.execute(() => {
      const input = document.querySelector('.rich-text-edit-textarea') as HTMLTextAreaElement;
      const rect = input.getBoundingClientRect();
      return {
        width: rect.width,
        height: rect.height,
        scrollHeight: input.scrollHeight,
        clientHeight: input.clientHeight,
        color: getComputedStyle(input).color,
        scale: Number(getComputedStyle(document.querySelector('.page-overlay-scale')!).transform.match(/^matrix\(([^,]+)/)?.[1] ?? 1),
        stampToolbar: Boolean(document.querySelector('.stamp-toolbar')),
      };
    });
    expect(layout.width / layout.scale).toBeGreaterThanOrEqual(300);
    expect(layout.height).toBeGreaterThan(44);
    expect(layout.scrollHeight).toBeLessThanOrEqual(layout.clientHeight + 1);
    expect(layout.color).not.toBe('rgba(0, 0, 0, 0)');
    expect(layout.stampToolbar).toBe(false);

    await $('.edit-toolbar-cancel').click();
  });

  it('keeps Edit Objects selection-only on empty page space', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    const editObjects = await $('[aria-label="Edit mode"]');
    await editObjects.click();
    await expect(editObjects).toHaveAttribute('aria-pressed', 'true');
    await expect($('.ribbon-hint')).toHaveText('Click text or an image to select it.');
    await expect($('.page-container')).toHaveElementClass('object-edit-cursor');
    await expect($('.text-layer')).toHaveElementClass('edit-targets');

    const hover = await browser.execute(() => {
      const span = Array.from(document.querySelectorAll('.text-layer span')).find((el) => el.textContent === 'Hello')!;
      const style = getComputedStyle(span);
      return {
        background: style.backgroundColor,
        shadow: style.boxShadow,
      };
    });
    expect(hover.background).not.toBe('rgba(0, 0, 0, 0)');
    expect(hover.shadow).not.toBe('none');

    const blank = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      return { x: Math.round(page.left + page.width * 0.75), y: Math.round(page.top + page.height * 0.25) };
    });
    await browser.action('pointer').move(blank).down({ button: 0 }).up({ button: 0 }).perform();
    await browser.pause(250);
    expect(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)).toBe(false);
    await expect(editObjects).toHaveAttribute('aria-pressed', 'true');

    await $('button=Add Text').click();
    await expect($('.ribbon-hint')).toHaveText('Click the page to place text.');
    await expect($('.page-container')).toHaveElementClass('text-edit-cursor');
    await expect($('.text-layer')).not.toHaveElementClass('edit-targets');
    const textPoint = await getCenterOfTextSpan('Hello');
    await browser.action('pointer').move({ x: textPoint.cx, y: textPoint.cy }).down({ button: 0 }).up({ button: 0 }).perform();
    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });
    await expect(textarea).toHaveValue('');
  });

  it('matches Add Text preview proportions on a rotated page', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await rotateCurrentPage();
    await waitForPageRendered();
    await selectEditRibbonTab();

    await $('button=Add Text').click();
    const { cx, cy } = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      return { cx: Math.round(page.left + page.width * 0.45), cy: Math.round(page.top + page.height * 0.35) };
    });
    await browser.action('pointer').move({ x: cx, y: cy }).down({ button: 0 }).up({ button: 0 }).perform();

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });
    await browser.keys('Rotated preview');
    const scaleX = await browser.execute(() => {
      const transform = getComputedStyle(document.querySelector('.rich-text-edit-textarea')!).transform;
      return Number(transform.match(/^matrix\(([^,]+)/)?.[1] ?? 1);
    });
    expect(scaleX).toBeLessThan(0.6);
    expect(scaleX).toBeGreaterThan(0.4);
  });

  it('commits Add Text at the preview baseline', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    await $('button=Add Text').click();
    const { cx, cy } = await browser.execute(() => {
      const hello = Array.from(document.querySelectorAll('.text-layer span')).find((el) => el.textContent === 'Hello')!.getBoundingClientRect();
      return { cx: Math.round(hello.left), cy: Math.round(hello.bottom + 24) };
    });
    await browser.action('pointer').move({ x: cx, y: cy }).down({ button: 0 }).up({ button: 0 }).perform();

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });
    await browser.keys('Position probe');
    const preview = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      const input = document.querySelector('.rich-text-edit-textarea')!.getBoundingClientRect();
      const fontSize = Number.parseFloat(getComputedStyle(document.querySelector('.rich-text-edit-textarea')!).fontSize);
      const transform = getComputedStyle(document.querySelector('.page-overlay-scale')!).transform;
      const scale = Number(transform.match(/^matrix\(([^,]+)/)?.[1] ?? 1);
      return { x: input.left - page.left, baseline: input.top - page.top + fontSize * scale, scale, expectedScale: page.width / 800 };
    });
    await $('.edit-toolbar-apply').click();
    await browser.waitUntil(
      () => browser.execute(() => Array.from(document.querySelectorAll('.text-layer span')).some((el) => el.textContent === 'Position probe')),
      { timeout: 15_000, timeoutMsg: 'expected committed Position probe text layer run' },
    );
    await $('.page-image').waitForDisplayed({ timeout: 15_000 });
    const committed = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      const span = Array.from(document.querySelectorAll('.text-layer span')).find((el) => el.textContent === 'Position probe')!.getBoundingClientRect();
      return { x: span.left - page.left, baseline: span.bottom - page.top };
    });
    expect(Math.abs(preview.scale - preview.expectedScale)).toBeLessThan(0.01);
    expect(Math.abs(committed.x - preview.x)).toBeLessThan(3);
    expect(Math.abs(committed.baseline - preview.baseline)).toBeLessThan(5);
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

    const layout = await browser.execute(() => {
      const overlay = document.querySelector('.rich-text-edit-overlay')!.getBoundingClientRect();
      const toolbar = document.querySelector('.edit-toolbar')!.getBoundingClientRect();
      const input = document.querySelector('.rich-text-edit-textarea')!.getBoundingClientRect();
      return {
        overlay: { top: overlay.top, bottom: overlay.bottom, width: overlay.width, height: overlay.height },
        toolbar: { top: toolbar.top, bottom: toolbar.bottom, width: toolbar.width },
        input: { width: input.width, height: input.height },
      };
    });
    expect(layout.toolbar.width).toBeGreaterThan(layout.overlay.width);
    expect(layout.toolbar.bottom <= layout.overlay.top || layout.toolbar.top >= layout.overlay.bottom).toBe(true);
    expect(Math.round(layout.input.width)).toBe(Math.round(layout.overlay.width));
    expect(Math.round(layout.input.height)).toBe(Math.round(layout.overlay.height));

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

  it('enters edit mode, drags the rich-text edit box by the move handle, and applies', async () => {
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

    const moveHandle = await $('.rich-text-move-handle');
    await moveHandle.waitForDisplayed({ timeout: 10_000 });

    const before = await browser.execute(() => {
      const el = document.querySelector('.rich-text-edit-overlay') as HTMLElement | null;
      if (!el) throw new Error('missing overlay');
      const rect = el.getBoundingClientRect();
      return { x: rect.left, y: rect.top };
    });

    const { hx, hy } = await browser.execute(() => {
      const el = document.querySelector('.rich-text-move-handle') as HTMLElement | null;
      if (!el) throw new Error('missing move handle');
      const rect = el.getBoundingClientRect();
      return { hx: Math.round(rect.left + rect.width / 2), hy: Math.round(rect.top + rect.height / 2) };
    });

    await browser
      .action('pointer')
      .move({ x: hx, y: hy })
      .down({ button: 0 })
      .move({ x: hx + 80, y: hy + 60 })
      .up({ button: 0 })
      .perform();

    const after = await browser.execute(() => {
      const el = document.querySelector('.rich-text-edit-overlay') as HTMLElement | null;
      if (!el) throw new Error('missing overlay');
      const rect = el.getBoundingClientRect();
      return { x: rect.left, y: rect.top };
    });

    expect(Math.round(after.x - before.x)).toBeGreaterThanOrEqual(50);
    expect(Math.round(after.y - before.y)).toBeGreaterThanOrEqual(30);

    const applyBtn = await $('.edit-toolbar-apply');
    await applyBtn.click();

    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected rich-text edit overlay to disappear after move apply' },
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

    await selectHomeRibbonTab();
    const undo = await $('[data-testid="undo-btn"]');
    await undo.waitForEnabled({ timeout: 10_000 });
    await browser.keys(['\uE009', 'z']);
    await browser.waitUntil(() => undo.isEnabled().then((enabled) => !enabled), {
      timeout: 15_000,
      timeoutMsg: 'expected Ctrl+Z to work while Edit Objects mode remains active',
    });
  });

  it('closes an unchanged Edit Text field without creating an edit', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    await $('button=Edit Text').click();
    const { cx, cy } = await getCenterOfTextSpan('Hello');
    await browser.action('pointer').move({ x: cx, y: cy }).down({ button: 0 }).up({ button: 0 }).perform();
    await $('.rich-text-edit-textarea').waitForDisplayed({ timeout: 10_000 });
    await $('[aria-label="Text editing toolbar"]').waitForDisplayed({ timeout: 10_000 });

    const blank = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      return { x: Math.round(page.left + page.width * 0.75), y: Math.round(page.top + page.height * 0.25) };
    });
    await browser.action('pointer').move(blank).down({ button: 0 }).up({ button: 0 }).perform();
    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 5_000, timeoutMsg: 'expected unchanged Edit Text field to close' },
    );
    await selectHomeRibbonTab();
    await expect($('[data-testid="undo-btn"]')).toBeDisabled();

    await browser.action('pointer').move(blank).down({ button: 0 }).up({ button: 0 }).perform();
    await browser.pause(250);
    expect(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)).toBe(false);
  });

  it('uses rich formatting when Edit Text changes existing text', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    const editText = await $('button=Edit Text');
    await editText.click();
    await expect(editText).toHaveAttribute('aria-pressed', 'true');
    const { cx, cy } = await getCenterOfTextSpan('Hello');
    await browser.action('pointer').move({ x: cx, y: cy }).down({ button: 0 }).up({ button: 0 }).perform();

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });
    await textarea.setValue('Hello formatted');
    await $('[aria-label="Bold"]').click();
    await expect($('[aria-label="Bold"]')).toHaveAttribute('aria-pressed', 'true');
    await $('.edit-toolbar-apply').click();

    await browser.waitUntil(
      () => browser.execute(() => Array.from(document.querySelectorAll('.text-layer span')).some((el) => el.textContent === 'Hello formatted')),
      { timeout: 15_000, timeoutMsg: 'expected rich Edit Text replacement to commit' },
    );
  });

  it('enters edit mode, deletes an existing text line, and the overlay disappears', async () => {
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

    const deleteBtn = await $('.edit-toolbar-delete');
    await deleteBtn.waitForDisplayed({ timeout: 10_000 });
    await deleteBtn.click();

    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected rich-text edit overlay to disappear after delete' },
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
    await $('[aria-label="Paragraph editing toolbar"]').waitForDisplayed({ timeout: 10_000 });
    expect(await $('[aria-label="Paragraph editing toolbar"]').$$('.pdf-edit-context-icon')).toHaveLength(3);
    await $('button=Edit Text').waitForDisplayed({ timeout: 10_000 });
    await selectionOverlay.click();
    await browser.keys('Enter');

    const textarea = await $('.rich-text-edit-textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });

    await $('.edit-toolbar-apply').click();
    await browser.waitUntil(
      async () => !(await $('.rich-text-edit-textarea').isDisplayed().catch(() => false)),
      { timeout: 5_000, timeoutMsg: 'expected unchanged paragraph editor to close' },
    );
    await selectHomeRibbonTab();
    await expect($('[data-testid="undo-btn"]')).toBeDisabled();
    await selectEditRibbonTab();

    const point = await getCenterOfTextSpan('First line');
    await browser.action('pointer').move({ x: point.cx, y: point.cy }).down({ button: 0 }).up({ button: 0 }).perform();
    await $('.paragraph-selection-overlay').waitForDisplayed({ timeout: 10_000 });
    const paragraphLeft = await browser.execute(() => Number.parseFloat(getComputedStyle(document.querySelector('.paragraph-selection-overlay')!).left));
    await browser.keys(['ArrowRight', 'ArrowRight']);
    const nudgedParagraphLeft = await browser.execute(() => Number.parseFloat(getComputedStyle(document.querySelector('.paragraph-selection-overlay')!).left));
    expect(nudgedParagraphLeft - paragraphLeft).toBe(2);
    await browser.keys('Enter');
    const reopenedTextarea = await $('.rich-text-edit-textarea');
    await reopenedTextarea.waitForDisplayed({ timeout: 10_000 });

    // Verify the textarea contains the paragraph text (both lines joined by newline).
    const value = await reopenedTextarea.getValue();
    expect(value).toContain('First line of paragraph');
    expect(value).toContain('Second line of paragraph');

    await reopenedTextarea.setValue('Edited paragraph text');

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

    // Click inside the visible page area to select the full-page image.
    const { cx, cy } = await browser.execute(() => {
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!img) throw new Error('missing page image');
      const rect = img.getBoundingClientRect();
      return { cx: Math.round(rect.left + rect.width / 2), cy: Math.round(rect.top + rect.height * 0.35) };
    });

    await browser
      .action('pointer')
      .move({ x: cx, y: cy })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();

    const overlay = await $('.image-selection-overlay');
    await overlay.waitForDisplayed({ timeout: 10_000 });
    await $('[aria-label="Image editing toolbar"]').waitForDisplayed({ timeout: 10_000 });
    expect(await $('[aria-label="Image editing toolbar"]').$$('.pdf-edit-context-icon')).toHaveLength(6);
    await $('button=Rotate Left').waitForDisplayed({ timeout: 10_000 });
    await $('button=Replace').waitForDisplayed({ timeout: 10_000 });
    await $('button=Apply').waitForDisplayed({ timeout: 10_000 });
    await $('button=Delete').waitForDisplayed({ timeout: 10_000 });
    await $('button=Cancel').waitForDisplayed({ timeout: 10_000 });

    await $('button=Apply').click();
    await browser.waitUntil(
      async () => !(await $('.image-selection-overlay').isDisplayed().catch(() => false)),
      { timeout: 5_000, timeoutMsg: 'expected unchanged image selection to close' },
    );
    await selectHomeRibbonTab();
    await expect($('[data-testid="undo-btn"]')).toBeDisabled();
    const freshImagePoint = await browser.execute(() => {
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!img) throw new Error('missing page image');
      const rect = img.getBoundingClientRect();
      return { x: Math.round(rect.left + rect.width / 2), y: Math.round(rect.top + rect.height * 0.35) };
    });
    await browser.action('pointer').move(freshImagePoint).down({ button: 0 }).up({ button: 0 }).perform();
    await $('.image-selection-overlay').waitForDisplayed({ timeout: 10_000 });

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

  it('inserts and immediately selects an image', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    const insertBtn = await $('[aria-label="Add image"]');
    await insertBtn.waitForDisplayed({ timeout: 10_000 });
    await browser.execute((imagePath) => {
      const target = window as Window & { __kanopriiOriginalPrompt?: typeof window.prompt };
      target.__kanopriiOriginalPrompt = window.prompt;
      window.prompt = () => imagePath;
    }, fixturePng);
    await insertBtn.click();

    const selection = await $('.image-selection-overlay');
    await selection.waitForDisplayed({ timeout: 15_000 });
    await $('[aria-label="Image editing toolbar"]').waitForDisplayed({ timeout: 10_000 });
    await expect($('button=Rotate Left')).toBeDisplayed();
    await expect($('button=Rotate Right')).toBeDisplayed();
    await expect($('button=Replace')).toBeDisplayed();
    await expect($('button=Apply')).toBeDisplayed();
    await expect($('button=Delete')).toBeDisplayed();
    await expect($('button=Cancel')).toBeDisplayed();
    expect(await selection.$$('.image-selection-handle')).toHaveLength(8);

    const beforeNudge = await browser.execute(() => Number.parseFloat(getComputedStyle(document.querySelector('.image-selection-overlay')!).left));
    await browser.execute(() => (document.querySelector('.image-selection-overlay') as HTMLElement).focus());
    await browser.keys(['ArrowRight', 'ArrowRight']);
    const afterNudge = await browser.execute(() => Number.parseFloat(getComputedStyle(document.querySelector('.image-selection-overlay')!).left));
    expect(afterNudge - beforeNudge).toBe(2);

    const before = await selection.getSize();
    const handle = await $('.image-selection-handle-se');
    const handleCenter = await browser.execute(() => {
      const rect = document.querySelector('.image-selection-handle-se')!.getBoundingClientRect();
      return { x: Math.round(rect.left + rect.width / 2), y: Math.round(rect.top + rect.height / 2) };
    });
    await browser.action('pointer').move(handleCenter).down({ button: 0 })
      .move({ x: handleCenter.x + 40, y: handleCenter.y + 30 }).up({ button: 0 }).perform();
    await handle.waitForDisplayed({ timeout: 5_000 });
    const after = await selection.getSize();
    expect(after.width).toBeGreaterThan(before.width);
    expect(after.height).toBeGreaterThan(before.height);

    await $('button=Cancel').click();
    await browser.waitUntil(
      async () => !(await $('.image-selection-overlay').isDisplayed().catch(() => false)),
      { timeout: 5_000, timeoutMsg: 'expected inserted image selection to close' },
    );
    await selectHomeRibbonTab();
    await expect($('[data-testid="undo-btn"]')).toBeEnabled();
    await browser.execute(() => {
      const target = window as Window & { __kanopriiOriginalPrompt?: typeof window.prompt };
      if (target.__kanopriiOriginalPrompt) window.prompt = target.__kanopriiOriginalPrompt;
      delete target.__kanopriiOriginalPrompt;
    });
  });

  it('selects, moves, and applies an existing vector', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await waitForPageRendered();
    await selectEditRibbonTab();

    await $('button=Edit Vector').click();
    const points = await browser.execute(() => {
      const page = document.querySelector('.page-image')!.getBoundingClientRect();
      return {
        x1: Math.round(page.left + page.width * 0.45),
        y1: Math.round(page.top + page.height * 0.3),
        x2: Math.round(page.left + page.width * 0.62),
        y2: Math.round(page.top + page.height * 0.4),
      };
    });
    await browser.action('pointer')
      .move({ x: points.x1, y: points.y1 }).down({ button: 0 })
      .move({ x: points.x2, y: points.y2 }).up({ button: 0 }).perform();

    const selection = await $('[aria-label="Vector selection"]');
    await selection.waitForDisplayed({ timeout: 15_000 });
    const vectorToolbar = await $('[aria-label="Vector editing toolbar"]');
    await vectorToolbar.waitForDisplayed({ timeout: 10_000 });
    expect(await selection.$$('.paragraph-handle')).toHaveLength(8);
    await expect(vectorToolbar.$('button=Apply')).toBeDisplayed();
    await expect(vectorToolbar.$('button=Delete')).toBeDisplayed();
    await expect(vectorToolbar.$('button=Cancel')).toBeDisplayed();
    await vectorToolbar.$('button=Apply').click();
    await browser.waitUntil(
      async () => !(await $('[aria-label="Vector selection"]').isDisplayed().catch(() => false)),
      { timeout: 5_000, timeoutMsg: 'expected unchanged vector selection to close' },
    );

    await selectHomeRibbonTab();
    const undo = await $('[data-testid="undo-btn"]');
    await undo.click();
    await browser.waitUntil(() => undo.isEnabled().then((enabled) => !enabled), {
      timeout: 15_000,
      timeoutMsg: 'expected one Undo to remove the newly created vector',
    });
    expect(await $('.page-vector-edit-overlay:not(.page-vector-draft)').isDisplayed().catch(() => false)).toBe(false);
    await selectEditRibbonTab();

    await browser.action('pointer')
      .move({ x: points.x1, y: points.y1 }).down({ button: 0 })
      .move({ x: points.x2, y: points.y2 }).up({ button: 0 }).perform();
    const restoredSelection = await $('[aria-label="Vector selection"]');
    await restoredSelection.waitForDisplayed({ timeout: 15_000 });

    const beforeNudge = await browser.execute(() => Number.parseFloat(getComputedStyle(document.querySelector('[aria-label="Vector selection"]')!).left));
    await browser.keys(['ArrowRight', 'ArrowRight']);
    const afterNudge = await browser.execute(() => Number.parseFloat(getComputedStyle(document.querySelector('[aria-label="Vector selection"]')!).left));
    expect(afterNudge - beforeNudge).toBe(2);
    const before = await restoredSelection.getLocation();
    const center = await browser.execute(() => {
      const rect = document.querySelector('[aria-label="Vector selection"]')!.getBoundingClientRect();
      return { x: Math.round(rect.left + rect.width / 2), y: Math.round(rect.top + rect.height / 2) };
    });
    await browser.action('pointer').move(center).down({ button: 0 })
      .move({ x: center.x + 60, y: center.y + 40 }).up({ button: 0 }).perform();
    const movedSelection = await restoredSelection.getLocation();
    expect(movedSelection.x - before.x).toBeGreaterThan(30);
    expect(movedSelection.y - before.y).toBeGreaterThan(20);
    const movedNatural = await browser.execute(() => {
      const style = getComputedStyle(document.querySelector('[aria-label="Vector selection"]')!);
      return { x: Number.parseFloat(style.left), y: Number.parseFloat(style.top) };
    });
    await $('[aria-label="Vector editing toolbar"]').$('button=Apply').click();

    await browser.waitUntil(
      async () => !(await $('[aria-label="Vector selection"]').isDisplayed().catch(() => false)),
      { timeout: 15_000, timeoutMsg: 'expected vector selection to close after Apply' },
    );
    await browser.waitUntil(() => browser.execute(({ x, y }) =>
      Array.from(document.querySelectorAll<HTMLElement>('.page-vector-edit-overlay:not(.page-vector-draft)')).some((node) => {
        const style = getComputedStyle(node);
        return Math.abs(Number.parseFloat(style.left) - x) < 1 && Math.abs(Number.parseFloat(style.top) - y) < 1;
      }), movedNatural), {
      timeout: 15_000,
      timeoutMsg: 'expected moved vector placement to persist after Apply',
    });
  });
});

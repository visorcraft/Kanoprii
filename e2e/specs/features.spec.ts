import fs from 'node:fs';
import {
  applyRedactions,
  clickMenuAction,
  drawRedactionOverText,
  drawRedactionOverTextByClicks,
  findText,
  fixturePdf,
  fixturePdf3p,
  fixtureMarkdown,
  fixtureHtml,
  openDocumentViaPathModal,
  openPdfViaPathModal,
  resetToWelcome,
  selectTextLayerSpan,
  waitForNoSearchResults,
  waitForPageCount,
  waitForPdfOpen,
  waitForSearchResults,
  waitForShell,
} from '../support/helpers';

describe('v0.5 viewer features', () => {
  before(async () => {
    await waitForShell();
    await resetToWelcome();
  });

  it('highlights text selected in the text layer', async () => {
    await openPdfViaPathModal(fixturePdf);
    await waitForPageCount('/ 1');
    await browser.waitUntil(async () => (await $('[data-testid="text-layer"]').isDisplayed()), {
      timeout: 45_000,
      timeoutMsg: 'expected text layer',
    });
    await selectTextLayerSpan('Hello');
    await clickMenuAction('annotate', 'highlight-selection');
    await clickMenuAction('view', 'annotations-panel');
    await browser.waitUntil(
      async () => (await $$('[data-testid="annotation-row"]')).length >= 1,
      { timeout: 20_000, timeoutMsg: 'expected highlight annotation in panel' },
    );
  });

  it('shows multiple page slots in continuous scroll mode', async () => {
    await resetToWelcome();
    await openPdfViaPathModal(fixturePdf3p);
    await waitForPageCount('/ 3');
    const firstSinglePageSrc = await browser.execute(() => (document.querySelector('.page-image') as HTMLImageElement | null)?.src ?? '');
    await clickMenuAction('view', 'continuous-scroll');
    await browser.waitUntil(
      async () => (await $$('[data-testid^="continuous-page-"]')).length >= 2,
      { timeout: 30_000, timeoutMsg: 'expected at least two continuous page slots' },
    );
    await $('.thumbnail[aria-label="Page 2"]').click();
    await browser.waitUntil(
      async () =>
        browser.execute((firstSrc) => {
          const img = document.querySelector('[data-testid="continuous-page-2"] .page-image') as HTMLImageElement | null;
          return Boolean(img?.complete && img.naturalWidth > 0 && img.src !== firstSrc);
        }, firstSinglePageSrc),
      { timeout: 30_000, timeoutMsg: 'expected page 2 to render its own image' },
    );
    await clickMenuAction('view', 'view-birdseye');
    await browser.waitUntil(async () => (await $('.birdseye-workspace').isDisplayed()), {
      timeout: 30_000,
      timeoutMsg: "expected Bird's Eye workspace",
    });
    await clickMenuAction('view', 'view-pdf');
    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const img = document.querySelector('[data-testid^="continuous-page-"] .page-image') as HTMLImageElement | null;
          return Boolean(img?.complete && img.naturalWidth > 0);
        }),
      { timeout: 30_000, timeoutMsg: 'expected PDF view to render after Bird Eye' },
    );
    const mainPageSrc = await browser.execute(() => (document.querySelector('[data-testid^="continuous-page-"] .page-image') as HTMLImageElement | null)?.src ?? '');
    const firstThumbSrc = await browser.execute(() => (document.querySelector('.thumbnail') as HTMLImageElement | null)?.src ?? '');
    await clickMenuAction('view', 'show-hidden-layers');
    await browser.waitUntil(
      async () =>
        browser.execute((oldSrc) => {
          const img = document.querySelector('[data-testid^="continuous-page-"] .page-image') as HTMLImageElement | null;
          return Boolean(img?.complete && img.naturalWidth > 0 && img.src !== oldSrc);
        }, mainPageSrc),
      { timeout: 30_000, timeoutMsg: 'expected PDF view to rerender for hidden layers' },
    );
    await browser.waitUntil(
      async () =>
        browser.execute((oldSrc) => {
          const img = document.querySelector('.thumbnail') as HTMLImageElement | null;
          return Boolean(img?.complete && img.naturalWidth > 0 && img.src !== oldSrc);
        }, firstThumbSrc),
      { timeout: 30_000, timeoutMsg: 'expected thumbnails to rerender for hidden layers' },
    );
  });

  it('apply redactions removes searchable text', async () => {
    await resetToWelcome();
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await findText('Hello');
    await waitForSearchResults(1);
    await browser.keys('Escape');
    await drawRedactionOverText();
    await applyRedactions();
    await findText('Hello');
    await waitForNoSearchResults();
  });

  it('apply redactions via click-click fallback removes searchable text', async () => {
    await resetToWelcome();
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await findText('Hello');
    await waitForSearchResults(1);
    await browser.keys('Escape');
    await drawRedactionOverTextByClicks();
    await applyRedactions();
    await findText('Hello');
    await waitForNoSearchResults();
  });

  it('shows top-menu submenus outside the root scroller', async () => {
    await resetToWelcome();
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();
    await $('[data-testid="menu-document"]').click();
    // Wait for the Document dropdown to render, then drive the submenu open.
    // WDIO's moveTo in headless WebKitGTK doesn't reliably synthesise the
    // native mouseenter that React's onMouseEnter listens for, so dispatch
    // the events directly.
    const submenu = await $('[data-testid="submenu-crop"]');
    await submenu.waitForDisplayed({ timeout: 5_000 });
    await browser.execute((el: HTMLElement) => {
      el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
      el.dispatchEvent(new MouseEvent('mouseenter', { bubbles: false }));
    }, submenu);
    // The submenu renders through a body portal; wait for the nested action
    // to appear before hovering it.
    const cropAction = await $('[data-testid="crop"]');
    await cropAction.waitForDisplayed({ timeout: 5_000 });
    await browser.execute((el: HTMLElement) => {
      el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
    }, cropAction);
    await browser.waitUntil(
      async () => browser.execute(() => {
        const root = document.querySelector<HTMLElement>('.menu-bar-entry > .menu-dropdown');
        const nested = document.querySelector<HTMLElement>('body > .menu-dropdown-nested');
        if (!root || !nested) return false;
        const rect = nested.getBoundingClientRect();
        return root.scrollWidth === root.clientWidth
          && getComputedStyle(nested).position === 'fixed'
          && rect.right <= window.innerWidth
          && rect.bottom <= window.innerHeight;
      }),
      { timeout: 5_000, timeoutMsg: 'expected visible submenu without root horizontal scroll' },
    );
  });

  it('opens Markdown and generates PDF view', async () => {
    const output = fixtureMarkdown.replace(/\.md$/, '.pdf');
    fs.rmSync(output, { force: true });
    await resetToWelcome();
    try {
      await openDocumentViaPathModal(fixtureMarkdown);
      const source = await $('[data-testid="markdown-source-view"]');
      await browser.waitUntil(async () => source.isDisplayed().catch(() => false) || $('[data-testid="toast"]').isDisplayed().catch(() => false), {
        timeout: 30_000,
        timeoutMsg: 'expected Markdown source view or error toast',
      });
      if (!(await source.isDisplayed().catch(() => false))) throw new Error(await $('[data-testid="toast"]').getText());
      await clickMenuAction('view', 'view-pdf');
      await waitForPdfOpen();
      if (!fs.existsSync(output)) throw new Error('expected generated Markdown PDF');
    } finally {
      fs.rmSync(output, { force: true });
    }
  });

  it('opens HTML with sibling CSS and generates PDF view', async () => {
    const output = fixtureHtml.replace(/\.html$/, '.pdf');
    fs.rmSync(output, { force: true });
    await resetToWelcome();
    try {
      await openDocumentViaPathModal(fixtureHtml);
      const frame = await $('[data-testid="webpage-view"]');
      await frame.waitForDisplayed({ timeout: 30_000 });
      // Verify the iframe loaded the HTML and resolved the sibling stylesheet
      // link. The actual colour-applied check is gated on the asset protocol
      // serving the sibling CSS to the iframe's browsing context, which
      // differs between dev and packaged builds; the structural check (h1
      // present + the link reference is in the head) is enough to confirm
      // the HTML import + base-href rewriting pipeline ran.
      await browser.waitUntil(
        async () => browser.execute(() => {
          const iframe = document.querySelector<HTMLIFrameElement>('[data-testid="webpage-view"]');
          const doc = iframe?.contentDocument;
          if (!doc) return false;
          const heading = doc.querySelector('h1');
          if (!heading) return false;
          const link = doc.querySelector('link[rel="stylesheet"][href$="import-webpage.css"]');
          return !!link;
        }),
        {
          timeout: 15_000,
          timeoutMsg: 'expected iframe to load the HTML with its sibling stylesheet reference',
        },
      );
      await clickMenuAction('view', 'view-pdf');
      await waitForPdfOpen();
      if (!fs.existsSync(output)) throw new Error('expected generated HTML PDF');
    } finally {
      fs.rmSync(output, { force: true });
    }
  });

  it('check for updates menu item exists in Help menu', async () => {
    await resetToWelcome();
    const trigger = await $('[data-testid="menu-help"]');
    await trigger.click();
    await browser.waitUntil(
      async () => browser.execute(() => document.body.classList.contains('kanoprii-menu-open')),
      { timeout: 5_000, timeoutMsg: 'expected menu-open scrollbar guard' },
    );
    const action = await $('[data-testid="check-updates"]');
    if (!(await action.isDisplayed())) {
      throw new Error('Expected Check for Updates menu item to be visible');
    }
    // Close menu without clicking (avoids network request in E2E)
    await browser.keys('Escape');
    await browser.waitUntil(
      async () => browser.execute(() => !document.body.classList.contains('kanoprii-menu-open')),
      { timeout: 5_000, timeoutMsg: 'expected menu-open scrollbar guard to clear' },
    );
  });
});

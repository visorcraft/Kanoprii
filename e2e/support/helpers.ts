import path from 'node:path';
import { fileURLToPath } from 'node:url';

const fixturesDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'fixtures');

export const fixturePdf = path.join(fixturesDir, 'sample.pdf');
export const fixturePdf3p = path.join(fixturesDir, 'sample-3p.pdf');
export const fixturePdfB = path.join(fixturesDir, 'sample-b.pdf');

type TauriExecute = <T>(script: (api: { core: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> } }) => T | Promise<T>, ...args: unknown[]) => Promise<T>;

function tauriExecute(): TauriExecute {
  const tauri = (browser as WebdriverIO.Browser & { tauri?: { execute: TauriExecute } }).tauri;
  if (!tauri?.execute) {
    throw new Error('browser.tauri.execute is unavailable - rebuild with scripts/e2e-build.sh');
  }
  return tauri.execute.bind(tauri);
}

export async function waitForTauriReady() {
  await browser.waitUntil(
    async () =>
      browser.execute(async () => {
        const w = window as Window & { wdioTauri?: { waitForInit?: () => Promise<void> } };
        if (w.wdioTauri?.waitForInit) {
          await w.wdioTauri.waitForInit();
          return true;
        }
        return Boolean((window as Window & { __TAURI__?: { core?: { invoke?: unknown } } }).__TAURI__?.core?.invoke);
      }),
    { timeout: 60_000, timeoutMsg: 'Tauri invoke API not ready' },
  );
}

export async function waitForShell() {
  await waitForTauriReady();
  await browser.waitUntil(async () => (await $('[data-testid="menu-file"]').isDisplayed()), {
    timeout: 30_000,
    timeoutMsg: 'expected application menu',
  });
}

export async function invokeBackend<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return tauriExecute()(({ core }) => core.invoke(cmd, args) as Promise<T>);
}

export async function pathExists(filePath: string): Promise<boolean> {
  if (!filePath) return false;
  try {
    await invokeBackend<number>('get_pdf_page_count', { path: filePath });
    return true;
  } catch {
    return false;
  }
}

export async function openFileMenu() {
  await waitForShell();
  const trigger = await $('[data-testid="menu-file"]');
  for (let attempt = 0; attempt < 3; attempt++) {
    await trigger.click();
    const openItem = await $('[data-testid="open-pdf"]');
    if (await openItem.isDisplayed().catch(() => false)) return;
    await browser.pause(150);
  }
  throw new Error('file menu did not open');
}

async function showOpenPdfModal() {
  const welcome = await $('[data-testid="welcome-open-pdf"]');
  if (await welcome.isDisplayed().catch(() => false)) {
    await welcome.click();
    return;
  }
  await openFileMenu();
  await $('[data-testid="open-pdf"]').click();
}

export async function discardUnsavedIfPrompted() {
  const discard = await $('[data-testid="unsaved-discard"]');
  if (await discard.isDisplayed().catch(() => false)) {
    await discard.click();
    await browser.pause(200);
  }
}

export async function cancelUnsavedIfPrompted() {
  const cancel = await $('[data-testid="unsaved-cancel"]');
  if (await cancel.isDisplayed().catch(() => false)) {
    await cancel.click();
    await browser.pause(200);
  }
}

export async function openPdfViaPathModal(pdfPath: string) {
  await showOpenPdfModal();
  await discardUnsavedIfPrompted();

  const pathInput = await $('[data-testid="open-pdf-path"]');
  await pathInput.waitForDisplayed({ timeout: 15_000 });
  await pathInput.click();
  await browser.execute((filePath: string) => {
    const input = document.querySelector('[data-testid="open-pdf-path"]') as HTMLInputElement | null;
    if (!input) throw new Error('path input missing');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter?.call(input, filePath);
    input.dispatchEvent(new InputEvent('input', { bubbles: true, data: filePath, inputType: 'insertFromPaste' }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, pdfPath);

  const submit = await $('[data-testid="open-pdf-submit"]');
  await browser.waitUntil(async () => (await submit.isEnabled()), {
    timeout: 10_000,
    timeoutMsg: 'open submit enabled',
  });
  await submit.click();
  const pdfView = await $('[data-testid="open-pdf-view"]');
  if (await pdfView.isDisplayed().catch(() => false)) {
    await pdfView.click();
  }
  await waitForPdfOpen();
}

export async function waitForPdfOpen() {
  await browser.waitUntil(
    async () => {
      const btn = await $('[data-testid="save-pdf"]');
      return btn.isDisplayed().catch(() => false);
    },
    { timeout: 90_000, timeoutMsg: 'expected PDF viewer toolbar' },
  );
  await waitForPageRendered();
}

export async function waitForPageRendered() {
  await browser.waitUntil(
    async () =>
      browser.execute(() => {
        const img = document.querySelector('.page-image') as HTMLImageElement | null;
        return Boolean(img?.complete && img.naturalWidth > 0);
      }),
    { timeout: 90_000, timeoutMsg: 'expected rendered page image' },
  );
}

export async function waitForPageCount(expected: string) {
  await browser.waitUntil(
    async () => {
      const els = await $$('[data-testid="page-count"]');
      if (els.length === 0) return false;
      const text = await els[0].getText();
      return text === expected;
    },
    { timeout: 60_000, timeoutMsg: `expected page count ${expected}` },
  );
}

export async function waitForWelcome() {
  await browser.waitUntil(
    async () => (await $('[data-testid="welcome-open-pdf"]').isDisplayed()),
    { timeout: 30_000, timeoutMsg: 'expected welcome screen' },
  );
}

export async function clickMenuAction(menuId: string, actionId: string) {
  const trigger = await $(`[data-testid="menu-${menuId}"]`);
  for (let attempt = 0; attempt < 3; attempt++) {
    await trigger.click();
    const action = await $(`[data-testid="${actionId}"]`);
    if (await action.isDisplayed().catch(() => false)) {
      await action.click();
      return;
    }
    await browser.pause(150);
  }
  throw new Error(`menu action ${menuId}/${actionId} not found`);
}

export async function clickQuickAction(testId: string) {
  const btn = await $(`[data-testid="${testId}"]`);
  await btn.waitForDisplayed({ timeout: 10_000 });
  await btn.click();
}

export async function rotateCurrentPage() {
  await clickQuickAction('rotate-page');
  const applyBtn = await $('[data-testid="rotate-modal-apply"]');
  await applyBtn.waitForDisplayed({ timeout: 10_000 });
  await applyBtn.click();
}

export async function selectTextLayerSpan(text: string) {
  await browser.waitUntil(
    async () => {
      const spans = await $$('.text-layer span');
      for (const span of spans) {
        if ((await span.getText()).includes(text)) return true;
      }
      return false;
    },
    { timeout: 60_000, timeoutMsg: `text layer span containing "${text}"` },
  );
  await browser.execute((needle: string) => {
    const layer = document.querySelector('.text-layer');
    if (!layer) throw new Error('text layer missing');
    const span = Array.from(layer.querySelectorAll('span')).find((el) => el.textContent?.includes(needle));
    if (!span?.firstChild) throw new Error(`span with "${needle}" missing`);
    const range = document.createRange();
    range.setStart(span.firstChild, 0);
    range.setEnd(span.firstChild, Math.min(needle.length, span.textContent?.length ?? 0));
    const sel = window.getSelection();
    sel?.removeAllRanges();
    sel?.addRange(range);
  }, text);
}

// `[data-testid^="doc-tab-"]` alone also matches the close buttons
// (`doc-tab-close-…`), so tab counts must exclude them.
export async function countDocTabs(): Promise<number> {
  return browser.execute(
    () =>
      document.querySelectorAll('[data-testid^="doc-tab-"]:not([data-testid^="doc-tab-close-"])')
        .length,
  );
}

export async function focusTabByLabel(label: string) {
  const tab = await $(`[data-testid="doc-tab-${label}"]`);
  await tab.waitForDisplayed({ timeout: 15_000 });
  await browser.execute((selector: string) => {
    const el = document.querySelector(`[data-testid="${selector}"]`) as HTMLElement | null;
    if (!el) throw new Error(`tab ${selector} not found`);
    el.focus();
  }, `doc-tab-${label}`);
}

export async function selectTab(label: string) {
  const tab = await $(`[data-testid="doc-tab-${label}"]`);
  await tab.waitForDisplayed({ timeout: 15_000 });
  await tab.click();
  await waitForPageRendered();
}

export async function closeTab(label: string) {
  const close = await $(`[data-testid="doc-tab-close-${label}"]`);
  await close.waitForDisplayed({ timeout: 10_000 });
  await close.click();
}

export async function closeActiveDocument() {
  const tabs = await $$('[data-testid^="doc-tab-"]');
  if (tabs.length > 0) {
    const close = await $('[data-testid^="doc-tab-close-"]');
    await close.click();
    return;
  }
  await clickMenuAction('file', 'close');
}

export async function dismissToastIfAny() {
  const close = await $('[data-testid="toast-close"]');
  if (await close.isDisplayed().catch(() => false)) {
    await close.click();
    await browser.pause(150);
  }
}

export async function resetToWelcome() {
  await dismissToastIfAny();

  // If the Settings page is open, close it first so the main chrome is reachable.
  const settingsBack = await $('[data-testid="settings-back-button"]');
  if (await settingsBack.isDisplayed().catch(() => false)) {
    await settingsBack.click();
    await browser.pause(200);
  }

  for (let guard = 0; guard < 8; guard++) {
    const welcome = await $('[data-testid="welcome-open-pdf"]');
    if (await welcome.isDisplayed().catch(() => false)) return;
    await closeActiveDocument();
    await discardUnsavedIfPrompted();
    await browser.pause(200);
  }
  await waitForWelcome();
}

export async function getSaveLabel(): Promise<string> {
  return (await $('[data-testid="save-pdf"]')).getText();
}

export async function getZoomInput(): Promise<string> {
  return (await $('[data-testid="zoom-input"]')).getValue();
}

export async function setZoomPercent(value: string) {
  const input = await $('[data-testid="zoom-input"]');
  await input.click();
  await browser.execute((zoom: string) => {
    const el = document.querySelector('[data-testid="zoom-input"]') as HTMLInputElement | null;
    if (!el) throw new Error('zoom input missing');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter?.call(el, zoom);
    el.dispatchEvent(new InputEvent('input', { bubbles: true, data: zoom, inputType: 'insertFromPaste' }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    el.blur();
  }, value);
  await browser.pause(150);
}

export async function drawRedactionOverText() {
  await waitForPageRendered();
  await clickMenuAction('annotate', 'redact');

  // mousedown
  await browser.execute(
    (x1: number, y1: number) => {
      const container = document.querySelector('[data-testid="page-container"]');
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!container || !img) throw new Error('page container missing');
      const rect = img.getBoundingClientRect();
      const cx = rect.left + rect.width * x1;
      const cy = rect.top + rect.height * y1;
      container.dispatchEvent(
        new MouseEvent('mousedown', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
    },
    0.2, 0.12,
  );
  await browser.pause(100);

  // mousemove
  await browser.execute(
    (x2: number, y2: number) => {
      const container = document.querySelector('[data-testid="page-container"]');
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!container || !img) throw new Error('page container missing');
      const rect = img.getBoundingClientRect();
      const cx = rect.left + rect.width * x2;
      const cy = rect.top + rect.height * y2;
      container.dispatchEvent(
        new MouseEvent('mousemove', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
    },
    0.75, 0.18,
  );
  await browser.pause(100);

  // mouseup
  await browser.execute(
    (x2: number, y2: number) => {
      const container = document.querySelector('[data-testid="page-container"]');
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!container || !img) throw new Error('page container missing');
      const rect = img.getBoundingClientRect();
      const cx = rect.left + rect.width * x2;
      const cy = rect.top + rect.height * y2;
      container.dispatchEvent(
        new MouseEvent('mouseup', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
    },
    0.75, 0.18,
  );
  await browser.pause(400);
}

export async function drawRedactionOverTextByClicks() {
  await waitForPageRendered();
  await clickMenuAction('annotate', 'redact');

  // First click: mousedown + click (arms)
  await browser.execute(
    (fx: number, fy: number) => {
      const container = document.querySelector('[data-testid="page-container"]');
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!container || !img) throw new Error('page container missing');
      const rect = img.getBoundingClientRect();
      const cx = rect.left + rect.width * fx;
      const cy = rect.top + rect.height * fy;
      container.dispatchEvent(
        new MouseEvent('mousedown', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
      container.dispatchEvent(
        new MouseEvent('click', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
    },
    0.2, 0.12,
  );
  await browser.pause(150);

  // Second click: mousedown + mouseup + click (commits)
  await browser.execute(
    (fx: number, fy: number) => {
      const container = document.querySelector('[data-testid="page-container"]');
      const img = document.querySelector('.page-image') as HTMLImageElement | null;
      if (!container || !img) throw new Error('page container missing');
      const rect = img.getBoundingClientRect();
      const cx = rect.left + rect.width * fx;
      const cy = rect.top + rect.height * fy;
      container.dispatchEvent(
        new MouseEvent('mousedown', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
      container.dispatchEvent(
        new MouseEvent('mouseup', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
      container.dispatchEvent(
        new MouseEvent('click', {
          bubbles: true,
          cancelable: true,
          clientX: cx,
          clientY: cy,
          button: 0,
        }),
      );
    },
    0.75, 0.18,
  );
  await browser.pause(400);
}

export async function openFindModal() {
  await clickQuickAction('find-btn');
  await (await $('[data-testid="search-query"]')).waitForDisplayed({ timeout: 10_000 });
}

export async function findText(query: string) {
  await openFindModal();
  const input = await $('[data-testid="search-query"]');
  await input.click();
  await browser.execute((q: string) => {
    const el = document.querySelector('[data-testid="search-query"]') as HTMLInputElement | null;
    if (!el) throw new Error('search query missing');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter?.call(el, q);
    el.dispatchEvent(new InputEvent('input', { bubbles: true, data: q, inputType: 'insertFromPaste' }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  }, query);
  await (await $('[data-testid="search-find"]')).click();
}

export async function waitForSearchResults(min = 1) {
  await browser.waitUntil(
    async () => {
      const summary = await $('[data-testid="search-results"]');
      if (!(await summary.isDisplayed().catch(() => false))) return min === 0;
      const text = await summary.getText();
      const match = text.match(/of (\d+)/);
      return match ? Number(match[1]) >= min : false;
    },
    { timeout: 30_000, timeoutMsg: `expected at least ${min} search result(s)` },
  );
}

export async function waitForNoSearchResults() {
  await browser.waitUntil(
    async () => !(await $('[data-testid="search-results"]').isDisplayed().catch(() => false)),
    { timeout: 30_000, timeoutMsg: 'expected no search results' },
  );
}

export async function captureTabWorkingPaths(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('[data-working-path]'))
      .map((el) => el.getAttribute('data-working-path') ?? '')
      .filter(Boolean),
  );
}

export async function applyRedactions() {
  await browser.waitUntil(
    async () => (await $$('.redaction-overlay')).length >= 1,
    { timeout: 20_000, timeoutMsg: 'expected redaction overlay on page' },
  );
  await clickMenuAction('document', 'apply-redactions');
  await (await $('[data-testid="apply-redactions-confirm"]')).waitForDisplayed({ timeout: 10_000 });
  await (await $('[data-testid="apply-redactions-confirm"]')).click();
  await waitForPageRendered();
}


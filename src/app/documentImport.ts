import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import html2canvas from 'html2canvas';
import { marked } from 'marked';

export type SourceKind = 'pdf' | 'markdown' | 'html';

export function sourceKindFromPath(path: string): SourceKind | null {
  const ext = path.split('.').pop()?.toLowerCase();
  if (ext === 'pdf') return 'pdf';
  if (ext === 'md' || ext === 'markdown') return 'markdown';
  if (ext === 'html' || ext === 'htm') return 'html';
  return null;
}

function baseUrl(path: string): string {
  const slash = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return convertFileSrc(slash < 0 ? path : path.slice(0, slash + 1));
}

export function documentHtml(path: string, text: string, kind: Exclude<SourceKind, 'pdf'>): string {
  const body = kind === 'markdown'
    ? `<main class="kanoprii-markdown">${marked.parse(text) as string}</main>`
    : text;
  const base = `<base href="${baseUrl(path)}">`;
  const defaults = `<style>
    :root { color-scheme: light; background: white; }
    body { margin: 0 auto; padding: 48px; max-width: 900px; color: #181818; background: white; font: 16px/1.55 system-ui, sans-serif; }
    img, svg, video { max-width: 100%; height: auto; }
    pre { white-space: pre-wrap; overflow-wrap: anywhere; }
    table { border-collapse: collapse; }
    th, td { border: 1px solid #bbb; padding: 6px 10px; }
  </style>`;
  if (kind === 'html' && /<html[\s>]/i.test(body)) {
    if (/<head[\s>]/i.test(body)) {
      return body.replace(/<head([^>]*)>/i, `<head$1>${base}${defaults}`);
    }
    return body.replace(/<html([^>]*)>/i, `<html$1><head>${base}${defaults}</head>`);
  }
  return `<!doctype html><html><head>${base}${defaults}</head><body>${body}</body></html>`;
}

const PAGE_WIDTH = 900;
const PAGE_HEIGHT = Math.round(PAGE_WIDTH * 792 / 612);
const MAX_PAGES = 100;
const SINGLE_PASS_LIMIT = 32000;
const BATCH_CANVAS_HEIGHT = 16000;
const JPEG_QUALITY = 0.92;

export type CreateWorkingPdfOptions = {
  onProgress?: (renderedPages: number, totalPages: number) => void;
};

async function within<T>(promise: Promise<T>, label: string, ms = 10_000): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => setTimeout(() => reject(new Error(`${label} timed out`)), ms)),
  ]);
}

async function canvasJpeg(canvas: HTMLCanvasElement): Promise<number[]> {
  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((value) => value ? resolve(value) : reject(new Error('Could not encode document page')), 'image/jpeg', JPEG_QUALITY),
  );
  return [...new Uint8Array(await blob.arrayBuffer())];
}

async function sliceCanvasToPages(canvas: HTMLCanvasElement, scale: number, sourceHeightCss: number): Promise<number[][]> {
  const pages: number[][] = [];
  const destWidth = Math.round(PAGE_WIDTH * scale);
  const destPageHeight = Math.round(PAGE_HEIGHT * scale);
  const pageCount = Math.ceil(sourceHeightCss / PAGE_HEIGHT);
  for (let i = 0; i < pageCount; i++) {
    const sliceTopCss = i * PAGE_HEIGHT;
    const sliceHeightCss = Math.min(PAGE_HEIGHT, sourceHeightCss - sliceTopCss);
    const srcTopPx = Math.round(sliceTopCss * scale);
    const srcHeightPx = Math.max(1, Math.round(sliceHeightCss * scale));
    const page = document.createElement('canvas');
    page.width = destWidth;
    page.height = destPageHeight;
    const context = page.getContext('2d');
    if (!context) throw new Error('Canvas is unavailable');
    context.fillStyle = '#ffffff';
    context.fillRect(0, 0, destWidth, destPageHeight);
    context.drawImage(canvas, 0, srcTopPx, canvas.width, srcHeightPx, 0, 0, destWidth, srcHeightPx);
    pages.push(await canvasJpeg(page));
  }
  return pages;
}

async function waitForImages(doc: Document): Promise<void> {
  const imagesReady = Promise.all([...doc.images].map((image) => image.complete
    ? Promise.resolve()
    : new Promise<void>((resolve) => {
        image.addEventListener('load', () => resolve(), { once: true });
        image.addEventListener('error', () => resolve(), { once: true });
      })));
  await Promise.race([imagesReady, new Promise((resolve) => setTimeout(resolve, 5000))]);
}

type RenderState = { pages: number[][]; totalRendered: number; totalPages: number };

async function renderFrame(srcdoc: string, state: RenderState, opts: CreateWorkingPdfOptions): Promise<void> {
  const frame = document.createElement('iframe');
  frame.sandbox.add('allow-same-origin');
  frame.style.cssText = 'position:fixed;left:-10000px;top:0;width:900px;height:1100px;border:0;background:white';
  document.body.appendChild(frame);
  const loaded = new Promise<void>((resolve, reject) => {
    frame.onload = () => resolve();
    frame.onerror = () => reject(new Error('Could not render document'));
  });
  frame.srcdoc = srcdoc;
  try {
    await within(loaded, 'Document load');
    const doc = frame.contentDocument;
    if (!doc) throw new Error('Could not access rendered document');
    await within(doc.fonts.ready, 'Font load');
    await waitForImages(doc);
    const root = doc.documentElement;
    const fullHeight = root.scrollHeight;
    if (fullHeight <= 0) throw new Error('Document produced no pages');
    const framePageCount = Math.ceil(fullHeight / PAGE_HEIGHT);
    if (state.totalPages + state.totalRendered + framePageCount > MAX_PAGES) {
      throw new Error(`Document exceeds the ${MAX_PAGES} page conversion limit`);
    }
    state.totalPages += framePageCount;
    const renderOptions = (top: number, height: number, scale: number) => ({
      backgroundColor: '#ffffff',
      logging: false,
      useCORS: true,
      scale,
      windowWidth: PAGE_WIDTH,
      width: PAGE_WIDTH,
      x: 0,
      y: top,
      height,
    });
    const fitScale = SINGLE_PASS_LIMIT / fullHeight;
    if (fitScale >= 0.5) {
      const scale = Math.min(2, fitScale);
      const canvas = await within(html2canvas(root, renderOptions(0, fullHeight, scale)), 'Document render', 120_000);
      const sliced = await sliceCanvasToPages(canvas, scale, fullHeight);
      state.pages.push(...sliced);
      state.totalRendered += sliced.length;
      opts.onProgress?.(state.totalRendered, state.totalPages);
    } else {
      const scale = 1;
      const batchHeightCss = Math.max(1, Math.floor(BATCH_CANVAS_HEIGHT / PAGE_HEIGHT)) * PAGE_HEIGHT;
      for (let top = 0; top < fullHeight; top += batchHeightCss) {
        const height = Math.min(batchHeightCss, fullHeight - top);
        const canvas = await within(html2canvas(root, renderOptions(top, height, scale)), 'Document render', 120_000);
        const sliced = await sliceCanvasToPages(canvas, scale, height);
        state.pages.push(...sliced);
        state.totalRendered += sliced.length;
        opts.onProgress?.(state.totalRendered, state.totalPages);
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }
  } finally {
    frame.remove();
  }
}

export async function createWorkingPdf(path: string, text: string, kind: Exclude<SourceKind, 'pdf'>, opts: CreateWorkingPdfOptions = {}): Promise<string> {
  if (kind === 'markdown') {
    return within(invoke<string>('create_pdf_from_markdown_text', { text }), 'PDF creation', 60_000);
  }
  const state: RenderState = { pages: [], totalRendered: 0, totalPages: 0 };
  await renderFrame(documentHtml(path, text, kind), state, opts);
  if (state.pages.length === 0) throw new Error('Document produced no pages');
  return within(invoke<string>('create_pdf_from_document_pages', { pages: state.pages }), 'PDF creation', 60_000);
}

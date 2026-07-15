import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DocumentSessionData } from '../app/documentSessionTypes';
import type { PdfEditState, Rect, TextStyle } from '../app/usePdfEditState';
import { runStructuralEdit, type StructuralEditDeps } from '../pdf/runStructuralEdit';
import type { PageTextRun } from '../pdf/useTextLayerLoader';

type PdfRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type PageImageInfo = {
  index: number;
  bbox: PdfRect;
  rect: PdfRect;
  rotation: number;
  width?: number;
  height?: number;
};

type PageImageHit = {
  index: number;
  viewerRect: Rect;
  rotation: number;
  width?: number;
  height?: number;
};

type TextLine = {
  lineIndex: number;
  text: string;
  bbox: Rect;
  fontFamily: TextStyle['fontFamily'];
  fontSize: number;
  bold: boolean;
  italic: boolean;
};

type UsePageInteractionEditOptions = {
  pdfEdit: PdfEditState;
} & StructuralEditDeps;

function toBackendStyle(style: TextStyle): TextStyle & { color: { r: number; g: number; b: number } } {
  return {
    ...style,
    color: {
      r: style.color.r / 255,
      g: style.color.g / 255,
      b: style.color.b / 255,
    },
  };
}

function rectToPdfRect(rect: Rect): PdfRect {
  return { x: rect.x, y: rect.y, width: rect.w, height: rect.h };
}

function sourceTextStyle(
  base: TextStyle,
  source: Pick<TextLine, 'fontFamily' | 'fontSize' | 'bold' | 'italic'> & {
    color?: TextStyle['color'];
  },
): TextStyle {
  return {
    ...base,
    fontFamily: source.fontFamily,
    fontSize: Math.max(6, Math.min(72, source.fontSize)),
    bold: source.bold,
    italic: source.italic,
    color: source.color ?? base.color,
  };
}

export function usePageInteractionEdit(deps: UsePageInteractionEditOptions) {
  const { pdfEdit, filePath } = deps;

  const viewerRectToPdf = useCallback(
    async (pageIndex: number, rect: Rect): Promise<PdfRect> => {
      return invoke<PdfRect>('viewer_rect_to_pdf', {
        path: filePath,
        pageIndex,
        rect: rectToPdfRect(rect),
      });
    },
    [filePath],
  );

  const pdfRectToViewerPx = useCallback(
    async (pageIndex: number, rect: PdfRect): Promise<Rect> => {
      const result = await invoke<PdfRect>('pdf_rect_to_viewer_px', {
        path: filePath,
        pageIndex,
        rect,
      });
      return { x: result.x, y: result.y, w: result.width, h: result.height };
    },
    [filePath],
  );

  const loadPageTextLines = useCallback(
    async (session: DocumentSessionData, pageIndex: number): Promise<TextLine[]> => {
      if (!session?.filePath) return [];
      const lines = await invoke<
        Array<{
          text: string;
          x: number;
          y: number;
          w: number;
          h: number;
          fontFamily: TextStyle['fontFamily'];
          fontSize: number;
          bold: boolean;
          italic: boolean;
        }>
      >(
        'get_page_text_lines',
        {
          path: session.filePath,
          pageIndex,
        },
      );
      // get_page_text_lines already returns viewer-pixel coordinates.
      return lines.map((line, idx) => ({
        lineIndex: idx,
        text: line.text,
        bbox: { x: line.x, y: line.y, w: line.w, h: line.h },
        fontFamily: line.fontFamily,
        fontSize: line.fontSize,
        bold: line.bold,
        italic: line.italic,
      }));
    },
    [],
  );

  const hitTestImage = useCallback(
    async (pageIndex: number, x: number, y: number): Promise<PageImageHit | null> => {
      if (!filePath) return null;
      const images = await invoke<PageImageInfo[]>('list_page_images', {
        path: filePath,
        pageIndex,
      });
      for (const img of images) {
        const hitRect = await pdfRectToViewerPx(pageIndex, img.bbox);
        if (
          x >= hitRect.x &&
          x <= hitRect.x + hitRect.w &&
          y >= hitRect.y &&
          y <= hitRect.y + hitRect.h
        ) {
          const viewerRect = await pdfRectToViewerPx(pageIndex, img.rect);
          return {
            index: img.index,
            viewerRect,
            rotation: img.rotation,
            width: img.width,
            height: img.height,
          };
        }
      }
      return null;
    },
    [filePath, pdfRectToViewerPx],
  );

  const hitTestPdfiumText = useCallback(
    async (session: DocumentSessionData, pageIndex: number, x: number, y: number): Promise<PageTextRun | null> => {
      const runs = await invoke<PageTextRun[]>('get_page_text_layout', {
        path: session.filePath,
        pageIndex,
      }).catch(() => []);
      for (let i = runs.length - 1; i >= 0; i -= 1) {
        const run = runs[i]!;
        if (x >= run.x && x <= run.x + run.w && y >= run.y && y <= run.y + run.h) return run;
      }
      return null;
    },
    [],
  );

  const handlePageClick = useCallback(
    async (
      pageIndex: number,
      point: { x: number; y: number },
      session: DocumentSessionData,
      hitTestImageFn?: (pageIndex: number, x: number, y: number) => Promise<PageImageHit | null>,
    ) => {
      if (!session?.filePath) return;

      if (pdfEdit.mode === 'text' || pdfEdit.mode === 'paragraph' || pdfEdit.mode === 'idle') {
        const lines = await loadPageTextLines(session, pageIndex);
        let hitLine: TextLine | null = null;
        for (let i = lines.length - 1; i >= 0; i -= 1) {
          const line = lines[i]!;
          if (
            point.x >= line.bbox.x &&
            point.x <= line.bbox.x + line.bbox.w &&
            point.y >= line.bbox.y &&
            point.y <= line.bbox.y + line.bbox.h
          ) {
            hitLine = line;
            break;
          }
        }
        const hitRun = await hitTestPdfiumText(session, pageIndex, point.x, point.y);
        if (hitLine) {
          // Try to detect a multi-line paragraph first.
          const paragraph = await invoke<{ lineIndices: number[]; x: number; y: number; w: number; h: number } | null>(
            'find_paragraph',
            {
              path: session.filePath,
              pageIndex,
              lineIndex: hitLine.lineIndex,
            },
          );
          if (paragraph) {
            const text = paragraph.lineIndices
              .map((idx) => lines[idx]?.text ?? '')
              .join('\n');
            const firstLine = lines[paragraph.lineIndices[0]!] ?? hitLine;
            pdfEdit.startEditingParagraph({
              pageIndex,
              lineIndices: paragraph.lineIndices,
              text,
              pageRect: { x: paragraph.x, y: paragraph.y, w: paragraph.w, h: paragraph.h },
              style: sourceTextStyle(pdfEdit.style, hitRun ?? firstLine),
            });
          } else {
            pdfEdit.startEditingText({
              pageIndex,
              point,
              text: hitLine.text,
              pageRect: hitLine.bbox,
              style: sourceTextStyle(pdfEdit.style, hitRun ?? hitLine),
              lineIndex: hitLine.lineIndex,
            });
          }
        } else if (hitRun) {
          const rect = { x: hitRun.x, y: hitRun.y, w: hitRun.w, h: hitRun.h };
          pdfEdit.startEditingText({
            pageIndex,
            point,
            text: hitRun.text,
            pageRect: rect,
            sourceRect: rect,
            style: sourceTextStyle(pdfEdit.style, hitRun),
          });
        } else if (pdfEdit.mode === 'text' || pdfEdit.mode === 'paragraph') {
          pdfEdit.startInsertingText(pageIndex, point, {
            x: point.x - 50,
            y: point.y - 10,
            w: 100,
            h: 20,
          });
        } else {
          // In idle mode, decide text vs. image by hit-testing.
          const image = await hitTestImageFn?.(pageIndex, point.x, point.y);
          if (image) {
            pdfEdit.startEditingImage({
              pageIndex,
              point,
              path: session.filePath,
              index: image.index,
              width: image.width,
              height: image.height,
              pageRect: image.viewerRect,
              rotation: image.rotation,
            });
          } else {
            pdfEdit.startInsertingText(pageIndex, point, {
              x: point.x - 50,
              y: point.y - 10,
              w: 100,
              h: 20,
            });
          }
        }
      } else if (pdfEdit.mode === 'image') {
        const image = await hitTestImageFn?.(pageIndex, point.x, point.y);
        if (image) {
          pdfEdit.startEditingImage({
            pageIndex,
            point,
            path: session.filePath,
            index: image.index,
            width: image.width,
            height: image.height,
            pageRect: image.viewerRect,
            rotation: image.rotation,
          });
        }
      }
    },
    [hitTestPdfiumText, pdfEdit, loadPageTextLines],
  );

  const applyTextEdit = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.textDraft) return;
      const draft = pdfEdit.textDraft;
      const boxRect = rectToPdfRect(draft.pageRect);
      const style = toBackendStyle(draft.style);
      let result: unknown;

      if (draft.lineIndex !== undefined) {
        result = await runStructuralEdit(deps, {
          command: 'edit_text_line',
          args: {
            path: session.filePath,
            pageIndex: draft.pageIndex,
            lineIndex: draft.lineIndex,
            newText: draft.text,
            style,
            boxRect,
          },
          reloadAt: draft.pageIndex,
          toast: 'Text updated',
        });
      } else if (draft.sourceRect) {
        result = await runStructuralEdit(deps, {
          command: 'edit_text_region',
          args: {
            path: session.filePath,
            pageIndex: draft.pageIndex,
            sourceRect: rectToPdfRect(draft.sourceRect),
            newText: draft.text,
            style,
            boxRect,
          },
          reloadAt: draft.pageIndex,
          toast: 'Text updated',
        });
      } else {
        result = await runStructuralEdit(deps, {
          command: 'add_text_box',
          args: {
            path: session.filePath,
            pageIndex: draft.pageIndex,
            text: draft.text,
            style,
            boxRect,
          },
          reloadAt: draft.pageIndex,
          toast: 'Text added',
        });
      }
      if (result !== undefined) pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  const applyParagraphEdit = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.paragraphDraft) return;
      const draft = pdfEdit.paragraphDraft;
      const boxRect = rectToPdfRect(draft.pageRect);
      const style = toBackendStyle(draft.style);
      const result = await runStructuralEdit(deps, {
        command: 'edit_paragraph',
        args: {
          path: session.filePath,
          pageIndex: draft.pageIndex,
          lineIndices: draft.lineIndices,
          newText: draft.text,
          style,
          boxRect,
        },
        reloadAt: draft.pageIndex,
        toast: 'Paragraph updated',
      });
      if (result !== undefined) pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  const deleteText = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.textDraft) return;
      const draft = pdfEdit.textDraft;
      let result: unknown;
      if (draft.lineIndex !== undefined) {
        result = await runStructuralEdit(deps, {
          command: 'delete_text_line',
          args: {
            path: session.filePath,
            pageIndex: draft.pageIndex,
            lineIndex: draft.lineIndex,
          },
          reloadAt: draft.pageIndex,
          toast: 'Text removed',
        });
      } else if (draft.sourceRect) {
        result = await runStructuralEdit(deps, {
          command: 'delete_text_region',
          args: {
            path: session.filePath,
            pageIndex: draft.pageIndex,
            sourceRect: rectToPdfRect(draft.sourceRect),
          },
          reloadAt: draft.pageIndex,
          toast: 'Text removed',
        });
      } else {
        pdfEdit.onCancel();
        return;
      }
      if (result !== undefined) pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  const deleteParagraph = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.paragraphDraft) return;
      const draft = pdfEdit.paragraphDraft;
      const result = await runStructuralEdit(deps, {
        command: 'delete_paragraph',
        args: {
          path: session.filePath,
          pageIndex: draft.pageIndex,
          lineIndices: draft.lineIndices,
        },
        reloadAt: draft.pageIndex,
        toast: 'Paragraph removed',
      });
      if (result !== undefined) pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  const applyImageEdit = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.imageDraft) return;
      const draft = pdfEdit.imageDraft;
      const newRect = await viewerRectToPdf(draft.pageIndex, draft.pageRect);
      const result = await runStructuralEdit(deps, {
        command: 'transform_page_image',
        args: {
          path: session.filePath,
          pageIndex: draft.pageIndex,
          imageIndex: draft.index,
          newRect,
          rotation: draft.rotation ?? 0,
        },
        reloadAt: draft.pageIndex,
        toast: 'Image updated',
      });
      if (result !== undefined) pdfEdit.onCancel();
    },
    [deps, pdfEdit, viewerRectToPdf],
  );

  const deleteImage = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.imageDraft) return;
      const draft = pdfEdit.imageDraft;
      const result = await runStructuralEdit(deps, {
        command: 'remove_page_image',
        args: {
          path: session.filePath,
          pageIndex: draft.pageIndex,
          imageIndex: draft.index,
        },
        reloadAt: draft.pageIndex,
        toast: 'Image removed',
      });
      if (result !== undefined) pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  return {
    handlePageClick,
    applyTextEdit,
    applyParagraphEdit,
    deleteText,
    deleteParagraph,
    applyImageEdit,
    deleteImage,
    hitTestImage,
  };
}

import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DocumentSessionData } from '../app/documentSessionTypes';
import type { PdfEditState, Rect, TextStyle } from '../app/usePdfEditState';
import { runStructuralEdit, type StructuralEditDeps } from '../pdf/runStructuralEdit';

type PdfRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type PageImageInfo = {
  index: number;
  bbox: PdfRect;
  width?: number;
  height?: number;
};

type PageImageHit = {
  index: number;
  viewerRect: Rect;
  width?: number;
  height?: number;
};

type TextLine = {
  lineIndex: number;
  text: string;
  bbox: Rect;
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
      const lines = await invoke<Array<{ text: string; x: number; y: number; w: number; h: number }>>(
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
        const rect = await pdfRectToViewerPx(pageIndex, img.bbox);
        if (
          x >= rect.x &&
          x <= rect.x + rect.w &&
          y >= rect.y &&
          y <= rect.y + rect.h
        ) {
          return { index: img.index, viewerRect: rect, width: img.width, height: img.height };
        }
      }
      return null;
    },
    [filePath, pdfRectToViewerPx],
  );

  const handlePageClick = useCallback(
    async (
      pageIndex: number,
      point: { x: number; y: number },
      session: DocumentSessionData,
      hitTestImageFn?: (pageIndex: number, x: number, y: number) => Promise<PageImageHit | null>,
    ) => {
      if (!session?.filePath) return;

      if (pdfEdit.mode === 'text' || pdfEdit.mode === 'idle') {
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
        if (hitLine) {
          pdfEdit.startEditingText({
            pageIndex,
            point,
            text: hitLine.text,
            pageRect: hitLine.bbox,
            style: pdfEdit.style,
            lineIndex: hitLine.lineIndex,
          });
        } else if (pdfEdit.mode === 'text') {
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
          });
        }
      }
    },
    [pdfEdit, loadPageTextLines],
  );

  const applyTextEdit = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.textDraft) return;
      const draft = pdfEdit.textDraft;
      const boxRect = rectToPdfRect(draft.pageRect);
      const style = toBackendStyle(draft.style);

      if (draft.lineIndex !== undefined) {
        await runStructuralEdit(deps, {
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
      } else {
        await runStructuralEdit(deps, {
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
      pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  const applyImageEdit = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.imageDraft) return;
      const draft = pdfEdit.imageDraft;
      const newRect = await viewerRectToPdf(draft.pageIndex, draft.pageRect);
      await runStructuralEdit(deps, {
        command: 'transform_page_image',
        args: {
          path: session.filePath,
          pageIndex: draft.pageIndex,
          imageIndex: draft.index,
          newRect,
        },
        reloadAt: draft.pageIndex,
        toast: 'Image updated',
      });
      pdfEdit.onCancel();
    },
    [deps, pdfEdit, viewerRectToPdf],
  );

  const deleteImage = useCallback(
    async (session: DocumentSessionData) => {
      if (!session?.filePath || !pdfEdit.imageDraft) return;
      const draft = pdfEdit.imageDraft;
      await runStructuralEdit(deps, {
        command: 'remove_page_image',
        args: {
          path: session.filePath,
          pageIndex: draft.pageIndex,
          imageIndex: draft.index,
        },
        reloadAt: draft.pageIndex,
        toast: 'Image removed',
      });
      pdfEdit.onCancel();
    },
    [deps, pdfEdit],
  );

  return {
    handlePageClick,
    applyTextEdit,
    applyImageEdit,
    deleteImage,
    hitTestImage,
  };
}

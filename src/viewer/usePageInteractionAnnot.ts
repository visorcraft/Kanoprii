import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { StampKind } from '../app/constants';
import type { AnnotationData } from '../app/types';
import { runAnnotationRemoveViaEdit, type AnnotationRemoveCommand } from '../pdf/runAnnotationEdit';
import type { createStructuralEditRunner } from '../pdf/runStructuralEdit';

export type PageInteractionAnnotOptions = {
  filePath: string;
  currentPage: number;
  runEdit: ReturnType<typeof createStructuralEditRunner>;
  setAnnotations: (annots: AnnotationData[]) => void;
};

export function usePageInteractionAnnot(opts: PageInteractionAnnotOptions) {
  const refreshAnnotations = useCallback(async () => {
    const annots = await invoke<AnnotationData[]>('get_annotations', {
      path: opts.filePath,
      pageIndex: opts.currentPage,
    });
    opts.setAnnotations(annots);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath, opts.currentPage, opts.setAnnotations]);

  const removeAnnotation = useCallback((command: AnnotationRemoveCommand, index: number, toast: string) => {
    runAnnotationRemoveViaEdit(opts.runEdit, refreshAnnotations, command, opts.currentPage, index, toast);
  }, [opts.runEdit, refreshAnnotations, opts.currentPage]);

  const removeRedaction = useCallback((index: number) => {
    removeAnnotation('remove_redaction', index, 'Redaction removed');
  }, [removeAnnotation]);

  const removeStamp = useCallback((kind: StampKind, index: number) => {
    const command = kind === 'text' ? 'remove_text_stamp' : 'remove_image_stamp';
    removeAnnotation(command, index, 'Stamp removed');
  }, [removeAnnotation]);

  const removeShape = useCallback((subtype: 'Square' | 'Circle' | 'Line', index: number) => {
    const command = subtype === 'Square' ? 'remove_square' : subtype === 'Circle' ? 'remove_circle' : 'remove_line';
    removeAnnotation(command, index, 'Shape removed');
  }, [removeAnnotation]);

  const removeInkStroke = useCallback((inkIndex: number) => {
    removeAnnotation('remove_ink_stroke', inkIndex, 'Drawing removed');
  }, [removeAnnotation]);

  const removeHighlight = useCallback((highlightIndex: number) => {
    removeAnnotation('remove_highlight', highlightIndex, 'Highlight removed');
  }, [removeAnnotation]);

  const removeHighlightOnPage = useCallback((pageIndex: number, index: number) => {
    runAnnotationRemoveViaEdit(opts.runEdit, refreshAnnotations, 'remove_highlight', pageIndex, index, 'Highlight removed');
  }, [opts.runEdit, refreshAnnotations]);

  const removeTextNote = useCallback((noteIndex: number) => {
    removeAnnotation('remove_text_note', noteIndex, 'Note removed');
  }, [removeAnnotation]);

  const removeTextNoteOnPage = useCallback((pageIndex: number, index: number) => {
    runAnnotationRemoveViaEdit(opts.runEdit, refreshAnnotations, 'remove_text_note', pageIndex, index, 'Note removed');
  }, [opts.runEdit, refreshAnnotations]);

  const removeRedactionOnPage = useCallback((pageIndex: number, index: number) => {
    runAnnotationRemoveViaEdit(opts.runEdit, refreshAnnotations, 'remove_redaction', pageIndex, index, 'Redaction removed');
  }, [opts.runEdit, refreshAnnotations]);

  const commitInkStroke = useCallback((points: number[]) => {
    if (points.length < 4) return;
    void opts.runEdit({
      command: 'add_ink_stroke',
      args: { pageIndex: opts.currentPage, points },
      afterEdit: async () => { await refreshAnnotations(); },
      toast: 'Drawing added',
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.runEdit, opts.currentPage, refreshAnnotations]);

  return {
    refreshAnnotations,
    removeRedaction,
    removeStamp,
    removeShape,
    removeInkStroke,
    removeHighlight,
    removeHighlightOnPage,
    removeTextNote,
    removeTextNoteOnPage,
    removeRedactionOnPage,
    commitInkStroke,
  };
}

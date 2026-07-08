import type { RunStructuralEditOptions } from './runStructuralEdit';

export type AnnotationRemoveCommand =
  | 'remove_highlight'
  | 'remove_ink_stroke'
  | 'remove_redaction'
  | 'remove_text_note'
  | 'remove_text_stamp'
  | 'remove_image_stamp'
  | 'remove_square'
  | 'remove_circle'
  | 'remove_line';

type StructuralEditRunner = <T = unknown>(
  options: RunStructuralEditOptions<T>,
) => Promise<T | undefined>;

export function runAnnotationRemoveViaEdit(
  runEdit: StructuralEditRunner,
  refreshAnnotations: () => Promise<void>,
  command: AnnotationRemoveCommand,
  pageIndex: number,
  index: number,
  toast: string,
): void {
  void runEdit({
    command,
    args: { pageIndex, index },
    afterEdit: async () => { await refreshAnnotations(); },
    toast,
  });
}

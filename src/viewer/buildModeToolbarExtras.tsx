import type { ShapeKind, StampKind } from '../app/constants';
import type { PdfEditState } from '../app/usePdfEditState';
import { ModeToolbarExtras } from './ModeToolbarExtras';

export type BuildModeToolbarExtrasInput = {
  filePath: string;
  pdfEdit: PdfEditState;
  onToggleEditMode: () => void;
  editTextRunMode: boolean;
  onToggleEditTextRunMode: () => void;
  onBeginTextInsert: () => void;
  onInsertEditImage?: () => void;
  vectorEditMode: boolean;
  onToggleVectorEditMode: () => void;
  imageInsertMode: boolean;
  imageSourcePath: string;
  onOpenImageInsertModal: () => void;
  stampMode: boolean;
  stampKind: StampKind;
  stampPreset: string;
  onStampKindChange: (kind: StampKind) => void;
  onStampPresetChange: (preset: string) => void;
  shapeMode: boolean;
  shapeKind: ShapeKind;
  onShapeKindChange: (kind: ShapeKind) => void;
};

export function buildModeToolbarExtras(input: BuildModeToolbarExtrasInput) {
  if (!input.filePath) return null;
  return (
    <ModeToolbarExtras
      pdfEdit={input.pdfEdit}
      onToggleEditMode={input.onToggleEditMode}
      editTextRunMode={input.editTextRunMode}
      onToggleEditTextRunMode={input.onToggleEditTextRunMode}
      onBeginTextInsert={input.onBeginTextInsert}
      onInsertEditImage={input.onInsertEditImage}
      vectorEditMode={input.vectorEditMode}
      onToggleVectorEditMode={input.onToggleVectorEditMode}
      imageInsertMode={input.imageInsertMode}
      imageSourcePath={input.imageSourcePath}
      onOpenImageInsertModal={input.onOpenImageInsertModal}
      stampMode={input.stampMode}
      stampKind={input.stampKind}
      stampPreset={input.stampPreset}
      onStampKindChange={input.onStampKindChange}
      onStampPresetChange={input.onStampPresetChange}
      shapeMode={input.shapeMode}
      shapeKind={input.shapeKind}
      onShapeKindChange={input.onShapeKindChange}
    />
  );
}

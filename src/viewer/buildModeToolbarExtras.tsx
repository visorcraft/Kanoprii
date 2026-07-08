import type { ShapeKind, StampKind } from '../app/constants';
import { ModeToolbarExtras } from './ModeToolbarExtras';

export type BuildModeToolbarExtrasInput = {
  filePath: string;
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

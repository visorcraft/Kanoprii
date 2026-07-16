import type { ReactNode } from 'react';
import type { ShapeKind, StampKind } from '../app/constants';
import type { PdfEditState } from '../app/usePdfEditState';
import { AnnotateToolOptions } from './AnnotateToolOptions';
import { EditRibbonTab } from './EditRibbonTab';

export type BuildRibbonTabExtrasInput = {
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
  onOpenPageEditsModal: () => void;
  stampMode: boolean;
  stampKind: StampKind;
  stampPreset: string;
  onStampKindChange: (kind: StampKind) => void;
  onStampPresetChange: (preset: string) => void;
  shapeMode: boolean;
  shapeKind: ShapeKind;
  onShapeKindChange: (kind: ShapeKind) => void;
};

export type RibbonTabExtras = {
  editTab: ReactNode;
  annotateOptions: ReactNode;
};

export function buildRibbonTabExtras(input: BuildRibbonTabExtrasInput): RibbonTabExtras {
  return {
    editTab: (
      <EditRibbonTab
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
        onOpenPageEditsModal={input.onOpenPageEditsModal}
      />
    ),
    annotateOptions: (
      <AnnotateToolOptions
        stampMode={input.stampMode}
        stampKind={input.stampKind}
        stampPreset={input.stampPreset}
        onStampKindChange={input.onStampKindChange}
        onStampPresetChange={input.onStampPresetChange}
        shapeMode={input.shapeMode}
        shapeKind={input.shapeKind}
        onShapeKindChange={input.onShapeKindChange}
      />
    ),
  };
}

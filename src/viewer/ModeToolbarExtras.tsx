import { STAMP_PRESETS, type ShapeKind, type StampKind } from '../app/constants';
import { fileNameFromPath } from '../app/utils';

type ModeToolbarExtrasProps = {
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

export function ModeToolbarExtras({
  imageInsertMode,
  imageSourcePath,
  onOpenImageInsertModal,
  stampMode,
  stampKind,
  stampPreset,
  onStampKindChange,
  onStampPresetChange,
  shapeMode,
  shapeKind,
  onShapeKindChange,
}: ModeToolbarExtrasProps) {
  return (
    <>
      {imageInsertMode && imageSourcePath && (
        <button
          type="button"
          onClick={onOpenImageInsertModal}
          className="btn"
          title="Change source image"
        >
          {fileNameFromPath(imageSourcePath)}
        </button>
      )}
      {stampMode && (
        <div className="stamp-toolbar" role="group" aria-label="Stamp options">
          <div className="shape-kind-toggle" role="group" aria-label="Stamp kind">
            <button
              type="button"
              className={stampKind === 'text' ? 'active' : ''}
              onClick={() => onStampKindChange('text')}
            >
              Text
            </button>
            <button
              type="button"
              className={stampKind === 'image' ? 'active' : ''}
              onClick={() => onStampKindChange('image')}
            >
              Image
            </button>
          </div>
          <select
            className="stamp-preset-select"
            value={stampPreset}
            onChange={(e) => onStampPresetChange(e.target.value)}
            aria-label="Stamp preset"
          >
            {STAMP_PRESETS.map((preset) => (
              <option key={preset.id} value={preset.id}>{preset.label}</option>
            ))}
          </select>
        </div>
      )}
      {shapeMode && (
        <div className="shape-kind-toggle" role="group" aria-label="Shape kind">
          <button
            type="button"
            className={shapeKind === 'square' ? 'active' : ''}
            onClick={() => onShapeKindChange('square')}
          >
            Rect
          </button>
          <button
            type="button"
            className={shapeKind === 'circle' ? 'active' : ''}
            onClick={() => onShapeKindChange('circle')}
          >
            Ellipse
          </button>
          <button
            type="button"
            className={shapeKind === 'line' ? 'active' : ''}
            onClick={() => onShapeKindChange('line')}
          >
            Line
          </button>
        </div>
      )}
    </>
  );
}

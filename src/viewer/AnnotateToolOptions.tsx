import { STAMP_PRESETS, type ShapeKind, type StampKind } from '../app/constants';
import { ShapeKindPicker } from './ShapeKindPicker';

export type AnnotateToolOptionsProps = {
  stampMode: boolean;
  stampKind: StampKind;
  stampPreset: string;
  onStampKindChange: (kind: StampKind) => void;
  onStampPresetChange: (preset: string) => void;
  shapeMode: boolean;
  shapeKind: ShapeKind;
  onShapeKindChange: (kind: ShapeKind) => void;
};

export function AnnotateToolOptions({
  stampMode,
  stampKind,
  stampPreset,
  onStampKindChange,
  onStampPresetChange,
  shapeMode,
  shapeKind,
  onShapeKindChange,
}: AnnotateToolOptionsProps) {
  if (!stampMode && !shapeMode) return null;
  return (
    <div className="ribbon-extras annotate-tool-options">
      {stampMode && (
        <div className="stamp-toolbar" role="group" aria-label="Stamp options">
          <div className="shape-kind-toggle" role="group" aria-label="Stamp kind">
            <button type="button" className={stampKind === 'text' ? 'active' : ''} onClick={() => onStampKindChange('text')}>
              Text
            </button>
            <button type="button" className={stampKind === 'image' ? 'active' : ''} onClick={() => onStampKindChange('image')}>
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
        <ShapeKindPicker value={shapeKind} onChange={onShapeKindChange} ariaLabel="Shape kind" />
      )}
    </div>
  );
}

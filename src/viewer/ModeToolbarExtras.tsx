import { STAMP_PRESETS, type ShapeKind, type StampKind } from '../app/constants';
import type { PdfEditState, TextStyle } from '../app/usePdfEditState';
import { fileNameFromPath } from '../app/utils';
import { EditToolbar } from './EditToolbar';

type ModeToolbarExtrasProps = {
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

export function ModeToolbarExtras({
  pdfEdit,
  onToggleEditMode,
  editTextRunMode,
  onToggleEditTextRunMode,
  onBeginTextInsert,
  onInsertEditImage,
  vectorEditMode,
  onToggleVectorEditMode,
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
  const textDraft = pdfEdit.textDraft ?? (pdfEdit.paragraphEditing ? pdfEdit.paragraphDraft : null);
  const updateTextStyle = (patch: Partial<TextStyle>) => {
    if (!textDraft) return;
    const style = { ...textDraft.style, ...patch };
    if (pdfEdit.textDraft) pdfEdit.onUpdate({ style });
    else pdfEdit.onUpdateParagraph({ style });
  };
  const rotateImage = (degrees: number) => {
    const rotation = ((pdfEdit.imageDraft?.rotation ?? 0) + degrees + 360) % 360;
    pdfEdit.onUpdateImageRotation(rotation);
  };

  return (
    <div className="mode-toolbar-extras">
      <div className="pdf-edit-ribbon" role="toolbar" aria-label="Edit PDF toolbar" data-testid="pdf-edit-toolbar">
        <div className="pdf-edit-ribbon-tools">
          <span className="pdf-edit-ribbon-title">Edit PDF</span>
          <button
            type="button"
            className={editTextRunMode ? 'btn btn-active' : 'btn'}
            aria-pressed={editTextRunMode}
            onClick={onToggleEditTextRunMode}
          >
            Edit Text
          </button>
          <button
            type="button"
            className={pdfEdit.editMode ? 'btn btn-active' : 'btn'}
            aria-pressed={pdfEdit.editMode}
            aria-label="Edit mode"
            title={pdfEdit.editMode ? 'Edit PDF content (on)' : 'Edit PDF content'}
            onClick={onToggleEditMode}
          >
            Edit Objects
          </button>
          <button
            type="button"
            className={pdfEdit.editMode && pdfEdit.mode === 'text' && !pdfEdit.textDraft ? 'btn btn-active' : 'btn'}
            aria-pressed={pdfEdit.editMode && pdfEdit.mode === 'text' && !pdfEdit.textDraft}
            onClick={onBeginTextInsert}
          >
            Add Text
          </button>
          {onInsertEditImage && (
            <button type="button" className="btn" aria-label="Add image" onClick={onInsertEditImage}>
              Add Image
            </button>
          )}
          <button
            type="button"
            className={vectorEditMode ? 'btn btn-active' : 'btn'}
            aria-pressed={vectorEditMode}
            onClick={onToggleVectorEditMode}
          >
            Edit Vector
          </button>
          {editTextRunMode && <span className="pdf-edit-ribbon-hint">Click existing text to edit.</span>}
          {pdfEdit.editMode && !pdfEdit.textDraft && !pdfEdit.paragraphDraft && !pdfEdit.imageDraft && (
            <span className="pdf-edit-ribbon-hint">
              Click text or an image to edit. Click empty space to add text.
            </span>
          )}
        </div>

        {textDraft && (
          <div className="pdf-edit-context">
            <span className="pdf-edit-context-label">Text</span>
            <EditToolbar
              style={textDraft.style}
              onChange={updateTextStyle}
              onApply={pdfEdit.onApply}
              onCancel={pdfEdit.onCancel}
              onDelete={pdfEdit.textDraft ? pdfEdit.onDeleteText : pdfEdit.onDeleteParagraph}
            />
          </div>
        )}

        {pdfEdit.paragraphDraft && !pdfEdit.paragraphEditing && (
          <div className="pdf-edit-context" role="toolbar" aria-label="Paragraph editing toolbar">
            <span className="pdf-edit-context-label">Paragraph</span>
            <button type="button" className="btn" onClick={pdfEdit.enterParagraphTextEdit}>Edit Text</button>
            <button type="button" className="btn danger" onClick={pdfEdit.onDeleteParagraph}>Delete</button>
            <button type="button" className="btn" onClick={pdfEdit.onCancel}>Done</button>
          </div>
        )}

        {pdfEdit.imageDraft && (
          <div className="pdf-edit-context" role="toolbar" aria-label="Image editing toolbar">
            <span className="pdf-edit-context-label">Image</span>
            <button type="button" className="btn" onClick={() => rotateImage(90)}>Rotate Left</button>
            <button type="button" className="btn" onClick={() => rotateImage(-90)}>Rotate Right</button>
            <button type="button" className="btn" onClick={pdfEdit.onReplaceImage}>Replace</button>
            <button type="button" className="btn primary" onClick={pdfEdit.onApply}>Apply</button>
            <button type="button" className="btn danger" onClick={pdfEdit.onDeleteImage}>Delete</button>
            <button type="button" className="btn" onClick={pdfEdit.onCancel}>Cancel</button>
          </div>
        )}
      </div>

      {imageInsertMode && imageSourcePath && (
        <button type="button" onClick={onOpenImageInsertModal} className="btn" title="Change source image">
          {fileNameFromPath(imageSourcePath)}
        </button>
      )}
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
        <div className="shape-kind-toggle" role="group" aria-label="Shape kind">
          <button type="button" className={shapeKind === 'square' ? 'active' : ''} onClick={() => onShapeKindChange('square')}>
            Rect
          </button>
          <button type="button" className={shapeKind === 'circle' ? 'active' : ''} onClick={() => onShapeKindChange('circle')}>
            Ellipse
          </button>
          <button type="button" className={shapeKind === 'line' ? 'active' : ''} onClick={() => onShapeKindChange('line')}>
            Line
          </button>
        </div>
      )}
    </div>
  );
}

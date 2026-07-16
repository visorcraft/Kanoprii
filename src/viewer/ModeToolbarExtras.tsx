import { STAMP_PRESETS, type ShapeKind, type StampKind } from '../app/constants';
import type { PdfEditState, TextStyle } from '../app/usePdfEditState';
import { fileNameFromPath } from '../app/utils';
import { EditToolbar } from './EditToolbar';
import type { ReactNode } from 'react';

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

type EditToolIconName =
  | 'text' | 'objects' | 'add-text' | 'image' | 'vector'
  | 'rotate-left' | 'rotate-right' | 'replace' | 'delete' | 'apply' | 'close';

function EditToolIcon({ name, className = 'pdf-edit-tool-icon' }: { name: EditToolIconName; className?: string }) {
  const paths: Record<EditToolIconName, ReactNode> = {
    text: <><path d="M6 5h12M12 5v14M9 19h6"/><path d="m17 13 3-3 2 2-3 3-3 1z"/></>,
    objects: <><rect x="5" y="5" width="14" height="14" rx="1"/><path d="M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5"/></>,
    'add-text': <><path d="M4 5h11M9.5 5v14M7 19h5"/><path d="M19 12v8M15 16h8"/></>,
    image: <><rect x="3" y="5" width="15" height="14" rx="2"/><circle cx="8" cy="10" r="1.5"/><path d="m5 17 4-4 3 3 2-2 4 4M21 7v8M17 11h8"/></>,
    vector: <><path d="m5 19 4-10 10-4-4 10zM9 9l6 6"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="5" r="2"/></>,
    'rotate-left': <><path d="M4 8V3m0 0h5M4 3l4 4"/><path d="M5 13a7 7 0 1 0 3-6"/></>,
    'rotate-right': <><path d="M20 8V3m0 0h-5m5 0-4 4"/><path d="M19 13a7 7 0 1 1-3-6"/></>,
    replace: <><rect x="4" y="5" width="13" height="13" rx="2"/><path d="m6 16 3-3 2 2 2-2 4 4M19 8h3m0 0-2-2m2 2-2 2"/></>,
    delete: <><path d="M4 7h16M9 7V4h6v3M7 7l1 14h8l1-14M10 11v6M14 11v6"/></>,
    apply: <path d="m5 12 4 4L19 6"/>,
    close: <path d="m6 6 12 12M18 6 6 18"/>,
  };
  return <svg className={className} viewBox="0 0 24 24" aria-hidden="true">{paths[name]}</svg>;
}

function EditTool({
  label,
  icon,
  active = false,
  onClick,
  ariaLabel,
}: {
  label: string;
  icon: EditToolIconName;
  active?: boolean;
  onClick: () => void;
  ariaLabel?: string;
}) {
  return (
    <button
      type="button"
      className={`pdf-edit-tool${active ? ' active' : ''}`}
      aria-pressed={active}
      aria-label={ariaLabel}
      onClick={onClick}
      title={label}
    >
      <EditToolIcon name={icon} />
      <span>{label}</span>
    </button>
  );
}

function ContextAction({
  label,
  icon,
  onClick,
  tone = 'default',
}: {
  label: string;
  icon: EditToolIconName;
  onClick: () => void;
  tone?: 'default' | 'primary' | 'danger';
}) {
  return (
    <button type="button" className={`pdf-edit-context-action ${tone}`} onClick={onClick} title={label}>
      <EditToolIcon name={icon} className="pdf-edit-context-icon" />
      <span>{label}</span>
    </button>
  );
}

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
  const addingText =
    pdfEdit.editMode &&
    pdfEdit.mode === 'text' &&
    (!pdfEdit.textDraft ||
      (pdfEdit.textDraft.lineIndex === undefined && pdfEdit.textDraft.sourceRect === undefined));

  return (
    <div className="mode-toolbar-extras">
      <div className="pdf-edit-ribbon" role="toolbar" aria-label="Edit PDF toolbar" data-testid="pdf-edit-toolbar">
        <div className="pdf-edit-ribbon-tools">
          <div className="pdf-edit-ribbon-title"><strong>Edit</strong><span>PDF</span></div>
          <div className="pdf-edit-tool-group">
            <div className="pdf-edit-tool-row">
              <EditTool label="Edit Text" icon="text" active={editTextRunMode} onClick={onToggleEditTextRunMode} />
              <EditTool label="Edit Objects" icon="objects" active={pdfEdit.editMode && !addingText} onClick={onToggleEditMode} ariaLabel="Edit mode" />
            </div>
            <span className="pdf-edit-tool-group-label">Edit</span>
          </div>
          <div className="pdf-edit-tool-group">
            <div className="pdf-edit-tool-row">
              <EditTool label="Add Text" icon="add-text" active={addingText} onClick={onBeginTextInsert} />
              {onInsertEditImage && <EditTool label="Add Image" icon="image" onClick={onInsertEditImage} ariaLabel="Add image" />}
            </div>
            <span className="pdf-edit-tool-group-label">Insert</span>
          </div>
          <div className="pdf-edit-tool-group">
            <div className="pdf-edit-tool-row">
              <EditTool label="Edit Vector" icon="vector" active={vectorEditMode} onClick={onToggleVectorEditMode} />
            </div>
            <span className="pdf-edit-tool-group-label">Graphics</span>
          </div>
          <div className="pdf-edit-ribbon-hint" aria-live="polite">
            {editTextRunMode && 'Click existing text to edit.'}
            {pdfEdit.editMode && !pdfEdit.textDraft && !pdfEdit.paragraphDraft && !pdfEdit.imageDraft &&
              'Click text or an image to edit. Click empty space to add text.'}
            {vectorEditMode && !pdfEdit.vectorDraft && 'Click a vector to edit, or drag empty space to draw a rectangle.'}
            {!editTextRunMode && !pdfEdit.editMode && !vectorEditMode && 'Choose a tool, then click the page.'}
          </div>
        </div>

        {textDraft && (
          <div className="pdf-edit-context">
            <span className="pdf-edit-context-label">Text format</span>
            <EditToolbar
              style={textDraft.style}
              onChange={updateTextStyle}
              onApply={pdfEdit.onApply}
              onCancel={pdfEdit.onCancel}
              onDelete={
                pdfEdit.textDraft
                  ? pdfEdit.textDraft.lineIndex !== undefined || pdfEdit.textDraft.sourceRect
                    ? pdfEdit.onDeleteText
                    : undefined
                  : pdfEdit.onDeleteParagraph
              }
            />
          </div>
        )}

        {pdfEdit.paragraphDraft && !pdfEdit.paragraphEditing && (
          <div className="pdf-edit-context" role="toolbar" aria-label="Paragraph editing toolbar">
            <span className="pdf-edit-context-label">Paragraph</span>
            <div className="pdf-edit-context-group">
              <ContextAction label="Edit Text" icon="text" onClick={pdfEdit.enterParagraphTextEdit} />
              <span className="pdf-edit-context-group-name">Content</span>
            </div>
            <span className="pdf-edit-context-help">Drag the handles to move or resize the paragraph.</span>
            <div className="pdf-edit-context-actions">
              <ContextAction label="Delete" icon="delete" onClick={pdfEdit.onDeleteParagraph} tone="danger" />
              <ContextAction label="Done" icon="apply" onClick={pdfEdit.onCancel} tone="primary" />
            </div>
          </div>
        )}

        {pdfEdit.imageDraft && (
          <div className="pdf-edit-context" role="toolbar" aria-label="Image editing toolbar">
            <span className="pdf-edit-context-label">Image</span>
            <div className="pdf-edit-context-group">
              <div className="pdf-edit-context-group-row">
                <ContextAction label="Rotate Left" icon="rotate-left" onClick={() => rotateImage(90)} />
                <ContextAction label="Rotate Right" icon="rotate-right" onClick={() => rotateImage(-90)} />
              </div>
              <span className="pdf-edit-context-group-name">Transform</span>
            </div>
            <div className="pdf-edit-context-group">
              <ContextAction label="Replace" icon="replace" onClick={pdfEdit.onReplaceImage} />
              <span className="pdf-edit-context-group-name">Image</span>
            </div>
            <span className="pdf-edit-context-help">Drag handles to move, resize, or rotate.</span>
            <div className="pdf-edit-context-actions">
              <ContextAction label="Delete" icon="delete" onClick={pdfEdit.onDeleteImage} tone="danger" />
              <ContextAction label="Cancel" icon="close" onClick={pdfEdit.onCancel} />
              <ContextAction label="Apply" icon="apply" onClick={pdfEdit.onApply} tone="primary" />
            </div>
          </div>
        )}

        {pdfEdit.vectorDraft && (
          <div className="pdf-edit-context" role="toolbar" aria-label="Vector editing toolbar">
            <span className="pdf-edit-context-label">Vector</span>
            <span className="pdf-edit-context-help">Drag the shape or its handles to move and resize.</span>
            <div className="pdf-edit-context-actions">
              <ContextAction label="Delete" icon="delete" onClick={pdfEdit.onDeleteVector} tone="danger" />
              <ContextAction label="Cancel" icon="close" onClick={pdfEdit.onCancel} />
              <ContextAction label="Apply" icon="apply" onClick={pdfEdit.onApply} tone="primary" />
            </div>
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

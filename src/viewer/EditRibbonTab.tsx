import type { ReactNode } from 'react';
import type { ShapeKind } from '../app/constants';
import type { PdfEditState, RgbColor, TextStyle } from '../app/usePdfEditState';
import { fileNameFromPath } from '../app/utils';
import { EditToolbar } from './EditToolbar';
import { ShapeKindPicker } from './ShapeKindPicker';

export type EditRibbonTabProps = {
  pdfEdit: PdfEditState;
  onToggleEditMode: () => void;
  editTextRunMode: boolean;
  onToggleEditTextRunMode: () => void;
  onBeginTextInsert: () => void;
  onInsertEditImage?: () => void;
  vectorEditMode: boolean;
  onToggleVectorEditMode: () => void;
  shapeKind: ShapeKind;
  onShapeKindChange: (kind: ShapeKind) => void;
  imageInsertMode: boolean;
  imageSourcePath: string;
  onOpenImageInsertModal: () => void;
  onOpenPageEditsModal: () => void;
};

type EditToolIconName =
  | 'text' | 'objects' | 'add-text' | 'image' | 'vector' | 'manage'
  | 'rotate-left' | 'rotate-right' | 'replace' | 'delete' | 'apply' | 'close';

function EditToolIcon({ name, className = 'pdf-edit-tool-icon' }: { name: EditToolIconName; className?: string }) {
  const paths: Record<EditToolIconName, ReactNode> = {
    text: <><path d="M6 5h12M12 5v14M9 19h6"/><path d="m17 13 3-3 2 2-3 3-3 1z"/></>,
    objects: <><rect x="5" y="5" width="14" height="14" rx="1"/><path d="M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5"/></>,
    'add-text': <><path d="M4 5h11M9.5 5v14M7 19h5"/><path d="M19 12v8M15 16h8"/></>,
    image: <><rect x="3" y="5" width="15" height="14" rx="2"/><circle cx="8" cy="10" r="1.5"/><path d="m5 17 4-4 3 3 2-2 4 4M21 7v8M17 11h8"/></>,
    vector: <><path d="m5 19 4-10 10-4-4 10zM9 9l6 6"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="5" r="2"/></>,
    manage: <><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4z"/></>,
    'rotate-left': <><path d="M4 8V3m0 0h5M4 3l4 4"/><path d="M5 13a7 7 0 1 0 3-6"/></>,
    'rotate-right': <><path d="M20 8V3m0 0h-5m5 0-4 4"/><path d="M19 13a7 7 0 1 1-3-6"/></>,
    replace: <><rect x="4" y="5" width="13" height="13" rx="2"/><path d="m6 16 3-3 2 2 2-2 4 4M19 8h3m0 0-2-2m2 2-2 2"/></>,
    delete: <path d="M4 7h16M9 7V4h6v3M7 7l1 14h8l1-14M10 11v6M14 11v6"/>,
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
  testId,
}: {
  label: string;
  icon: EditToolIconName;
  active?: boolean;
  onClick: () => void;
  ariaLabel?: string;
  testId?: string;
}) {
  return (
    <button
      type="button"
      className={`pdf-edit-tool${active ? ' active' : ''}`}
      aria-pressed={active}
      aria-label={ariaLabel}
      onClick={onClick}
      title={label}
      data-testid={testId}
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

function colorToHex(color: RgbColor): string {
  const part = (value: number) => Math.max(0, Math.min(255, Math.round(value))).toString(16).padStart(2, '0');
  return `#${part(color.r)}${part(color.g)}${part(color.b)}`;
}

function hexToRgb(value: string): RgbColor {
  const hex = value.replace('#', '');
  const int = Number.parseInt(hex, 16);
  if (Number.isNaN(int)) return { r: 255, g: 0, b: 0 };
  return {
    r: (int >> 16) & 255,
    g: (int >> 8) & 255,
    b: int & 255,
  };
}

export function EditRibbonTab({
  pdfEdit,
  onToggleEditMode,
  editTextRunMode,
  onToggleEditTextRunMode,
  onBeginTextInsert,
  onInsertEditImage,
  vectorEditMode,
  onToggleVectorEditMode,
  shapeKind,
  onShapeKindChange,
  imageInsertMode,
  imageSourcePath,
  onOpenImageInsertModal,
  onOpenPageEditsModal,
}: EditRibbonTabProps) {
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
    <div className="edit-ribbon-tab" data-testid="pdf-edit-toolbar">
      <div className="edit-ribbon-tools">
        <div className="ribbon-group pdf-edit-tool-group" role="group" aria-label="Edit">
          <div className="ribbon-group-controls">
            <EditTool label="Edit Text" icon="text" active={editTextRunMode} onClick={onToggleEditTextRunMode} testId="edit-text" />
            <EditTool label="Edit Objects" icon="objects" active={pdfEdit.editMode && !addingText} onClick={onToggleEditMode} ariaLabel="Edit mode" testId="pdf-edit" />
          </div>
          <span className="ribbon-group-label">Edit</span>
        </div>
        <div className="ribbon-group pdf-edit-tool-group" role="group" aria-label="Insert">
          <div className="ribbon-group-controls">
            <EditTool label="Add Text" icon="add-text" active={addingText} onClick={onBeginTextInsert} testId="page-text" />
            {onInsertEditImage && <EditTool label="Add Image" icon="image" onClick={onInsertEditImage} ariaLabel="Add image" testId="insert-image" />}
          </div>
          <span className="ribbon-group-label">Insert</span>
        </div>
        <div className="ribbon-group pdf-edit-tool-group" role="group" aria-label="Graphics">
          <div className="ribbon-group-controls">
            <EditTool label="Shapes" icon="vector" active={vectorEditMode} onClick={onToggleVectorEditMode} testId="vector" />
            {vectorEditMode && (
              <ShapeKindPicker
                value={shapeKind}
                onChange={onShapeKindChange}
                ariaLabel="Edit shape kind"
                className="edit-shape-kind-toggle"
              />
            )}
            {vectorEditMode && (
              <div className="shape-kind-toggle edit-shape-style-toggle" role="group" aria-label="Shape appearance">
                <label className="pdf-edit-compact-field" title="Stroke color">
                  <span>Stroke</span>
                  <input
                    type="color"
                    value={colorToHex(pdfEdit.shapeStyle.strokeColor)}
                    onChange={(e) => pdfEdit.updateShapeStyle({ strokeColor: hexToRgb(e.target.value) })}
                    aria-label="Shape stroke color"
                  />
                </label>
                <label className="pdf-edit-compact-field" title="Stroke width">
                  <span>Width</span>
                  <select
                    value={String(pdfEdit.shapeStyle.strokeWidth)}
                    onChange={(e) => pdfEdit.updateShapeStyle({ strokeWidth: Number(e.target.value) })}
                    aria-label="Shape stroke width"
                  >
                    {[1, 2, 3, 4, 6, 8, 12].map((width) => (
                      <option key={width} value={String(width)}>{width}px</option>
                    ))}
                  </select>
                </label>
              </div>
            )}
          </div>
          <span className="ribbon-group-label">Graphics</span>
        </div>
        <div className="ribbon-group pdf-edit-tool-group" role="group" aria-label="Manage">
          <div className="ribbon-group-controls">
            <EditTool label="Manage" icon="manage" onClick={onOpenPageEditsModal} ariaLabel="Manage page edits" testId="edits" />
          </div>
          <span className="ribbon-group-label">Manage</span>
        </div>
        <div className="ribbon-hint" aria-live="polite">
          {editTextRunMode && 'Click existing text to edit.'}
          {pdfEdit.editMode && !pdfEdit.textDraft && !pdfEdit.paragraphDraft && !pdfEdit.imageDraft &&
            (addingText ? 'Click the page to place text.' : 'Click text or an image to select it.')}
          {vectorEditMode && !pdfEdit.vectorDraft && !pdfEdit.shapeDraft && 'Click a legacy vector to edit, or drag empty space to draw a shape. Apply commits it; Cancel discards it.'}
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
          <span className="pdf-edit-context-help">Drag to move/resize. Arrow keys nudge; Shift moves 10 px.</span>
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
          <span className="pdf-edit-context-help">Drag to transform. Arrow keys nudge; Shift moves 10 px.</span>
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
          <span className="pdf-edit-context-help">Drag to move/resize. Arrow keys nudge; Shift moves 10 px.</span>
          <div className="pdf-edit-context-actions">
            <ContextAction label="Delete" icon="delete" onClick={pdfEdit.onDeleteVector} tone="danger" />
            <ContextAction label="Cancel" icon="close" onClick={pdfEdit.onCancel} />
            <ContextAction label="Apply" icon="apply" onClick={pdfEdit.onApply} tone="primary" />
          </div>
        </div>
      )}

      {pdfEdit.shapeDraft && (
        <div className="pdf-edit-context" role="toolbar" aria-label="Shape editing toolbar">
          <span className="pdf-edit-context-label">
            {pdfEdit.shapeDraft.kind === 'circle' ? 'Ellipse' : pdfEdit.shapeDraft.kind === 'line' ? 'Line' : pdfEdit.shapeDraft.kind === 'arrow' ? 'Arrow' : 'Rectangle'}
          </span>
          <span className="pdf-edit-context-help">Drag to move/resize. Arrow keys nudge; Shift moves 10 px. Apply commits the annotation.</span>
          <div className="pdf-edit-context-actions">
            <ContextAction label="Delete" icon="delete" onClick={pdfEdit.onDeleteShape} tone="danger" />
            <ContextAction label="Cancel" icon="close" onClick={pdfEdit.onCancel} />
            <ContextAction label="Apply" icon="apply" onClick={pdfEdit.onApply} tone="primary" />
          </div>
        </div>
      )}

      {imageInsertMode && imageSourcePath && (
        <button type="button" onClick={onOpenImageInsertModal} className="btn" title="Change source image">
          {fileNameFromPath(imageSourcePath)}
        </button>
      )}
    </div>
  );
}

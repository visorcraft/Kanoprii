import type { PageRangeScope } from '../pageRange/types';
import type { RotateDirection } from '../app/useAppModalState';
import { Modal } from '../ui/Modal';

type RotateModalProps = {
  scope: PageRangeScope;
  startPage: number;
  endPage: number;
  pageCount: number | null;
  direction: RotateDirection;
  onClose: () => void;
  onScopeChange: (scope: PageRangeScope) => void;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onDirectionChange: (dir: RotateDirection) => void;
  onApply: () => void;
};

export function RotateModal({
  scope,
  startPage,
  endPage,
  pageCount,
  direction,
  onClose,
  onScopeChange,
  onStartChange,
  onEndChange,
  onDirectionChange,
  onApply,
}: RotateModalProps) {
  const max = pageCount ?? 1;
  const startDisplay = startPage + 1;
  const endDisplay = endPage + 1;
  const invalidRange =
    scope === 'range' &&
    (startPage > endPage ||
      startPage < 0 ||
      endPage >= max ||
      startDisplay < 1 ||
      endDisplay > max);

  const clamp = (value: number) => Math.min(Math.max(value, 1), max);

  return (
    <Modal onClose={onClose}>
      <h3>Rotate Pages</h3>
      <p className="modal-help">
        Choose a scope and direction, then rotate 90°.
      </p>

      <label>Scope</label>
      <div className="rotate-scope-tabs">
        <button
          type="button"
          className={`btn ${scope === 'current' ? 'btn-active' : ''}`}
          onClick={() => onScopeChange('current')}
          aria-pressed={scope === 'current'}
        >
          Current
        </button>
        <button
          type="button"
          className={`btn ${scope === 'all' ? 'btn-active' : ''}`}
          onClick={() => onScopeChange('all')}
          aria-pressed={scope === 'all'}
        >
          All
        </button>
        <button
          type="button"
          className={`btn ${scope === 'range' ? 'btn-active' : ''}`}
          onClick={() => onScopeChange('range')}
          aria-pressed={scope === 'range'}
        >
          Range
        </button>
      </div>

      {scope === 'range' && (
        <div className="rotate-range-row">
          <div>
            <label>From</label>
            <input
              type="number"
              min={1}
              max={max}
              value={startDisplay}
              onChange={(e) =>
                onStartChange(clamp(parseInt(e.target.value || '1', 10)) - 1)
              }
              className="modal-input"
            />
          </div>
          <div>
            <label>To</label>
            <input
              type="number"
              min={1}
              max={max}
              value={endDisplay}
              onChange={(e) =>
                onEndChange(clamp(parseInt(e.target.value || '1', 10)) - 1)
              }
              className="modal-input"
            />
          </div>
        </div>
      )}

      {invalidRange && (
        <p className="modal-error">Please enter a valid page range.</p>
      )}

      <label>Direction</label>
      <div className="rotate-direction-row">
        <button
          type="button"
          className={`rotate-direction-btn ${direction === 'cw' ? 'rotate-direction-active' : ''}`}
          onClick={() => onDirectionChange('cw')}
          aria-pressed={direction === 'cw'}
          aria-label="Rotate 90 degrees clockwise"
        >
          <span className="rotate-direction-icon">↻</span>
          <span>Clockwise</span>
        </button>
        <button
          type="button"
          className={`rotate-direction-btn ${direction === 'ccw' ? 'rotate-direction-active' : ''}`}
          onClick={() => onDirectionChange('ccw')}
          aria-pressed={direction === 'ccw'}
          aria-label="Rotate 90 degrees counter-clockwise"
        >
          <span className="rotate-direction-icon">↺</span>
          <span>Counter-clockwise</span>
        </button>
      </div>

      <div className="rotate-degrees-readout">
        <div className="rotate-degrees-value">90°</div>
        <div className="rotate-degrees-label">rotation</div>
      </div>

      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary" type="button">
          Cancel
        </button>
        <button
          onClick={() => void onApply()}
          className="btn"
          type="button"
          disabled={invalidRange}
          data-testid="rotate-modal-apply"
        >
          Rotate
        </button>
      </div>
    </Modal>
  );
}

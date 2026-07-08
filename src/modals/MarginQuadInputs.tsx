export type MarginValues = {
  top: number;
  right: number;
  bottom: number;
  left: number;
};

type MarginQuadInputsProps = {
  margins: MarginValues;
  onChange: (margins: MarginValues) => void;
  /** 'crop' uses "Top margin:" labels; default uses "Top:" */
  labelStyle?: 'short' | 'crop';
};

const parseMargin = (value: string) => Math.max(0, parseInt(value, 10) || 0);

const sideLabel = (side: string, style: 'short' | 'crop') => (
  style === 'crop' ? `${side} margin` : side
);

export function MarginQuadInputs({ margins, onChange, labelStyle = 'short' }: MarginQuadInputsProps) {
  return (
    <>
      <label>
        {sideLabel('Top', labelStyle)}:
        {' '}
        <input
          type="number"
          value={margins.top}
          onChange={(e) => onChange({ ...margins, top: parseMargin(e.target.value) })}
          min="0"
          className="modal-input"
        />
      </label>
      <label>
        {sideLabel('Right', labelStyle)}:
        {' '}
        <input
          type="number"
          value={margins.right}
          onChange={(e) => onChange({ ...margins, right: parseMargin(e.target.value) })}
          min="0"
          className="modal-input"
        />
      </label>
      <label>
        {sideLabel('Bottom', labelStyle)}:
        {' '}
        <input
          type="number"
          value={margins.bottom}
          onChange={(e) => onChange({ ...margins, bottom: parseMargin(e.target.value) })}
          min="0"
          className="modal-input"
        />
      </label>
      <label>
        {sideLabel('Left', labelStyle)}:
        {' '}
        <input
          type="number"
          value={margins.left}
          onChange={(e) => onChange({ ...margins, left: parseMargin(e.target.value) })}
          min="0"
          className="modal-input"
        />
      </label>
    </>
  );
}

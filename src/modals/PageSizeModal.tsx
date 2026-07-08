import { ScopedPageActionModal } from './ScopedPageActionModal';
import type { PageRangeController } from '../pageRange/usePageRange';

export type PageSizePreset = 'letter' | 'a4' | 'legal';

type PageSizeModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  preset: PageSizePreset;
  onPresetChange: (value: PageSizePreset) => void;
  onClose: () => void;
  onApply: () => void;
  onApplyOdd: () => void;
  onApplyEven: () => void;
};

export function PageSizeModal({
  range,
  pageCount,
  preset,
  onPresetChange,
  onClose,
  onApply,
  onApplyOdd,
  onApplyEven,
}: PageSizeModalProps) {
  return (
    <ScopedPageActionModal
      title="Page Size"
      help="Set MediaBox to a standard paper size (content is not scaled)."
      range={range}
      pageCount={pageCount}
      onClose={onClose}
      onApply={onApply}
      onApplyOdd={onApplyOdd}
      onApplyEven={onApplyEven}
    >
      <label>Preset:</label>
      <select
        className="modal-input"
        value={preset}
        onChange={(e) => onPresetChange(e.target.value as PageSizePreset)}
      >
        <option value="letter">Letter (612×792 pt)</option>
        <option value="a4">A4 (595×842 pt)</option>
        <option value="legal">Legal (612×1008 pt)</option>
      </select>
    </ScopedPageActionModal>
  );
}

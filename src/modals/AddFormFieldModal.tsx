import { useId } from 'react';
import { Modal } from '../ui/Modal';

export type FormFieldKind = 'text' | 'checkbox' | 'choice' | 'radio';

type AddFormFieldModalProps = {
  fieldKind: FormFieldKind;
  fieldName: string;
  fieldOptions: string;
  checkboxChecked: boolean;
  radioGroup: string;
  radioOption: string;
  onFieldKindChange: (kind: FormFieldKind) => void;
  onFieldNameChange: (value: string) => void;
  onFieldOptionsChange: (value: string) => void;
  onCheckboxCheckedChange: (checked: boolean) => void;
  onRadioGroupChange: (value: string) => void;
  onRadioOptionChange: (value: string) => void;
  onClose: () => void;
  onConfirm: () => void;
};

export function AddFormFieldModal({
  fieldKind,
  fieldName,
  fieldOptions,
  checkboxChecked,
  radioGroup,
  radioOption,
  onFieldKindChange,
  onFieldNameChange,
  onFieldOptionsChange,
  onCheckboxCheckedChange,
  onRadioGroupChange,
  onRadioOptionChange,
  onClose,
  onConfirm,
}: AddFormFieldModalProps) {
  const baseId = useId();
  const kindId = `${baseId}-kind`;
  const nameId = `${baseId}-name`;
  const optionsId = `${baseId}-options`;
  const checkedId = `${baseId}-checked`;
  const groupId = `${baseId}-group`;
  const optionId = `${baseId}-option`;

  const disabled = fieldKind === 'radio'
    ? !radioGroup.trim() || !radioOption.trim()
    : !fieldName.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Add Form Field</h3>
      <p className="modal-help">Choose a field type, then place it on the current page.</p>
      <label htmlFor={kindId}>Field type:</label>
      <select
        id={kindId}
        className="modal-input"
        value={fieldKind}
        onChange={(e) => onFieldKindChange(e.target.value as FormFieldKind)}
      >
        <option value="text">Text</option>
        <option value="checkbox">Checkbox</option>
        <option value="choice">Choice list</option>
        <option value="radio">Radio button</option>
      </select>
      {fieldKind === 'radio' ? (
        <>
          <label htmlFor={groupId}>Group name:</label>
          <input
            id={groupId}
            type="text"
            value={radioGroup}
            onChange={(e) => onRadioGroupChange(e.target.value)}
            className="modal-input"
            placeholder="Color"
          />
          <label htmlFor={optionId}>Option name:</label>
          <input
            id={optionId}
            type="text"
            value={radioOption}
            onChange={(e) => onRadioOptionChange(e.target.value)}
            className="modal-input"
            placeholder="Red"
          />
        </>
      ) : (
        <>
          <label htmlFor={nameId}>Field name:</label>
          <input
            id={nameId}
            type="text"
            value={fieldName}
            onChange={(e) => onFieldNameChange(e.target.value)}
            className="modal-input"
            placeholder="Email"
          />
          {fieldKind === 'choice' && (
            <>
              <label htmlFor={optionsId}>Options (comma-separated):</label>
              <input
                id={optionsId}
                type="text"
                value={fieldOptions}
                onChange={(e) => onFieldOptionsChange(e.target.value)}
                className="modal-input"
                placeholder="US, CA, MX"
              />
            </>
          )}
          {fieldKind === 'checkbox' && (
            <label htmlFor={checkedId} className="form-checkbox-row">
              <input
                id={checkedId}
                type="checkbox"
                checked={checkboxChecked}
                onChange={(e) => onCheckboxCheckedChange(e.target.checked)}
              />
              <span>Checked by default</span>
            </label>
          )}
        </>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={onConfirm} className="btn" disabled={disabled}>Place on page</button>
      </div>
    </Modal>
  );
}

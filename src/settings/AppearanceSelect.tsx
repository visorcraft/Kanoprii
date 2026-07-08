import { APPEARANCE_OPTIONS, type AppearanceKey } from './appearancePalettes';

type AppearanceSelectProps = {
  appearance: AppearanceKey;
  setAppearance: (key: AppearanceKey) => void;
};

export function AppearanceSelect({ appearance, setAppearance }: AppearanceSelectProps) {
  return (
    <div className="settings-form-row">
      <label htmlFor="appearance-select" className="settings-form-label">
        Color scheme
      </label>
      <div className="settings-form-control">
        <select
          id="appearance-select"
          className="appearance-select"
          value={appearance}
          onChange={(e) => setAppearance(e.target.value as AppearanceKey)}
          aria-label="Color scheme"
        >
          {APPEARANCE_OPTIONS.map((option) => (
            <option key={option.key} value={option.key}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

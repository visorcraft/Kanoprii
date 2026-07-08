/**
 * E2E-only render-time throw trigger.
 *
 * When the attribute `data-e2e-throw` is present on `<html>`, this component
 * throws during render so the error-boundary E2E spec can verify the fallback
 * UI. The attribute is set and cleared by the E2E test; in normal use it is
 * absent and this component renders nothing.
 */
export function E2EThrowTrigger() {
  if (
    typeof document !== 'undefined' &&
    document.documentElement.hasAttribute('data-e2e-throw')
  ) {
    throw new Error('E2E forced render error');
  }
  return null;
}

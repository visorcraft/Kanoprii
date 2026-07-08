import {
  clickMenuAction,
  dismissToastIfAny,
  fixturePdf,
  openPdfViaPathModal,
  resetToWelcome,
  waitForPdfOpen,
  waitForShell,
} from '../support/helpers';

async function disarmThrowTrigger() {
  await browser.execute(() => {
    document.documentElement.removeAttribute('data-e2e-throw');
  });
}

describe('error boundary', () => {
  before(async () => {
    await waitForShell();
  });

  afterEach(async () => {
    await disarmThrowTrigger();
    await dismissToastIfAny();
  });

  it('catches a panel render error and shows a fallback', async () => {
    await resetToWelcome();
    await openPdfViaPathModal(fixturePdf);
    await waitForPdfOpen();

    // Arm the E2E-only render-time throw trigger inside AppBody.
    await browser.execute(() => {
      document.documentElement.setAttribute('data-e2e-throw', '');
    });

    // Trigger a re-render of AppBody by toggling continuous scroll mode.
    await clickMenuAction('view', 'continuous-scroll');

    // The viewer panel ErrorBoundary should catch the throw and render fallback UI.
    await browser.waitUntil(
      async () => {
        const text = await $('body').getText();
        return /Viewer error|panel failed to render|Try again/i.test(text);
      },
      { timeout: 10_000, timeoutMsg: 'expected error-boundary fallback' },
    );

    // Disarm the trigger and reset the boundary.
    await disarmThrowTrigger();

    const tryAgain = await $('button*=Try again');
    await tryAgain.click();

    // The fallback should be gone and the viewer region should return.
    await browser.waitUntil(
      async () => !(await $('h2=Viewer error').isExisting()),
      { timeout: 10_000, timeoutMsg: 'expected fallback to be removed' },
    );
    await browser.waitUntil(
      async () => (await $('.viewer-main').isDisplayed()),
      { timeout: 10_000, timeoutMsg: 'expected viewer to recover' },
    );
  });
});

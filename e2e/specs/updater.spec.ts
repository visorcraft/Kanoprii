import fs from 'node:fs';
import { waitForShell, clickMenuAction } from '../support/helpers';

type LatestManifest = {
  version: string;
  notes?: string;
};

function writeLatest(result: LatestManifest) {
  const manifestPath = process.env.KANOPRII_LATEST_JSON_PATH;
  if (!manifestPath) {
    throw new Error('KANOPRII_LATEST_JSON_PATH is required; run the suite through scripts/e2e-test.sh.');
  }
  fs.writeFileSync(manifestPath, JSON.stringify({ version: result.version, notes: result.notes }), 'utf8');
}

async function openUpdateModal() {
  await clickMenuAction('help', 'check-updates');
  const modal = await $('[data-testid="update-modal"]');
  await modal.waitForDisplayed({ timeout: 15_000 });
  return modal;
}

async function closeUpdateModal() {
  const backdrop = await $('.modal-backdrop');
  await backdrop.click();
  await $('[data-testid="update-modal"]').waitForDisplayed({ reverse: true, timeout: 5_000 });
}

describe('update notification', () => {
  beforeEach(async () => {
    await browser.execute(() => window.location.reload());
    await waitForShell();
  });

  it('shows check-only update available state on Linux packages', async () => {
    writeLatest({ version: '9.9.9', notes: 'Synthetic release notes' });
    const modal = await openUpdateModal();
    await browser.waitUntil(
      async () => (await modal.getText()).includes('Version 9.9.9 is available'),
      { timeout: 15_000, timeoutMsg: 'expected available update state' },
    );

    const text = await modal.getText();
    expect(text).toContain('Check for Updates');
    expect(text).toContain('Your platform does not support in-app updates');
    expect(text).toContain('Synthetic release notes');

    const releaseBtn = await modal.$('button=Open Release Page');
    await releaseBtn.waitForDisplayed({ timeout: 5_000 });
    expect(await releaseBtn.getAttribute('class')).toContain('btn');

    await closeUpdateModal();
  });

  it('opens update modal and shows up-to-date status on Linux packages', async () => {
    writeLatest({ version: '0.6.5', notes: 'E2E notes' });
    const modal = await openUpdateModal();
    await browser.waitUntil(
      async () => (await modal.getText()).includes('Kanoprii is up to date'),
      { timeout: 15_000, timeoutMsg: 'expected current update state' },
    );

    const text = await modal.getText();
    expect(text).toContain('Check for Updates');
    expect(text).toContain('Kanoprii is up to date');

    await closeUpdateModal();
  });
});

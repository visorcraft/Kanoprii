import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const ext = os.platform() === 'win32' ? '.exe' : '';
const targetDir = path.join(root, 'src-tauri', 'target');
const appBinary = path.join(targetDir, 'debug', `kanoprii${ext}`);

// Native OS dialogs cannot be driven through the embedded WebDriver. Keep E2E
// on Kanoprii's in-app path prompt so Add Image and other picker flows are testable.
process.env.KANOPRII_DISABLE_NATIVE_DIALOGS = '1';

if (!fs.existsSync(appBinary)) {
  throw new Error(
    `Missing ${appBinary}. Run scripts/e2e-build.sh (or scripts/e2e-test.sh) before npm run test:e2e.`,
  );
}

export const config = {
  runner: 'local',
  specs: [
    path.join(root, 'e2e', 'specs', 'smoke.spec.ts'),
    path.join(root, 'e2e', 'specs', 'features.spec.ts'),
    path.join(root, 'e2e', 'specs', 'pdf-edit.spec.ts'),
    path.join(root, 'e2e', 'specs', 'multitab.spec.ts'),
    path.join(root, 'e2e', 'specs', 'updater.spec.ts'),
    path.join(root, 'e2e', 'specs', 'restore-setup.spec.ts'),
    path.join(root, 'e2e', 'specs', 'restore.spec.ts'),
    path.join(root, 'e2e', 'specs', 'keyboard-a11y.spec.ts'),
    path.join(root, 'e2e', 'specs', 'error-boundary.spec.ts'),
    path.join(root, 'e2e', 'specs', 'print.spec.ts'),
    path.join(root, 'e2e', 'specs', 'axe-a11y.spec.ts'),
  ],
  maxInstances: 1,
  logLevel: 'info',
  baseUrl: 'http://localhost:4445',
  waitforTimeout: 15_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 3,
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 180_000,
  },
  services: [
    [
      '@wdio/tauri-service',
      {
        appBinaryPath: appBinary,
        driverProvider: 'embedded',
        embeddedPort: 4445,
        startTimeout: 90_000,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: root,
      },
    },
  ],
};

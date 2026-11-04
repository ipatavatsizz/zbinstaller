import path from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = path.dirname(fileURLToPath(import.meta.url));
const application = process.env.TAURI_TEST_APP;
if (!application) throw new Error('Set TAURI_TEST_APP to the native acceptance build.');

export const config = {
  runner: 'local',
  specs: [path.join(directory, 'cross-platform-ui.spec.js')],
  maxInstances: 1,
  capabilities: [{
    browserName: 'tauri',
    'tauri:options': { application: path.resolve(application) },
  }],
  services: [['@wdio/tauri-service', { driverProvider: 'embedded' }]],
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { timeout: 90_000 },
  waitforTimeout: 10_000,
  connectionRetryTimeout: 90_000,
  connectionRetryCount: 1,
  logLevel: 'warn',
};

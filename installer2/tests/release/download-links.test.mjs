import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';

const page = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8');
const script = page.match(/<script>([\s\S]*?)<\/script>/)[1];

for (const [platform, filename] of [
  ['Win32', 'ZombieBuddyInstaller-windows-amd64.exe'],
  ['Linux x86_64', 'ZombieBuddyInstaller-linux-amd64'],
  ['MacIntel', 'ZombieBuddyInstaller-macos-arm64.zip'],
  ['MacIntel', 'ZombieBuddyInstaller-macos-arm64'],
]) {
  test(`landing page selects the actual ${filename} release asset`, async () => {
    const elements = new Map();
    const document = {
      documentElement: { dataset: {} },
      querySelector: (selector) => {
        if (!elements.has(selector)) elements.set(selector, { setAttribute() {}, addEventListener() {} });
        return elements.get(selector);
      },
    };
    const expected = `https://example.test/releases/download/test/${filename}`;
    const warnings = [];
    vm.runInNewContext(script, {
      document, navigator: { platform }, window: { matchMedia: () => ({ matches: false }) },
      console: { warn: (...args) => warnings.push(args) },
      fetch: async (url) => ({ ok: true, json: async () => url.includes('/git/ref/')
        ? { object: { sha: '123456789abcdef' } }
        : { tag_name: 'v26.11.3', assets: [{ name: filename, size: 1024, browser_download_url: expected }] } }),
    });
    await new Promise(setImmediate);
    assert.equal(elements.get('#download-link').href, expected);
    assert.match(elements.get('#download-meta').textContent, /1234567/);
    assert.deepEqual(warnings, []);
  });
}

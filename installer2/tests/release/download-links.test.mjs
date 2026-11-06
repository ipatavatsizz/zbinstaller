import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';

const page = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8');
const script = page.match(/<script>([\s\S]*?)<\/script>/)[1];
const releasePage = 'https://github.com/ipatavatsizz/zbinstaller/releases/latest';
const release = (assets) => ({ tag_name: 'v26.11.3', html_url: 'https://example.test/releases/tag/v26.11.3', assets });
const asset = (name) => ({ name, size: 1572864, browser_download_url: `https://example.test/download/${name}` });

async function run({ platform = 'Win32', light = false, releaseData = release([]), releaseStatus = 200, tagStatus = 200, tagData = { object: { sha: '123456789abcdef' } }, networkError = false, tagError = false } = {}) {
  const elements = new Map();
  const listeners = new Map();
  const system = { matches: light, addEventListener: (event, handler) => listeners.set(`system:${event}`, handler) };
  const document = {
    documentElement: { dataset: {} },
    querySelector(selector) {
      if (!elements.has(selector)) elements.set(selector, {
        attributes: {}, hidden: true,
        setAttribute(name, value) { this.attributes[name] = value; },
        addEventListener(event, handler) { listeners.set(`${selector}:${event}`, handler); },
      });
      return elements.get(selector);
    },
  };
  const warnings = [];
  const requests = [];
  vm.runInNewContext(script, {
    document, navigator: { platform }, window: { matchMedia: () => system },
    console: { warn: (...args) => warnings.push(args) },
    fetch: async (url) => {
      requests.push(url);
      if (networkError) throw new Error('Network unavailable');
      const tag = url.includes('/git/ref/');
      if (tag && tagError) throw new Error('Tag unavailable');
      const status = tag ? tagStatus : releaseStatus;
      return { ok: status === 200, status, json: async () => tag ? tagData : releaseData };
    },
  });
  await new Promise(setImmediate);
  return { elements, document, warnings, requests, listeners };
}

for (const [platform, filename] of [
  ['Win32', 'ZombieBuddyInstaller-windows-amd64.exe'],
  ['Linux x86_64', 'ZombieBuddyInstaller-linux-amd64'],
  ['MacIntel', 'ZombieBuddyInstaller-macos-arm64.zip'],
  ['MacIntel', 'ZombieBuddyInstaller-macos-arm64'],
]) {
  test(`landing page selects actual ${filename} release asset`, async () => {
    const expected = asset(filename);
    const { elements, warnings } = await run({ platform, releaseData: release([asset('unrelated.zip'), expected]) });
    assert.equal(elements.get('#download-link').href, expected.browser_download_url);
    assert.match(elements.get('#download-meta').textContent, /v26\.11\.3 · 1234567 · .* · 1\.5 MB/);
    assert.deepEqual(warnings, []);
  });
}

test('macOS prefers the APP ZIP when both current and legacy assets exist', async () => {
  const zip = asset('ZombieBuddyInstaller-macos-arm64.zip');
  const { elements } = await run({ platform: 'MacIntel', releaseData: release([asset('ZombieBuddyInstaller-macos-arm64'), zip]) });
  assert.equal(elements.get('#download-link').href, zip.browser_download_url);
});

for (const [name, options, description] of [
  ['unknown platform', { platform: 'FreeBSD' }, /Choose your platform/],
  ['missing platform asset', {}, /No Windows download listed/],
  ['malformed asset collection', { releaseData: release({}) }, /No Windows download listed/],
  ['asset without URL', { releaseData: release([{ name: 'ZombieBuddyInstaller-windows-amd64.exe' }]) }, /No Windows download listed/],
]) {
  test(`${name} keeps a usable release page link`, async () => {
    const { elements, requests } = await run(options);
    assert.equal(elements.get('#download-link').href, 'https://example.test/releases/tag/v26.11.3');
    assert.equal(elements.get('#download-label').textContent, 'View latest release');
    assert.match(elements.get('#download-meta').textContent, description);
    assert.equal(requests.length, 1);
  });
}

for (const options of [{ releaseStatus: 403 }, { networkError: true }]) {
  test(`release API failure ${JSON.stringify(options)} falls back to GitHub`, async () => {
    const { elements, warnings } = await run(options);
    assert.equal(elements.get('#download-link').href, releasePage);
    assert.equal(elements.get('#download-label').textContent, 'View latest release');
    assert.match(elements.get('#download-meta').textContent, /Release details unavailable/);
    assert.equal(warnings.length, 1);
  });
}

for (const options of [{ tagStatus: 404 }, { tagError: true }, { tagData: {} }]) {
  test(`optional tag metadata failure ${JSON.stringify(options)} preserves the asset`, async () => {
    const expected = asset('ZombieBuddyInstaller-windows-amd64.exe');
    const { elements, warnings } = await run({ ...options, releaseData: release([expected]) });
    assert.equal(elements.get('#download-link').href, expected.browser_download_url);
    assert.equal(elements.get('#download-meta').textContent, 'v26.11.3 · Windows · 1.5 MB');
    assert.deepEqual(warnings, []);
  });
}

test('missing tag and size still offer the asset without fabricated metadata', async () => {
  const expected = asset('ZombieBuddyInstaller-windows-amd64.exe');
  delete expected.size;
  const { elements, requests } = await run({ releaseData: { assets: [expected] } });
  assert.equal(elements.get('#download-link').href, expected.browser_download_url);
  assert.equal(elements.get('#download-meta').textContent, 'latest · Windows');
  assert.equal(requests.length, 1);
});

for (const light of [false, true]) {
  test(`theme follows system preference (${light ? 'light' : 'dark'}) until manually changed`, async () => {
    const { document, elements, listeners } = await run({ light });
    const toggle = elements.get('.theme-toggle');
    assert.equal(document.documentElement.dataset.theme, light ? 'light' : 'dark');
    assert.equal(toggle.hidden, false);
    listeners.get('system:change')({ matches: !light });
    assert.equal(document.documentElement.dataset.theme, light ? 'dark' : 'light');
    listeners.get('.theme-toggle:click')();
    assert.equal(document.documentElement.dataset.theme, light ? 'light' : 'dark');
    assert.equal(toggle.attributes['aria-label'], light ? 'Switch to dark theme' : 'Switch to light theme');
    listeners.get('system:change')({ matches: !light });
    assert.equal(document.documentElement.dataset.theme, light ? 'light' : 'dark');
  });
}

test('static HTML offers downloads and content before JavaScript runs', () => {
  const staticPage = page.replace(/<script>[\s\S]*?<\/script>/g, '');
  assert.match(staticPage, /id="download-link" href="https:\/\/github.com\/ipatavatsizz\/zbinstaller\/releases\/latest"/);
  assert.match(staticPage, /All downloads and release notes/);
  assert.match(staticPage, /3619862853/);
  assert.match(staticPage, /3812624292/);
  assert.equal((staticPage.match(/class="feature"/g) || []).length, 6);
  assert.equal((staticPage.match(/class="step"/g) || []).length, 4);
});

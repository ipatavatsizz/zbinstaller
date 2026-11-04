import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const outputDirectory = process.env.TAURI_TEST_OUTPUT ?? path.resolve('test-results/native');
fs.mkdirSync(outputDirectory, { recursive: true });

async function waitFor(check, message, timeout = 15_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const result = await check();
    if (result) return result;
    await browser.pause(100);
  }
  throw new Error(`Timed out: ${message}`);
}

async function currentDocumentContains(fragment) {
  return (await browser.getUrl()).includes(fragment);
}

async function switchToDocument(fragment) {
  const handles = await browser.getWindowHandles();
  for (const handle of handles) {
    await browser.switchToWindow(handle);
    if (await currentDocumentContains(fragment)) return handle;
  }
  return undefined;
}

describe('native Linux/macOS application', () => {
  it('opens, renders both branches, and opens/reuses the native diagnostics window', async () => {
    const mainHandle = await browser.getWindowHandle();

    await waitFor(async () => (await browser.$$('.branch-card')).length === 2, 'both branch cards render');
    await waitFor(async () => browser.execute(() => document.querySelector('.brand img')?.naturalWidth > 0), 'bundled logo loads');

    const layout = await browser.execute(() => {
      const logo = document.querySelector('.brand img').getBoundingClientRect();
      const author = document.querySelector('.author-footer button').getBoundingClientRect();
      const status = document.querySelector('.activity-panel').getBoundingClientRect();
      return {
        width: innerWidth,
        height: innerHeight,
        logoWidth: logo.width,
        logoHeight: logo.height,
        authorCentered: Math.abs((author.left + author.right) / 2 - innerWidth / 2) < 2,
        authorVisibleAboveStatus: author.top >= 0 && author.bottom <= status.top && status.bottom <= innerHeight + 1,
        noHorizontalOverflow: document.documentElement.scrollWidth <= innerWidth,
      };
    });
    assert.equal(layout.width, 680);
    assert.equal(layout.height, 540);
    assert.equal(layout.logoWidth, 96);
    assert.equal(layout.logoHeight, 96);
    assert(layout.authorCentered && layout.authorVisibleAboveStatus && layout.noHorizontalOverflow, JSON.stringify(layout));
    await browser.saveScreenshot(path.join(outputDirectory, 'main.png'));

    const stableTrigger = await browser.$('#lts-actions-menu-trigger');
    await stableTrigger.click();
    assert.equal(await stableTrigger.getAttribute('aria-expanded'), 'true', 'a native click opens the branch actions');
    const enabledItems = await browser.$$('#lts-actions-menu [role="menuitem"]:not(:disabled)');
    assert(enabledItems.length > 0);
    const betaTrigger = await browser.$('#beta-actions-menu-trigger');
    await betaTrigger.click();
    assert.equal(await betaTrigger.getAttribute('aria-expanded'), 'true');
    await betaTrigger.click();
    assert.equal(await betaTrigger.getAttribute('aria-expanded'), 'false');
    await browser.switchToWindow(mainHandle);

    await browser.execute(() => window.__TAURI_INTERNALS__.invoke('open_diagnostics'));
    const diagnosticsHandle = await waitFor(() => switchToDocument('diagnostics.html'), 'F5 opens diagnostics');
    assert(diagnosticsHandle, 'diagnostics WebView must be addressable as a native window');
    await waitFor(async () => browser.execute(() => document.querySelector('#summary')?.textContent?.trim() !== 'Loading diagnostics…'), 'diagnostics finishes loading');
    assert.equal(await browser.execute(() => document.title), 'ZombieBuddy Installer Diagnostics');
    await browser.saveScreenshot(path.join(outputDirectory, 'diagnostics.png'));

    await browser.execute(() => window.__TAURI_INTERNALS__.invoke('close_window'));
    const windowStates = await browser.execute(() => window.__TAURI_INTERNALS__.invoke('plugin:wdio|get_window_states'));
    assert.equal(windowStates.find((window) => window.label === 'main')?.is_focused, true, 'closing diagnostics returns focus to the main native window');
    assert((await browser.getWindowHandles()).includes(mainHandle), 'main native window remains open after diagnostics closes');
    await browser.execute(() => window.__TAURI_INTERNALS__.invoke('open_diagnostics'));
    await waitFor(async () => {
      const handle = await switchToDocument('diagnostics.html');
      return handle && browser.execute(() => document.querySelector('#summary')?.textContent?.trim() !== 'Loading diagnostics…');
    }, 'diagnostics can be reopened after closing');
    await browser.execute(() => window.__TAURI_INTERNALS__.invoke('close_window'));

    // The native driver shuts down the isolated test process after the session.
  });
});

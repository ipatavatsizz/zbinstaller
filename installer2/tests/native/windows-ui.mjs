// Native WebView2 acceptance; launched by windows-ui.ps1. Uses Node 22's WebSocket and fetch.
import assert from 'node:assert/strict';
const port = Number(process.argv[2]);
assert(Number.isInteger(port) && port > 0 && port < 65536, 'Expected a local CDP port');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(check, description) {
  for (let attempt = 0; attempt < 40; attempt++) {
    const value = await check();
    if (value) return value;
    await delay(150);
  }
  throw new Error(`Timed out: ${description}`);
}

const pages = await until(async () => {
  try {
    const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(2000) })).json();
    const pages = targets.filter(target => target.type === 'page' && target.url.includes('tauri.localhost'));
    return pages.length === 2 ? pages : null;
  } catch { return null; }
}, 'main and diagnostics WebViews');

async function connect(target) {
  assert(target, 'Expected native WebView target');
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let next = 0;
  const pending = new Map();
  socket.onmessage = event => {
    const message = JSON.parse(event.data);
    const call = pending.get(message.id);
    if (call) { pending.delete(message.id); message.error ? call.reject(new Error(message.error.message)) : call.resolve(message.result); }
  };
  socket.onclose = () => { for (const call of pending.values()) call.reject(new Error('WebView closed')); pending.clear(); };
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++next;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 5000);
    pending.set(id, { resolve: value => { clearTimeout(timer); resolve(value); }, reject: error => { clearTimeout(timer); reject(error); } });
    socket.send(JSON.stringify({ id, method, params }));
  });
  return {
    send,
    async evaluate(expression) {
      const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
      return result.result?.value;
    },
    async key(key, virtualKey, closing = false) {
      const event = { key, code: key === ' ' ? 'Space' : key, windowsVirtualKeyCode: virtualKey, nativeVirtualKeyCode: virtualKey };
      const text = key === 'Enter' ? '\r' : key === ' ' ? ' ' : undefined;
      try {
        await send('Input.dispatchKeyEvent', { type: 'keyDown', ...event, text, unmodifiedText: text });
        if (!closing) await send('Input.dispatchKeyEvent', { type: 'keyUp', ...event });
      } catch (error) { if (!closing || !String(error).includes('WebView closed')) throw error; }
    },
    close: () => socket.close(),
  };
}

const main = await connect(pages.find(page => !page.url.includes('diagnostics')));
const diagnostics = await connect(pages.find(page => page.url.includes('diagnostics')));
try {
  for (const page of [main, diagnostics]) {
    await page.evaluate(`(() => {
      const allowed = new Set(['discover_installation', 'diagnostic_state', 'diagnostic_report', 'open_diagnostics', 'close_window']);
      const invoke = window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__);
      window.__acceptanceCalls = [];
      window.__acceptanceDenied = [];
      window.__TAURI_INTERNALS__.invoke = (command, ...args) => {
        if (!allowed.has(command)) { window.__acceptanceDenied.push(command); return Promise.reject(new Error('Acceptance runner blocked: ' + command)); }
        window.__acceptanceCalls.push(command);
        return invoke(command, ...args);
      };
    })()`);
  }
  await until(() => main.evaluate(`document.querySelectorAll('.branch-card').length === 2 && document.querySelector('.brand img')?.naturalWidth > 0`), 'main UI load');
  assert.equal(await main.evaluate('document.hasFocus()'), true, 'main WebView startup focus');
  const layout = await main.evaluate(`(() => {
    const logo = document.querySelector('.brand img').getBoundingClientRect();
    const author = document.querySelector('.author-footer button').getBoundingClientRect();
    const status = document.querySelector('.activity-panel').getBoundingClientRect();
    return { width: innerWidth, height: innerHeight, logoWidth: logo.width, logoHeight: logo.height,
      authorCentered: Math.abs((author.left + author.right) / 2 - innerWidth / 2) < 2,
      authorVisibleAboveStatus: author.top >= 0 && author.bottom <= status.top && status.bottom <= innerHeight + 1,
      noHorizontalOverflow: document.documentElement.scrollWidth <= innerWidth,
      statusHeadingRemoved: !document.querySelector('.activity-heading') };
  })()`);
  assert.equal(layout.width, 680);
  assert.equal(layout.height, 540);
  assert.equal(layout.logoWidth, 96);
  assert.equal(layout.logoHeight, 96);
  assert(layout.authorCentered && layout.authorVisibleAboveStatus && layout.noHorizontalOverflow && layout.statusHeadingRemoved, JSON.stringify(layout));

  // Reach the trigger with actual WebView key input, not DOM-dispatched synthetic events.
  let reached = false;
  for (let index = 0; index < 12; index++) {
    await main.key('Tab', 9);
    if (await main.evaluate(`document.activeElement?.getAttribute('aria-label') === 'Stable release actions'`)) { reached = true; break; }
  }
  assert(reached, 'branch action trigger must be keyboard reachable');
  await delay(650);
  assert.equal(await main.evaluate(`document.querySelector('#lts-actions-menu-trigger').getAttribute('aria-expanded')`), 'false', 'focus alone must not open a menu');
  await main.key('ArrowDown', 40);
  await until(() => main.evaluate(`document.activeElement?.getAttribute('role') === 'menuitem'`), 'focus first menu action');
  const enabledCount = await main.evaluate(`document.querySelectorAll('#lts-actions-menu [role="menuitem"]:not(:disabled)').length`);
  assert(enabledCount > 0);
  await main.key('End', 35);
  assert.equal(await main.evaluate(`document.activeElement === [...document.querySelectorAll('#lts-actions-menu [role="menuitem"]:not(:disabled)')].at(-1)`), true);
  await main.key('Home', 36);
  assert.equal(await main.evaluate(`document.activeElement === document.querySelector('#lts-actions-menu [role="menuitem"]:not(:disabled)')`), true);
  await main.key('ArrowUp', 38);
  assert.equal(await main.evaluate(`document.activeElement === [...document.querySelectorAll('#lts-actions-menu [role="menuitem"]:not(:disabled)')].at(-1)`), true);
  await main.key('Escape', 27);
  await until(() => main.evaluate(`document.activeElement?.id === 'lts-actions-menu-trigger' && document.querySelector('#lts-actions-menu-trigger').getAttribute('aria-expanded') === 'false'`), 'Escape dismisses menu and returns focus');
  assert.equal(await main.evaluate(`window.__acceptanceCalls.includes('close_window')`), false, 'menu Escape must not close app');
  await main.key('Enter', 13);
  await until(() => main.evaluate(`document.activeElement?.getAttribute('role') === 'menuitem'`), 'Enter opens menu');
  await main.key('Escape', 27);
  await main.key(' ', 32);
  await until(() => main.evaluate(`document.activeElement?.getAttribute('role') === 'menuitem'`), 'Space opens menu');
  await main.key('Escape', 27);

  const mainDocumentStart = await main.evaluate('performance.timeOrigin');
  await main.key('F5', 116);
  await until(() => diagnostics.evaluate('document.hasFocus()'), 'F5 transfers focus to diagnostics');
  assert.equal(await main.evaluate(`performance.timeOrigin === ${mainDocumentStart} && document.querySelectorAll('.branch-card').length === 2`), true, 'F5 must open diagnostics without reloading or replacing the main view');
  await diagnostics.key('Escape', 27);
  await until(() => main.evaluate('document.hasFocus()'), 'diagnostics Escape returns focus to main');
  await main.key('F5', 116);
  await until(() => diagnostics.evaluate('document.hasFocus()'), 'F5 reopens reusable diagnostics');
  await diagnostics.key('Escape', 27);
  await until(() => main.evaluate('document.hasFocus()'), 'main regains focus after second diagnostics close');
  assert.deepEqual(await main.evaluate('window.__acceptanceDenied'), []);
  assert.deepEqual(await diagnostics.evaluate('window.__acceptanceDenied'), []);
  console.log(JSON.stringify({ result: 'PASS', layout, keyboard: ['Tab', 'ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter', 'Space', 'Escape', 'F5'], diagnosticsReopened: true, installationCommandsInvoked: false }, null, 2));
  // The PowerShell runner independently verifies that this closes the native process.
  await main.key('Escape', 27, true);
} finally { main.close(); diagnostics.close(); }

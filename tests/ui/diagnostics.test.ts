import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  diagnosticReport: vi.fn(),
  diagnosticState: vi.fn(),
  diagnosticAction: vi.fn(),
  clearDiagnosticLog: vi.fn(),
  closeWindow: vi.fn(),
}));
vi.mock('../../src/api', () => api);

function report() {
  return {
    steamRunning: true,
    discovery: {
      steamPath: 'C:/Steam', gamePath: 'C:/Steam/steamapps/common/ProjectZomboid',
      libraries: ['C:/Steam', 'D:/Library'], configPaths: ['C:/Steam/userdata/1/config/localconfig.vdf'],
      branches: [
        { branch: 'lts', path: 'C:/Steam/workshop/lts', version: '2.3.4', ready: true },
        { branch: 'beta', path: null, version: null, ready: false },
      ],
      files: [{ name: 'ProjectZomboid64.json', exists: true, size: 622 }],
    },
  };
}

async function loadDiagnostics() {
  document.body.innerHTML = `
    <p id="summary"></p><pre id="configs"></pre><pre id="libraries"></pre><pre id="files"></pre><pre id="log"></pre>
    <button id="refresh"></button><button id="patch-steam"></button><button id="patch-json"></button>
    <button id="patch-launcher"></button><button id="clear-log"></button>`;
  vi.spyOn(window, 'setInterval').mockReturnValue(1);
  await import('../../src/diagnostics');
  await Promise.resolve();
  await Promise.resolve();
}

describe('diagnostics window UI contract', () => {
  beforeEach(() => {
    vi.resetModules();
    document.open();
    document.write('<!doctype html><html><head></head><body></body></html>');
    document.close();
    api.diagnosticReport.mockReset().mockResolvedValue(report());
    api.diagnosticState.mockReset().mockResolvedValue({ status: 'Idle', message: 'Ready', log: [] });
    api.diagnosticAction.mockReset().mockResolvedValue('Updated configuration');
    api.clearDiagnosticLog.mockReset().mockResolvedValue(undefined);
    api.closeWindow.mockReset().mockResolvedValue(undefined);
  });

  it('renders Steam, game, branch, config and file diagnostics', async () => {
    await loadDiagnostics();
    expect(document.querySelector('#summary')?.textContent).toContain('Steam: C:/Steam (running)');
    expect(document.querySelector('#summary')?.textContent).toContain('LTS: 2.3.4');
    expect(document.querySelector('#configs')?.textContent).toContain('localconfig.vdf');
    expect(document.querySelector('#libraries')?.textContent).toContain('D:/Library');
    expect(document.querySelector('#files')?.textContent).toContain('available (622 bytes)');
  });

  it.each([
    ['patch-steam', 'steam'], ['patch-json', 'json'], ['patch-launcher', 'launcher'],
  ] as const)('routes %s to the %s diagnostic action', async (buttonId, action) => {
    await loadDiagnostics();
    document.querySelector<HTMLButtonElement>(`#${buttonId}`)!.click();
    await Promise.resolve();
    await Promise.resolve();
    expect(api.diagnosticAction).toHaveBeenCalledWith(action);
  });

  it('clears both the visible and backend diagnostic log', async () => {
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: ['previous session'] });
    api.diagnosticAction.mockRejectedValue(new Error('operation rejected'));
    await loadDiagnostics();
    document.querySelector<HTMLButtonElement>('#patch-json')!.click();
    await vi.waitFor(() => expect(document.querySelector('#log')?.textContent).toContain('operation rejected'));
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: [] });
    document.querySelector<HTMLButtonElement>('#clear-log')!.click();
    await Promise.resolve();
    expect(api.clearDiagnosticLog).toHaveBeenCalledOnce();
    expect(document.querySelector('#log')?.textContent).toBe('');
    document.querySelector<HTMLButtonElement>('#refresh')!.click();
    await Promise.resolve();
    await Promise.resolve();
    expect(document.querySelector('#log')?.textContent).toBe('');
  });

  it('does not overlap refresh requests while the backend is pending', async () => {
    let resolveReport!: (value: ReturnType<typeof report>) => void;
    api.diagnosticReport.mockReturnValue(new Promise(resolve => { resolveReport = resolve; }));
    await loadDiagnostics();
    document.querySelector<HTMLButtonElement>('#refresh')!.click();
    const poll = vi.mocked(window.setInterval).mock.calls.at(-1)![0] as () => void;
    poll();
    expect(api.diagnosticReport).toHaveBeenCalledOnce();
    resolveReport(report());
    await vi.waitFor(() => expect(document.querySelector('#summary')?.textContent).toContain('Steam: C:/Steam'));
    document.querySelector<HTMLButtonElement>('#refresh')!.click();
    expect(api.diagnosticReport).toHaveBeenCalledTimes(2);
  });

  it('does not restore cleared logs from an older pending refresh', async () => {
    let resolveReport!: (value: ReturnType<typeof report>) => void;
    api.diagnosticReport.mockReturnValue(new Promise(resolve => { resolveReport = resolve; }));
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: ['old entry'] });
    await loadDiagnostics();
    document.querySelector<HTMLButtonElement>('#clear-log')!.click();
    await Promise.resolve();
    resolveReport(report());
    await vi.waitFor(() => expect(document.querySelector('#summary')?.textContent).toContain('Steam: C:/Steam'));
    expect(document.querySelector('#log')?.textContent).toBe('');
  });

  it.each(['action', 'clear'] as const)('keeps a local %s error visible across automatic refreshes', async kind => {
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: ['previous session'] });
    api.diagnosticAction.mockRejectedValue(new Error('operation rejected'));
    api.clearDiagnosticLog.mockRejectedValue(new Error('operation rejected'));
    await loadDiagnostics();
    document.querySelector<HTMLButtonElement>(kind === 'action' ? '#patch-json' : '#clear-log')!.click();
    await vi.waitFor(() => expect(document.querySelector('#log')?.textContent).toContain('ERROR: Error: operation rejected'));
    const poll = vi.mocked(window.setInterval).mock.calls.at(-1)![0] as () => void;
    poll();
    await Promise.resolve();
    await Promise.resolve();
    expect(document.querySelector('#log')?.textContent).toContain('previous session');
    expect(document.querySelector('#log')?.textContent).toContain('ERROR: Error: operation rejected');
  });

  it('shows missing-game diagnostics instead of hiding all Steam details', async () => {
    const value = report();
    value.discovery.gamePath = '';
    api.diagnosticReport.mockResolvedValue(value);
    await loadDiagnostics();
    expect(document.querySelector('#summary')?.textContent).toContain('Project Zomboid: (not detected)');
    expect(document.querySelector('#libraries')?.textContent).toContain('D:/Library');
  });

  it('loads log history even when the file report fails, then recovers', async () => {
    api.diagnosticReport.mockRejectedValue(new Error('file report unavailable'));
    api.diagnosticState.mockResolvedValue({ status: 'Failed', message: 'Patch failed', log: ['failure details'] });
    await loadDiagnostics();
    await vi.waitFor(() => expect(document.querySelector('#summary')?.textContent).toContain('file report unavailable'));
    expect(document.querySelector('#log')?.textContent).toContain('failure details');
    api.diagnosticReport.mockResolvedValue(report());
    document.querySelector<HTMLButtonElement>('#refresh')!.click();
    await vi.waitFor(() => expect(document.querySelector('#summary')?.textContent).toContain('Steam: C:/Steam'));
  });

  it('renders the file report even when diagnostic state fails', async () => {
    api.diagnosticState.mockRejectedValue(new Error('state unavailable'));
    await loadDiagnostics();
    await vi.waitFor(() => expect(document.querySelector('#summary')?.textContent).toContain('state unavailable'));
    expect(document.querySelector('#summary')?.textContent).toContain('Steam: C:/Steam');
    expect(document.querySelector('#files')?.textContent).toContain('available (622 bytes)');
  });

  it.each([
    { position: 120, expected: 120, label: 'preserves the reading position above the bottom' },
    { position: 800, expected: 1500, label: 'follows new entries when already at the bottom' },
  ])('$label', async ({ position, expected }) => {
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: ['history'] });
    await loadDiagnostics();
    const log = document.querySelector<HTMLElement>('#log')!;
    Object.defineProperties(log, {
      clientHeight: { configurable: true, value: 200 },
      scrollHeight: { configurable: true, get: () => log.textContent?.includes('new entry') ? 1500 : 1000 },
    });
    log.scrollTop = position;
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: ['history', 'new entry'] });
    const poll = vi.mocked(window.setInterval).mock.calls.at(-1)![0] as () => void;
    poll();
    await vi.waitFor(() => expect(log.textContent).toContain('new entry'));
    expect(log.scrollTop).toBe(expected);
  });

  it('does not rebuild unchanged log content during polling', async () => {
    api.diagnosticState.mockResolvedValue({ status: 'Idle', message: 'Ready', log: ['history'] });
    await loadDiagnostics();
    const log = document.querySelector<HTMLElement>('#log')!;
    const textNode = log.firstChild;
    const poll = vi.mocked(window.setInterval).mock.calls.at(-1)![0] as () => void;
    poll();
    await Promise.resolve();
    await Promise.resolve();
    expect(log.firstChild).toBe(textNode);
  });
});

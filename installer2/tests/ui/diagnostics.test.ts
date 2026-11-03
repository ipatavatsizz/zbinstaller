import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  diagnosticReport: vi.fn(),
  diagnosticState: vi.fn(),
  diagnosticAction: vi.fn(),
  clearDiagnosticLog: vi.fn(),
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
    await loadDiagnostics();
    document.querySelector<HTMLButtonElement>('#clear-log')!.click();
    await Promise.resolve();
    expect(api.clearDiagnosticLog).toHaveBeenCalledOnce();
    expect(document.querySelector('#log')?.textContent).toBe('');
  });
});

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { createElement } from 'react';

const api = vi.hoisted(() => ({ discover: vi.fn(), install: vi.fn(), uninstall: vi.fn(), openWorkshop: vi.fn(), openDiagnostics: vi.fn(), onProgress: vi.fn() }));
vi.mock('../../src/api', () => api);

const branches = (ltsPath: string | null = 'C:/Workshop/LTS', ltsReady = Boolean(ltsPath)) => ({
  steamPath: 'C:/Steam', gamePath: 'C:/Steam/steamapps/common/ProjectZomboid', libraries: ['C:/Steam'], configPaths: [],
  files: [{ name: 'zbNative.dll', exists: true, size: 42 }],
  branches: [
    { branch: 'lts', path: ltsPath, version: ltsPath ? '2.3.4' : null, ready: ltsReady },
    { branch: 'beta', path: 'C:/Workshop/Beta', version: '3.0.0-beta3', ready: true },
  ],
});

describe('main window UI contract', () => {
  beforeEach(() => {
    api.discover.mockReset().mockResolvedValue(branches());
    api.install.mockReset().mockResolvedValue('Installation completed successfully.');
    api.uninstall.mockReset().mockResolvedValue('ZombieBuddy was uninstalled successfully.');
    api.openWorkshop.mockReset().mockResolvedValue(undefined);
    api.openDiagnostics.mockReset().mockResolvedValue(undefined);
    api.onProgress.mockReset().mockReturnValue(() => undefined);
    vi.stubGlobal('matchMedia', vi.fn().mockReturnValue({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }));
    vi.spyOn(window, 'setInterval').mockReturnValue(1);
    vi.stubGlobal('confirm', vi.fn().mockReturnValue(true));
  });
  afterEach(() => { cleanup(); vi.useRealTimers(); vi.unstubAllGlobals(); });

  it('shows discovered branch versions and follows system appearance', async () => {
    const { App } = await import('../../src/main');
    render(createElement(App));
    expect(await screen.findByText('Version 2.3.4')).toBeTruthy();
    expect(screen.getByLabelText('System theme: dark')).toBeTruthy();
    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });

  it('opens the Workshop page for an unsubscribed branch and installs a ready build', async () => {
    api.discover.mockResolvedValue(branches(null));
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click(await screen.findByRole('button', { name: /get this version/i }));
    await waitFor(() => expect(api.openWorkshop).toHaveBeenCalledWith('lts'));
    fireEvent.click(screen.getByRole('button', { name: /install update/i }));
    await waitFor(() => expect(api.install).toHaveBeenCalledWith('beta'));
  });

  it('lets Steam finish a subscribed item before beginning installation', async () => {
    api.discover.mockResolvedValue(branches('C:/Workshop/LTS', false));
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click(await screen.findByRole('button', { name: /wait for download/i }));
    await waitFor(() => expect(api.install).toHaveBeenCalledWith('lts'));
  });

  it('opens diagnostics with F5 and reveals secondary actions after a long hover', async () => {
    const { App } = await import('../../src/main');
    render(createElement(App));
    const keyEvent = new KeyboardEvent('keydown', { key: 'F5', cancelable: true });
    document.dispatchEvent(keyEvent);
    expect(keyEvent.defaultPrevented).toBe(true);
    expect(api.openDiagnostics).toHaveBeenCalledOnce();

    fireEvent.mouseEnter(screen.getByText('Stable release').closest('section')!);
    await act(async () => { await new Promise(resolve => window.setTimeout(resolve, 600)); });
    expect(screen.getByRole('button', { name: /open workshop page/i })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /uninstall zombiebuddy/i }));
    await waitFor(() => expect(api.uninstall).toHaveBeenCalledOnce());
  });

  it('disables installation while the backend reports progress', async () => {
    let finish!: (value: string) => void;
    api.install.mockReturnValue(new Promise<string>(resolve => { finish = resolve; }));
    api.onProgress.mockImplementation((callback: (value: { step: number; message: string }) => void) => {
      callback({ step: 3, message: 'Copying library…' });
      return () => undefined;
    });
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click((await screen.findAllByRole('button', { name: /install update/i }))[0]!);
    expect(await screen.findByText('Copying library…')).toBeTruthy();
    expect(screen.getByRole('button', { name: /working/i }).hasAttribute('disabled')).toBe(true);
    await act(async () => finish('Installation completed.'));
  });
});

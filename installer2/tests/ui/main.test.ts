import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { createElement } from 'react';

const api = vi.hoisted(() => ({ discover: vi.fn(), install: vi.fn(), uninstall: vi.fn(), cancelInstall: vi.fn(), openWorkshop: vi.fn(), openDiagnostics: vi.fn(), closeWindow: vi.fn(), openProjectPage: vi.fn(), onProgress: vi.fn() }));
vi.mock('../../src/api', () => api);

const branches = (ltsPath: string | null = 'C:/Workshop/LTS', ltsReady = Boolean(ltsPath)) => ({
  platform: 'windows', steamPath: 'C:/Steam', gamePath: 'C:/Steam/steamapps/common/ProjectZomboid', libraries: ['C:/Steam'], configPaths: [],
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
    api.closeWindow.mockReset().mockResolvedValue(undefined);
    api.cancelInstall.mockReset().mockResolvedValue(undefined);
    api.onProgress.mockReset().mockResolvedValue(() => undefined);
    vi.stubGlobal('matchMedia', vi.fn().mockReturnValue({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }));
    vi.spyOn(window, 'setInterval').mockReturnValue(1);
    vi.stubGlobal('confirm', vi.fn().mockReturnValue(true));
  });
  afterEach(() => { cleanup(); vi.useRealTimers(); vi.unstubAllGlobals(); });

  it('shows discovered branch versions, follows system appearance and closes with Escape', async () => {
    const { App } = await import('../../src/main');
    render(createElement(App));
    expect(await screen.findByRole('button', { name: /install · 2.3.4/i })).toBeTruthy();
    expect(screen.queryByLabelText('Installation check')).toBeNull();
    expect(screen.getByLabelText('Switch to light theme')).toBeTruthy();
    expect(document.documentElement.classList.contains('dark')).toBe(true);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(api.closeWindow).toHaveBeenCalledOnce();
  });

  it('opens Workshop for an unsubscribed branch and installs the selected ready build', async () => {
    api.discover.mockResolvedValue(branches(null));
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click(await screen.findByRole('button', { name: /subscribe to build/i }));
    await waitFor(() => expect(api.openWorkshop).toHaveBeenCalledWith('lts'));
    const button = await screen.findByRole('button', { name: /install · 3.0.0-beta3/i });
    await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false));
    fireEvent.click(button);
    await waitFor(() => expect(api.install).toHaveBeenCalledWith('beta'));
  });

  it('waits for a subscribed download, converts steps to percent, cancels and displays the final result', async () => {
    api.discover.mockResolvedValue(branches('C:/Workshop/LTS', false));
    let reject!: (error: Error) => void;
    let progress!: (value: { step: number; message: string }) => void;
    const stop = vi.fn();
    api.install.mockReturnValue(new Promise<string>((_, fail) => { reject = fail; }));
    api.onProgress.mockImplementation(callback => { progress = callback; return Promise.resolve(stop); });
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click(await screen.findByRole('button', { name: /wait for download/i }));
    await act(async () => { await Promise.resolve(); });
    await waitFor(() => expect(api.install).toHaveBeenCalledWith('lts'));
    act(() => progress({ step: 1, message: 'Waiting for Steam to finish downloading the Workshop build…' }));
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('13');
    fireEvent.click(screen.getByRole('button', { name: /cancel download wait/i }));
    expect(api.cancelInstall).toHaveBeenCalledOnce();
    await act(async () => reject(new Error('Installation cancelled')));
    expect(await screen.findByText('Error: Installation cancelled')).toBeTruthy();
    expect(screen.queryByRole('progressbar')).toBeNull();
    expect(stop).toHaveBeenCalledOnce();
  });

  it('opens diagnostics with F5 and supports context menus and uninstall on Unix', async () => {
    const found = branches();
    found.files = [{ name: 'libzbNative.so', exists: true, size: 42 }];
    api.discover.mockResolvedValue(found);
    const { App } = await import('../../src/main');
    render(createElement(App));
    await screen.findByRole('button', { name: /install · 2.3.4/i });
    const keyEvent = new KeyboardEvent('keydown', { key: 'F5', cancelable: true });
    document.dispatchEvent(keyEvent);
    expect(keyEvent.defaultPrevented).toBe(true);
    expect(api.openDiagnostics).toHaveBeenCalledOnce();
    fireEvent.contextMenu(screen.getByText('Stable release').closest('section')!);
    fireEvent.click(screen.getByRole('menuitem', { name: /uninstall zombiebuddy/i }));
    await waitFor(() => expect(api.uninstall).toHaveBeenCalledOnce());
  });

  it('disables both branches during installation and clears progress after success', async () => {
    let finish!: (value: string) => void;
    let progress!: (value: { step: number; message: string }) => void;
    api.install.mockReturnValue(new Promise<string>(resolve => { finish = resolve; }));
    api.onProgress.mockImplementation(callback => { progress = callback; return Promise.resolve(() => undefined); });
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click(await screen.findByRole('button', { name: /install · 2.3.4/i }));
    await act(async () => { await Promise.resolve(); });
    await waitFor(() => expect(api.install).toHaveBeenCalledOnce());
    act(() => progress({ step: 4, message: 'Copying library…' }));
    expect(screen.getByText('Copying library… (50%)')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: /working/i }).every(button => button.hasAttribute('disabled'))).toBe(true);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(api.closeWindow).not.toHaveBeenCalled();
    await act(async () => finish('Installation completed.'));
    expect(await screen.findByText('Installation completed.')).toBeTruthy();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });

  it('navigates branch actions with arrows and dismisses the menu before closing the window', async () => {
    const { App } = await import('../../src/main');
    render(createElement(App));
    await screen.findByRole('button', { name: /install · 2.3.4/i });
    const trigger = screen.getByRole('button', { name: 'Stable release actions' });
    act(() => trigger.focus());
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    const workshop = screen.getByRole('menuitem', { name: /open workshop/i });
    const uninstall = screen.getByRole('menuitem', { name: /uninstall/i });
    expect(document.activeElement).toBe(workshop);
    fireEvent.keyDown(workshop, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(uninstall);
    fireEvent.keyDown(uninstall, { key: 'Home' });
    expect(document.activeElement).toBe(workshop);
    fireEvent.keyDown(workshop, { key: 'End' });
    expect(document.activeElement).toBe(uninstall);
    fireEvent.keyDown(uninstall, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(workshop);
    fireEvent.keyDown(workshop, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(trigger);
    expect(api.closeWindow).not.toHaveBeenCalled();
    fireEvent.keyDown(trigger, { key: 'Escape', repeat: true });
    expect(api.closeWindow).not.toHaveBeenCalled();
    fireEvent.keyDown(trigger, { key: 'Escape' });
    expect(api.closeWindow).toHaveBeenCalledOnce();
    expect(api.install).not.toHaveBeenCalled();
    expect(api.uninstall).not.toHaveBeenCalled();
    expect(api.openWorkshop).not.toHaveBeenCalled();
  });

  it('skips disabled actions and does not open a menu merely because its trigger receives focus', async () => {
    api.discover.mockResolvedValue({ ...branches(), files: [] });
    const { App } = await import('../../src/main');
    render(createElement(App));
    await screen.findByRole('button', { name: /install · 2.3.4/i });
    vi.useFakeTimers();
    const trigger = screen.getByRole('button', { name: 'Stable release actions' });
    act(() => trigger.focus());
    act(() => vi.advanceTimersByTime(600));
    expect(screen.queryByRole('menu')).toBeNull();
    fireEvent.keyDown(trigger, { key: 'ArrowUp' });
    const workshop = screen.getByRole('menuitem', { name: /open workshop/i });
    expect(document.activeElement).toBe(workshop);
    fireEvent.keyDown(workshop, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(workshop);
  });

  it('cancels pending hover timers when a menu is dismissed with Escape', async () => {
    const { App } = await import('../../src/main');
    render(createElement(App));
    await screen.findByRole('button', { name: /install · 2.3.4/i });
    vi.useFakeTimers();
    const trigger = screen.getByRole('button', { name: 'Stable release actions' });
    fireEvent.mouseEnter(trigger.closest('section')!);
    fireEvent.click(trigger);
    fireEvent.keyDown(screen.getByRole('menuitem', { name: /open workshop/i }), { key: 'Escape' });
    act(() => vi.advanceTimersByTime(700));
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(trigger);
    expect(api.closeWindow).not.toHaveBeenCalled();
  });

  it('shows terminal install errors even after progress has been received', async () => {
    let fail!: (error: Error) => void;
    let progress!: (value: { step: number; message: string }) => void;
    api.install.mockReturnValue(new Promise<string>((_, reject) => { fail = reject; }));
    api.onProgress.mockImplementation(callback => { progress = callback; return Promise.resolve(() => undefined); });
    const { App } = await import('../../src/main');
    render(createElement(App));
    fireEvent.click(await screen.findByRole('button', { name: /install · 2.3.4/i }));
    await act(async () => { await Promise.resolve(); });
    await waitFor(() => expect(api.install).toHaveBeenCalledOnce());
    act(() => progress({ step: 3, message: 'Updating launcher' }));
    await act(async () => fail(new Error('Installation failed; original files restored')));
    expect(await screen.findByText('Error: Installation failed; original files restored')).toBeTruthy();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });
});

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type Branch = 'lts' | 'beta';
export interface BranchInfo { branch: Branch; path: string | null; version: string | null; ready: boolean }
export interface Discovery {
  steamPath: string;
  gamePath: string;
  libraries: string[];
  configPaths: string[];
  branches: BranchInfo[];
  files: FileInfo[];
}
export interface FileInfo { name: string; exists: boolean; size: number | null }
export interface Progress { step: number; message: string }
export interface DiagnosticReport { discovery: Discovery; steamRunning: boolean }
export interface DiagnosticState { status: string; message: string; log: string[] }
export type DiagnosticAction = 'steam' | 'json' | 'launcher';

export const discover = () => invoke<Discovery>('discover_installation');
export const install = (branch: Branch) => invoke<string>('install_branch', { branch });
export const cancelInstall = () => invoke<void>('cancel_install');
export const uninstall = () => invoke<string>('uninstall_zombiebuddy');
export const openWorkshop = (branch: Branch) => invoke<void>('open_workshop', { branch });
export const openDiagnostics = () => invoke<void>('open_diagnostics');
export const diagnosticReport = () => invoke<DiagnosticReport>('diagnostic_report');
export const diagnosticState = () => invoke<DiagnosticState>('diagnostic_state');
export const diagnosticAction = (action: DiagnosticAction) => invoke<string>('diagnostic_action', { action });
export const clearDiagnosticLog = () => invoke<void>('clear_diagnostic_log');
export const onProgress = (callback: (value: Progress) => void) =>
  listen<Progress>('install-progress', event => callback(event.payload));

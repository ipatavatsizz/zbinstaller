import * as api from './api';
import './diagnostics.css';

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing diagnostics element: ${id}`);
  return found as T;
}

const summary = element('summary');
const configs = element('configs');
const libraries = element('libraries');
const files = element('files');
const log = element('log');
let actionBusy = false;

function append(message: string) {
  log.textContent += `[${new Date().toLocaleTimeString()}] ${message}\n`;
  log.scrollTop = log.scrollHeight;
}

async function refresh() {
  try {
    const [report, state] = await Promise.all([api.diagnosticReport(), api.diagnosticState()]);
    const data = report.discovery;
    summary.textContent = `Status: ${state.status}\nMessage: ${state.message}\n\nSteam: ${data.steamPath || '(not detected)'} (${report.steamRunning ? 'running' : 'not running'})\nProject Zomboid: ${data.gamePath || '(not detected)'}\n\nLTS: ${data.branches[0]?.version ?? (data.branches[0]?.path ? 'Version not found' : '(Not subscribed)')}; Beta: ${data.branches[1]?.version ?? (data.branches[1]?.path ? 'Version not found' : '(Not subscribed)')}${data.warnings?.length ? '\n\n' + data.warnings.join('\n') : ''}`;
    configs.textContent = data.configPaths.length ? data.configPaths.join('\n') : 'No Steam user configurations found.';
    libraries.textContent = data.libraries.join('\n');
    files.textContent = data.files.map(file => `${file.name}: ${file.exists ? `available (${file.size ?? 0} bytes)` : 'missing'}`).join('\n');
    const existing = log.textContent ?? '';
    const combined = state.log.join('\n');
    if (combined && !existing.endsWith(combined)) log.textContent = combined;
  } catch (error) {
    summary.textContent = `Diagnostics could not be refreshed: ${String(error)}`;
  }
}

async function runAction(action: api.DiagnosticAction) {
  if (actionBusy) return;
  actionBusy = true;
  for (const button of document.querySelectorAll<HTMLButtonElement>('button')) button.disabled = true;
  try { append(await api.diagnosticAction(action)); }
  catch (error) { append(`ERROR: ${String(error)}`); }
  finally {
    actionBusy = false;
    for (const button of document.querySelectorAll<HTMLButtonElement>('button')) button.disabled = false;
    await refresh();
  }
}

element('refresh').addEventListener('click', () => void refresh());
element('patch-steam').addEventListener('click', () => void runAction('steam'));
element('patch-json').addEventListener('click', () => void runAction('json'));
element('patch-launcher').addEventListener('click', () => void runAction('launcher'));
element('clear-log').addEventListener('click', async () => {
  try { await api.clearDiagnosticLog(); log.textContent = ''; }
  catch (error) { append(`ERROR: ${String(error)}`); }
});
document.addEventListener('keydown', event => {
  if (event.key === 'Escape') void api.closeWindow().catch(error => append(String(error)));
});
void refresh();
window.setInterval(() => { if (!actionBusy) void refresh(); }, 1000);

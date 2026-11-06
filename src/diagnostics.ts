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
let refreshBusy = false;
let logRevision = 0;
let backendLog: string[] = [];
const localLog: string[] = [];

function renderLog() {
  const content = [...backendLog, ...localLog].join('\n');
  if (log.textContent === content) return;
  const position = log.scrollTop;
  const followLatest = log.scrollHeight - log.clientHeight - position <= 1;
  log.textContent = content;
  log.scrollTop = followLatest ? log.scrollHeight : position;
}

function append(message: string) {
  localLog.push(`[${new Date().toLocaleTimeString()}] ${message}`);
  if (localLog.length > 2000) localLog.shift();
  renderLog();
}

async function refresh() {
  if (refreshBusy) return;
  refreshBusy = true;
  const revision = logRevision;
  try {
    const [reportResult, stateResult] = await Promise.allSettled([api.diagnosticReport(), api.diagnosticState()]);
    if (stateResult.status === 'fulfilled' && revision === logRevision) {
      backendLog = stateResult.value.log;
      renderLog();
    }
    if (reportResult.status === 'rejected') throw reportResult.reason;
    const report = reportResult.value;
    const state = stateResult.status === 'fulfilled' ? stateResult.value : {
      status: 'Unavailable',
      message: `Diagnostic state could not be refreshed: ${String(stateResult.reason)}`,
    };
    const data = report.discovery;
    summary.textContent = `Status: ${state.status}\nMessage: ${state.message}\n\nSteam: ${data.steamPath || '(not detected)'} (${report.steamRunning ? 'running' : 'not running'})\nProject Zomboid: ${data.gamePath || '(not detected)'}\n\nLTS: ${data.branches[0]?.version ?? (data.branches[0]?.path ? 'Version not found' : '(Not subscribed)')}; Beta: ${data.branches[1]?.version ?? (data.branches[1]?.path ? 'Version not found' : '(Not subscribed)')}${data.warnings?.length ? '\n\n' + data.warnings.join('\n') : ''}`;
    configs.textContent = data.configPaths.length ? data.configPaths.join('\n') : 'No Steam user configurations found.';
    libraries.textContent = data.libraries.join('\n');
    files.textContent = data.files.map(file => `${file.name}: ${file.exists ? `available (${file.size ?? 0} bytes)` : 'missing'}`).join('\n');
  } catch (error) {
    summary.textContent = `Diagnostics could not be refreshed: ${String(error)}`;
  } finally {
    refreshBusy = false;
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
  logRevision++;
  try {
    await api.clearDiagnosticLog();
    logRevision++;
    backendLog = [];
    localLog.length = 0;
    renderLog();
  }
  catch (error) { append(`ERROR: ${String(error)}`); }
});
document.addEventListener('keydown', event => {
  if (event.key === 'Escape') void api.closeWindow().catch(error => append(String(error)));
});
void refresh();
window.setInterval(() => { if (!actionBusy) void refresh(); }, 1000);

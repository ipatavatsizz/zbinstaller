import { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/geist';
import './style.css';
import { Button } from './components/ui/button';
import { Card } from './components/ui/card';
import * as api from './api';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ArrowDownToLine, ChevronDown, ExternalLink, Moon, RefreshCw, Sun, Trash2 } from 'lucide-react';

function GithubMark() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" width="17" height="17" fill="currentColor"><path d="M12 .297a12 12 0 0 0-3.79 23.4c.6.111.82-.261.82-.577v-2.234c-3.338.726-4.043-1.416-4.043-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.09-.745.083-.729.083-.729 1.205.084 1.84 1.237 1.84 1.237 1.07 1.835 2.807 1.305 3.492.998.108-.776.418-1.305.762-1.605-2.665-.3-5.467-1.335-5.467-5.93 0-1.31.467-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23a11.5 11.5 0 0 1 6.005 0c2.295-1.552 3.3-1.23 3.3-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.435.375.81 1.102.81 2.222v3.293c0 .315.21.69.825.57A12.01 12.01 0 0 0 24 12c0-6.63-5.37-12-12-12z" /></svg>;
}

function ThemeController() {
  const [dark, setDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches);
  const [manual, setManual] = useState(false);
  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const update = (event: MediaQueryListEvent) => { if (!manual) setDark(event.matches); };
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, [manual]);
  useEffect(() => {
    document.documentElement.classList.toggle('dark', dark);
    document.documentElement.classList.toggle('light', !dark);
  }, [dark]);
  return <button type="button" className="theme-indicator" title={`Switch to ${dark ? 'light' : 'dark'} theme`} aria-label={`Switch to ${dark ? 'light' : 'dark'} theme`} aria-pressed={dark} onClick={() => { setManual(true); setDark(value => !value); }}>
    {dark ? <Moon size={15} /> : <Sun size={15} />}
  </button>;
}

function BranchCard({ branch, label, description, installed, hasInstalled, onRefresh, onUninstall, onStatus }: {
  branch: api.Branch; label: string; description: string; installed: api.BranchInfo | undefined;
  hasInstalled: boolean; onRefresh: () => void; onUninstall: () => void; onStatus: (message: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');
  const reportStatus = (message: string) => { setStatus(message); onStatus(message); };
  const timer = useRef<number | undefined>(undefined);
  const closeTimer = useRef<number | undefined>(undefined);
  const longHover = () => {
    window.clearTimeout(closeTimer.current);
    timer.current = window.setTimeout(() => setOpen(true), 550);
  };
  const dismiss = () => {
    window.clearTimeout(timer.current);
    closeTimer.current = window.setTimeout(() => setOpen(false), 220);
  };
  useEffect(() => () => { window.clearTimeout(timer.current); window.clearTimeout(closeTimer.current); }, []);

  async function install() {
    if (busy) return;
    setBusy(true);
    try {
      if (!installed?.path) {
        await api.openWorkshop(branch);
        reportStatus('Workshop opened in Steam. Subscribe, then install when ready.');
      } else if (!installed.ready) {
        reportStatus('Warning: Steam is still downloading this build. Wait for the download to finish.');
      } else {
        const unlisten = await api.onProgress(value => reportStatus(value.message));
        try { reportStatus(await api.install(branch)); } finally { unlisten(); }
      }
    } catch (error) { reportStatus(String(error)); }
    finally { setBusy(false); onRefresh(); }
  }

  return <Card className="branch-card" onMouseEnter={longHover} onMouseLeave={dismiss} onTouchStart={longHover} onTouchEnd={dismiss} onFocus={longHover} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) dismiss(); }}>
    <div className="branch-heading">
      <div className={`branch-mark ${branch}`}><span>{branch === 'lts' ? 'L' : 'β'}</span></div>
      <div className="branch-copy"><div className="branch-title-row"><h2>{label}</h2></div><p>{description}</p></div>
      <ChevronDown className={`more-chevron ${open ? 'is-open' : ''}`} size={17} />
    </div>
    <Button className={`install-button ${installed?.ready ? 'install-ready' : 'install-warning'}`} onClick={() => void install()} disabled={busy}>
      {busy ? <RefreshCw size={15} className="spin" /> : <ArrowDownToLine size={15} />}
      {busy ? 'Working…' : installed?.path ? (installed.ready ? `Install · ${installed.version ?? 'Ready'}` : `Warning · Downloading${installed.version ? ` · ${installed.version}` : ''}`) : `Warning · Subscribe${installed?.version ? ` · ${installed.version}` : ' to build'}`}
    </Button>
    {busy && status.includes('Waiting for Steam') && <Button variant="ghost" size="sm" className="cancel-button" onClick={() => void api.cancelInstall().catch(error => setStatus(String(error)))}>Cancel download wait</Button>}
    <div className={`branch-menu ${open ? 'branch-menu-open' : ''}`} aria-hidden={!open} inert={!open}>
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => void api.openWorkshop(branch).then(() => reportStatus('Workshop page opened in Steam.')).catch(error => reportStatus(String(error)))}><ExternalLink size={14} /> Open Workshop page</Button>
      <Button variant="ghost" size="sm" disabled={busy || !hasInstalled} onClick={onUninstall}><Trash2 size={14} /> Uninstall ZombieBuddy</Button>
    </div>
  </Card>;
}

export function App() {
  const [data, updateDiscovery] = useState<api.Discovery>();
  const [status, updateStatus] = useState('Finding your Steam installation…');
  const [progress, updateProgress] = useState<api.Progress>();
  const refresh = () => void api.discover().then(updateDiscovery).catch(error => updateStatus(String(error)));
  const requestUninstall = async () => {
    if (!window.confirm('Remove ZombieBuddy from Project Zomboid? This removes its installed files and launch options.')) return;
    try { updateStatus('Uninstalling ZombieBuddy…'); updateStatus(await api.uninstall()); }
    catch (error) { updateStatus(String(error)); }
    finally { void api.discover().then(updateDiscovery).catch(() => undefined); }
  };

  useEffect(() => {
    let mounted = true;
    let firstLoad = true;
    const refresh = () => void api.discover().then(value => { if (mounted) { updateDiscovery(value); if (firstLoad) updateStatus('Choose a ZombieBuddy version to install.'); firstLoad = false; } })
      .catch(error => { if (mounted) updateStatus(String(error)); });
    refresh();
    const interval = window.setInterval(refresh, 5000);
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'F5') return;
      event.preventDefault();
      event.stopPropagation();
      event.stopImmediatePropagation();
      void api.openDiagnostics().catch(error => updateStatus(String(error)));
    };
    document.addEventListener('keydown', onKey, true);
    const unlisten = api.onProgress(value => { updateProgress(value); updateStatus(value.message); });
    return () => { mounted = false; window.clearInterval(interval); document.removeEventListener('keydown', onKey, true); void unlisten.then(stop => stop()); };
  }, []);

  const installed = (branch: api.Branch) => data?.branches.find(item => item.branch === branch);
  return <main className="app-shell">
    <header className="topbar"><div className="brand"><img src="/src/assets/zombiebuddy.png" alt="" /><div><span className="eyebrow">PROJECT ZOMBOID</span><h1>ZombieBuddy <span>Installer</span></h1></div></div><div className="topbar-actions"><Button variant="ghost" size="icon" aria-label="Open ZombieBuddy GitHub repository" onClick={() => void openUrl('https://github.com/ipatavatsizz/zbinstaller/').catch(error => updateStatus(String(error)))}><GithubMark /></Button><ThemeController /></div></header>
    <div className="intro"><div><span className="section-label">INSTALLER</span><h2>Choose your build</h2><p>Keep your ZombieBuddy version up to date.</p></div></div>
    <div className="branch-grid">
      <BranchCard branch="lts" label="Stable release" description="Reliable and recommended" installed={installed('lts')} hasInstalled={!!data?.files.some(file => file.name === 'zbNative.dll' && file.exists)} onRefresh={refresh} onUninstall={() => void requestUninstall()} onStatus={message => { updateProgress(undefined); updateStatus(message); }} />
      <BranchCard branch="beta" label="Beta release" description="Get the latest features" installed={installed('beta')} hasInstalled={!!data?.files.some(file => file.name === 'zbNative.dll' && file.exists)} onRefresh={refresh} onUninstall={() => void requestUninstall()} onStatus={message => { updateProgress(undefined); updateStatus(message); }} />
    </div>
    <section className="activity-panel" role="status" aria-live="polite"><div className="activity-heading"><span className={`status-dot ${status.toLowerCase().includes('error') || status.toLowerCase().includes('failed') ? 'status-error' : ''}`} />Activity</div><p>{progress ? `${progress.message} (${progress.step}%)` : status}</p>{progress && <div className="progress-track"><span style={{ width: `${Math.max(0, Math.min(100, progress.step))}%` }} /></div>}</section>
  </main>;
}

const root = document.getElementById('root');
if (root) createRoot(root).render(<App />);

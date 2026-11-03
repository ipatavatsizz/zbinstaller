import { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/geist';
import './style.css';
import { Button } from './components/ui/button';
import { Card } from './components/ui/card';
import * as api from './api';
import { ArrowDownToLine, Check, ChevronDown, CircleHelp, ExternalLink, Moon, RefreshCw, Sun, Trash2 } from 'lucide-react';

function ThemeController() {
  const [dark, setDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches);
  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const update = (event: MediaQueryListEvent) => setDark(event.matches);
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  useEffect(() => { document.documentElement.classList.toggle('dark', dark); }, [dark]);
  return <span className="theme-indicator" title={`System theme: ${dark ? 'dark' : 'light'}`} aria-label={`System theme: ${dark ? 'dark' : 'light'}`}>
    {dark ? <Moon size={14} /> : <Sun size={14} />}<span>System</span>
  </span>;
}

function BranchCard({ branch, label, description, installed, hasInstalled, onRefresh, onUninstall }: {
  branch: api.Branch; label: string; description: string; installed: api.BranchInfo | undefined;
  hasInstalled: boolean; onRefresh: () => void; onUninstall: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');
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
        setStatus('Workshop opened in Steam. Subscribe, then install when ready.');
      } else {
        const unlisten = await api.onProgress(value => setStatus(value.message));
        try { setStatus(await api.install(branch)); } finally { unlisten(); }
      }
    } catch (error) { setStatus(String(error)); }
    finally { setBusy(false); onRefresh(); }
  }

  return <Card className="branch-card" onMouseEnter={longHover} onMouseLeave={dismiss} onTouchStart={longHover} onTouchEnd={dismiss} onFocus={longHover} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) dismiss(); }}>
    <div className="branch-heading">
      <div className={`branch-mark ${branch}`}><span>{branch === 'lts' ? 'L' : 'β'}</span></div>
      <div className="branch-copy"><div className="branch-title-row"><h2>{label}</h2>{installed?.ready && <span className="ready-pill"><Check size={11} /> Ready</span>}</div>
        <p>{installed?.version ? `Version ${installed.version}` : description}</p></div>
      <ChevronDown className={`more-chevron ${open ? 'is-open' : ''}`} size={17} />
    </div>
    <Button className="install-button" onClick={() => void install()} disabled={busy}>
      {busy ? <RefreshCw size={15} className="spin" /> : <ArrowDownToLine size={15} />}
      {busy ? 'Working…' : installed?.path ? (installed.ready ? 'Install update' : 'Wait for download') : 'Get this version'}
    </Button>
    {status && <p className="branch-status" role="status">{status}</p>}
    {busy && status.includes('Waiting for Steam') && <Button variant="ghost" size="sm" className="cancel-button" onClick={() => void api.cancelInstall().catch(error => setStatus(String(error)))}>Cancel download wait</Button>}
    <div className={`branch-menu ${open ? 'branch-menu-open' : ''}`} aria-hidden={!open} inert={!open}>
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => void api.openWorkshop(branch).catch(error => setStatus(String(error)))}><ExternalLink size={14} /> Open Workshop page</Button>
      <Button variant="ghost" size="sm" disabled={busy || !hasInstalled} onClick={onUninstall}><Trash2 size={14} /> Uninstall ZombieBuddy</Button>
    </div>
  </Card>;
}

export function App() {
  const [data, updateDiscovery] = useState<api.Discovery>();
  const [status, updateStatus] = useState('Finding your Steam installation…');
  const refresh = () => void api.discover().then(value => { updateDiscovery(value); updateStatus('Choose a ZombieBuddy version to install.'); }).catch(error => updateStatus(String(error)));
  const requestUninstall = async () => {
    if (!window.confirm('Remove ZombieBuddy from Project Zomboid? This removes its installed files and launch options.')) return;
    try { updateStatus('Uninstalling ZombieBuddy…'); updateStatus(await api.uninstall()); }
    catch (error) { updateStatus(String(error)); }
    finally { void api.discover().then(updateDiscovery).catch(() => undefined); }
  };

  useEffect(() => {
    let mounted = true;
    const refresh = () => void api.discover().then(value => { if (mounted) { updateDiscovery(value); updateStatus('Choose a ZombieBuddy version to install.'); } })
      .catch(error => { if (mounted) updateStatus(String(error)); });
    refresh();
    const interval = window.setInterval(refresh, 5000);
    const onKey = (event: KeyboardEvent) => { if (event.key === 'F5') { event.preventDefault(); void api.openDiagnostics().catch(error => updateStatus(String(error))); } };
    document.addEventListener('keydown', onKey);
    return () => { mounted = false; window.clearInterval(interval); document.removeEventListener('keydown', onKey); };
  }, []);

  const installed = (branch: api.Branch) => data?.branches.find(item => item.branch === branch);
  return <main className="app-shell">
    <header className="topbar"><div className="brand"><img src="/src/assets/zombiebuddy.png" alt="" /><div><span className="eyebrow">PROJECT ZOMBOID</span><h1>ZombieBuddy <span>Installer</span></h1></div></div><ThemeController /></header>
    <div className="intro"><div><span className="section-label">INSTALLER</span><h2>Choose your build</h2><p>Keep your ZombieBuddy version up to date.</p></div><Button variant="ghost" size="icon" aria-label="Open diagnostics" title="Diagnostics (F5)" onClick={() => void api.openDiagnostics()}><CircleHelp size={17} /></Button></div>
    <div className="branch-grid">
      <BranchCard branch="lts" label="Stable release" description="Reliable and recommended" installed={installed('lts')} hasInstalled={!!data?.files.some(file => file.name === 'zbNative.dll' && file.exists)} onRefresh={refresh} onUninstall={() => void requestUninstall()} />
      <BranchCard branch="beta" label="Beta release" description="Get the latest features" installed={installed('beta')} hasInstalled={!!data?.files.some(file => file.name === 'zbNative.dll' && file.exists)} onRefresh={refresh} onUninstall={() => void requestUninstall()} />
    </div>
    <div className="status-line" role="status" aria-live="polite"><span className={`status-dot ${status.includes('Error') || status.includes('failed') ? 'status-error' : ''}`} />{status}</div>
    <footer><span>ZombieBuddy Installer</span><span className="footer-separator">·</span><a href="https://github.com/zed-0xff/ZombieBuddy" target="_blank" rel="noreferrer">Community project <ExternalLink size={11} /></a><span className="footer-hint">F5 · Diagnostics</span></footer>
  </main>;
}

const root = document.getElementById('root');
if (root) createRoot(root).render(<App />);

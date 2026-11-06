import '@fontsource-variable/geist'
import { ArrowDownToLine, ChevronDown, ExternalLink, Moon, RefreshCw, Sun, Trash2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { createRoot } from 'react-dom/client'
import * as api from './api'
import logo from './assets/zombiebuddy.png'
import { Button } from './components/ui/button'
import { Card } from './components/ui/card'
import './style.css'

function GithubMark() {
  return (
    <svg aria-hidden="true" viewBox="0 0 24 24" width="17" height="17" fill="currentColor">
      <path d="M12 .297a12 12 0 0 0-3.79 23.4c.6.111.82-.261.82-.577v-2.234c-3.338.726-4.043-1.416-4.043-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.09-.745.083-.729.083-.729 1.205.084 1.84 1.237 1.84 1.237 1.07 1.835 2.807 1.305 3.492.998.108-.776.418-1.305.762-1.605-2.665-.3-5.467-1.335-5.467-5.93 0-1.31.467-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23a11.5 11.5 0 0 1 6.005 0c2.295-1.552 3.3-1.23 3.3-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.435.375.81 1.102.81 2.222v3.293c0 .315.21.69.825.57A12.01 12.01 0 0 0 24 12c0-6.63-5.37-12-12-12z" />
    </svg>
  )
}

function ThemeController() {
  const [dark, setDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches)
  const [manual, setManual] = useState(false)
  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)')
    const update = (event: MediaQueryListEvent) => {
      if (!manual) setDark(event.matches)
    }
    query.addEventListener('change', update)
    return () => query.removeEventListener('change', update)
  }, [manual])
  useEffect(() => {
    document.documentElement.classList.toggle('dark', dark)
    document.documentElement.classList.toggle('light', !dark)
  }, [dark])
  return (
    <button
      type="button"
      className="theme-indicator"
      title={`Switch to ${dark ? 'light' : 'dark'} theme`}
      aria-label={`Switch to ${dark ? 'light' : 'dark'} theme`}
      aria-pressed={dark}
      onClick={() => {
        setManual(true)
        setDark((value) => !value)
      }}
    >
      {dark ? <Moon size={15} /> : <Sun size={15} />}
    </button>
  )
}

function BranchCard({
  branch,
  label,
  description,
  installed,
  hasInstalled,
  busy,
  waiting,
  onInstall,
  onWorkshop,
  onUninstall,
  onCancel
}: {
  branch: api.Branch
  label: string
  description: string
  installed: api.BranchInfo | undefined
  hasInstalled: boolean
  busy: boolean
  waiting: boolean
  onInstall: () => void
  onWorkshop: () => void
  onUninstall: () => void
  onCancel: () => void
}) {
  const [open, setOpen] = useState(false)
  const [focusEdge, setFocusEdge] = useState<'first' | 'last'>()
  const trigger = useRef<HTMLButtonElement>(null)
  const menu = useRef<HTMLDivElement>(null)
  const menuId = `${branch}-actions-menu`
  const timer = useRef<number | undefined>(undefined)
  const closeTimer = useRef<number | undefined>(undefined)
  const clearTimers = () => {
    window.clearTimeout(timer.current)
    window.clearTimeout(closeTimer.current)
  }
  const openMenu = (edge: 'first' | 'last' = 'first') => {
    clearTimers()
    setOpen(true)
    setFocusEdge(edge)
  }
  const closeMenu = (restoreFocus = false) => {
    clearTimers()
    setOpen(false)
    setFocusEdge(undefined)
    if (restoreFocus) trigger.current?.focus()
  }
  const longHover = () => {
    clearTimers()
    timer.current = window.setTimeout(() => setOpen(true), 550)
  }
  const dismiss = () => {
    clearTimers()
    if (menu.current?.contains(document.activeElement)) return
    closeTimer.current = window.setTimeout(() => setOpen(false), 220)
  }
  useEffect(() => {
    if (!open || !focusEdge) return
    const items = menu.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')
    if (items?.length) items[focusEdge === 'first' ? 0 : items.length - 1]?.focus()
    setFocusEdge(undefined)
  }, [open, focusEdge])
  useEffect(
    () => () => {
      window.clearTimeout(timer.current)
      window.clearTimeout(closeTimer.current)
    },
    []
  )

  return (
    <Card
      className="branch-card"
      onContextMenu={(event) => {
        event.preventDefault()
        openMenu()
      }}
      onMouseEnter={longHover}
      onMouseLeave={dismiss}
      onTouchStart={longHover}
      onTouchEnd={dismiss}
      onBlur={(event) => {
        if (!menu.current?.contains(event.relatedTarget) && event.relatedTarget !== trigger.current) closeMenu()
      }}
      onKeyDown={(event) => {
        if (event.key === 'Escape' && open) {
          event.preventDefault()
          event.stopPropagation()
          closeMenu(true)
        }
      }}
    >
      <div className="branch-heading">
        <div className={`branch-mark ${branch}`}>
          <span>{branch === 'lts' ? 'L' : 'β'}</span>
        </div>
        <div className="branch-copy">
          <div className="branch-title-row">
            <h2>{label}</h2>
          </div>
          <p>{description}</p>
        </div>
        <Button
          ref={trigger}
          id={`${menuId}-trigger`}
          variant="ghost"
          size="icon"
          aria-label={`${label} actions`}
          aria-haspopup="menu"
          aria-controls={menuId}
          aria-expanded={open}
          onClick={() => open ? closeMenu() : openMenu()}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault()
              openMenu(event.key === 'ArrowUp' ? 'last' : 'first')
            }
          }}
        >
          <ChevronDown className={`more-chevron ${open ? 'is-open' : ''}`} size={17} />
        </Button>
      </div>
      <Button
        className={`install-button ${installed?.ready ? 'install-ready' : 'install-warning'}`}
        onClick={installed?.path ? onInstall : onWorkshop}
        disabled={busy || !installed}
      >
        {busy ? <RefreshCw size={15} className="spin" /> : <ArrowDownToLine size={15} />}
        {busy
          ? 'Working…'
          : installed?.path
            ? installed.ready
              ? `Install · ${installed.version ?? 'Version not found'}`
              : `Wait for download${installed.version ? ` · ${installed.version}` : ''}`
            : 'Subscribe to build'}
      </Button>
      {waiting && (
        <Button variant="ghost" size="sm" className="cancel-button" onClick={onCancel}>
          Cancel download wait
        </Button>
      )}
      <div
        ref={menu}
        id={menuId}
        role="menu"
        aria-labelledby={`${menuId}-trigger`}
        className={`branch-menu ${open ? 'branch-menu-open' : ''}`}
        aria-hidden={!open}
        inert={!open}
        onKeyDown={(event) => {
          if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return
          event.preventDefault()
          const items = [...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')]
          if (!items.length) return
          const index = items.indexOf(document.activeElement as HTMLButtonElement)
          const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
          items[next]?.focus()
        }}
      >
        <Button role="menuitem" tabIndex={-1} variant="ghost" size="sm" disabled={busy} onClick={() => { closeMenu(true); onWorkshop() }}>
          <ExternalLink size={14} /> Open Workshop page
        </Button>
        <Button role="menuitem" tabIndex={-1} variant="ghost" size="sm" disabled={busy || !hasInstalled} onClick={() => { closeMenu(true); onUninstall() }}>
          <Trash2 size={14} /> Uninstall ZombieBuddy
        </Button>
      </div>
    </Card>
  )
}

export function App() {
  const [data, updateDiscovery] = useState<api.Discovery>()
  const [status, updateStatus] = useState('Finding your Steam installation…')
  const [progress, updateProgress] = useState<api.Progress>()
  const [activeBranch, setActiveBranch] = useState<api.Branch>()
  const [busy, setBusy] = useState(false)
  const operation = useRef(false)
  const reportStatus = (message: string) => {
    updateProgress(undefined)
    updateStatus(message)
  }
  const refresh = () =>
    void api
      .discover()
      .then(updateDiscovery)
      .catch((error) => {
        if (!operation.current) reportStatus(String(error))
      })

  async function perform(action: () => Promise<string>) {
    if (operation.current) return
    operation.current = true
    setBusy(true)
    updateProgress(undefined)
    try {
      reportStatus(await action())
    } catch (error) {
      reportStatus(String(error))
    } finally {
      operation.current = false
      setBusy(false)
      setActiveBranch(undefined)
      refresh()
    }
  }

  const install = (branch: api.Branch) =>
    void perform(async () => {
      setActiveBranch(branch)
      updateStatus('Starting installation…')
      const unlisten = await api.onProgress((value) => {
        updateProgress(value)
        updateStatus(value.message)
      })
      try {
        return await api.install(branch)
      } finally {
        unlisten()
      }
    })
  const workshop = (branch: api.Branch) =>
    void perform(async () => {
      await api.openWorkshop(branch)
      return 'Workshop opened in Steam. Subscribe, then install when ready.'
    })
  const requestUninstall = () => {
    if (operation.current || !window.confirm('Remove ZombieBuddy from Project Zomboid? This removes its installed files and launch options.')) return
    void perform(async () => {
      updateStatus('Uninstalling ZombieBuddy…')
      return await api.uninstall()
    })
  }
  useEffect(() => {
    let mounted = true
    let firstLoad = true
    const refresh = () =>
      void api
        .discover()
        .then((value) => {
          if (!mounted) return
          updateDiscovery(value)
          if (firstLoad && !operation.current) updateStatus(value.warnings?.length ? value.warnings.join('\n') : 'Choose a ZombieBuddy version to install.')
          firstLoad = false
        })
        .catch((error) => {
          if (mounted && !operation.current) updateStatus(String(error))
        })
    refresh()
    const interval = window.setInterval(refresh, 3000)
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.repeat) return
      if (event.key === 'Escape' && !operation.current) {
        void api.closeWindow().catch((error) => updateStatus(String(error)))
        return
      }
      if (event.key !== 'F5') return
      event.preventDefault()
      event.stopPropagation()
      event.stopImmediatePropagation()
      void api.openDiagnostics().catch((error) => {
        if (mounted) updateStatus(String(error))
      })
    }
    // Bubble after menu handlers so Escape can dismiss a menu before closing the window.
    document.addEventListener('keydown', onKey)
    return () => {
      mounted = false
      window.clearInterval(interval)
      document.removeEventListener('keydown', onKey)
    }
  }, [])

  const installed = (branch: api.Branch) => data?.branches.find((item) => item.branch === branch)
  const hasInstalled = !!data?.files.some((file) => ['zbNative.dll', 'libzbNative.so', 'libzbNative.dylib', 'ZombieBuddy.jar'].includes(file.name) && file.exists)
  const percentage = progress ? Math.round((progress.step / 8) * 100) : 0
  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <button
            type="button"
            className="brand-link"
            aria-label="Open ZombieBuddy GitHub repository"
            title="ZombieBuddy on GitHub"
            onClick={() => void api.openProjectPage('original').catch((error) => reportStatus(String(error)))}
          >
            <img src={logo} alt="ZombieBuddy" />
          </button>
        </div>
        <div className="topbar-actions">
          <Button
            variant="ghost"
            size="icon"
            aria-label="Open installer GitHub repository"
            onClick={() => void api.openProjectPage('installer').catch((error) => reportStatus(String(error)))}
          >
            <GithubMark />
          </Button>
          <ThemeController />
        </div>
      </header>
      <div className="intro">
        <div>
          <h1>Choose your build</h1>
          <p>Keep your ZombieBuddy version up to date.</p>
        </div>
      </div>
      <div className="branch-grid">
        {(['lts', 'beta'] as const).map((branch) => (
          <BranchCard
            key={branch}
            branch={branch}
            label={branch === 'lts' ? 'Stable release' : 'Beta release'}
            description={branch === 'lts' ? 'Reliable and recommended' : 'Get the latest features'}
            installed={installed(branch)}
            hasInstalled={hasInstalled}
            busy={busy}
            waiting={activeBranch === branch && progress?.step === 1 && progress.message.includes('Waiting for Steam')}
            onInstall={() => install(branch)}
            onWorkshop={() => workshop(branch)}
            onUninstall={requestUninstall}
            onCancel={() => void api.cancelInstall().catch((error) => reportStatus(String(error)))}
          />
        ))}
      </div>
      <footer className="author-footer">
        <Button variant="ghost" size="sm" onClick={() => void api.openProjectPage('website').catch((error) => reportStatus(String(error)))}>
          ipatavatsizz
        </Button>
      </footer>
      <section className="activity-panel" role="status" aria-live="polite">
        <div className="activity-message">
          <span aria-hidden="true" className={`status-dot ${status.toLowerCase().includes('error') || status.toLowerCase().includes('failed') ? 'status-error' : ''}`} />
          <p>{progress ? `${progress.message} (${percentage}%)` : status}</p>
        </div>
        {progress && (
          <div className="progress-track" role="progressbar" aria-label="Installation progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percentage}>
            <span style={{ width: `${percentage}%` }} />
          </div>
        )}
      </section>
    </main>
  )
}

const root = document.getElementById('root')
if (root) createRoot(root).render(<App />)

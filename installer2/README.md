# ZombieBuddy Installer 2

Tauri 2 + Rust migration of the [Go installer](../installer/install.go).
The first implementation targets Windows. The Go application remains the release implementation until migration acceptance is complete.

## Develop

Requirements: Node.js 22.12+ (or a newer supported release), npm, stable Rust with the MSVC toolchain, Visual Studio C++ Build Tools, and Microsoft Edge WebView2 Runtime.

```powershell
cd installer2
npm ci
npm run tauri dev
```

From the repository root in VS Code, press **F5** and choose **Run ZombieBuddy Installer 2 (Tauri)**. It starts Vite, builds the Rust debug app, and sends output to VS Code's Debug Console without opening a terminal. The Rust file watcher is disabled to avoid Windows locking errors on the running `.exe` and `.pdb`. Closing the main window exits the app and ends the debug session. Stop it with **Shift+F5** when needed.

```powershell
npm run build
cargo test -p installer2-tests
npm run test:ui
cargo check -p installer2
cargo clippy --workspace --all-targets -- -D warnings
npm run tauri build
```

The fixture-based Rust unit/integration tests and mocked UI tests are described in [tests/README.md](tests/README.md).

The Windows NSIS installer is produced under `target/release/bundle/nsis/`.
`npm run dev` alone serves the UI; native commands require `npm run tauri dev`.
The main window follows the Fyne layout in a resizable window. Use a branch card's additional actions to open its Workshop page or uninstall ZombieBuddy. Diagnostics is available from the diagnostics controls.

## Current behavior

- Registry-based Steam discovery, additional Steam libraries and game manifests.
- LTS/Beta Workshop pages, local versions and download readiness checks.
- Polls Steam's `appworkshop_108600.acf` every two seconds and waits until the selected Workshop item is recorded as fully installed and its payload files are ready; the wait can be cancelled in the UI.
- Graceful Steam shutdown with a 30-second timeout; no forced process termination.
- Native DLL/JAR installation, Steam launch options, launcher JSON and batch patching.
- All patches are validated before writing; atomic replacement and in-memory rollback on write failures.
- Background execution, progress events, diagnostic paths and session logs.
- Installation and Workshop commands cannot overlap within one application process. Closing the window is prevented while a native operation runs.
- System-matched light/dark appearance, delayed branch action menus, and transactional ZombieBuddy uninstall.

Close the game before installing. Steam must have a Project Zomboid app entry with a `LaunchOptions` string (as required by the Go implementation). If absent, configure launch options in Steam first. The installer reports an error instead of inventing or overwriting an unfamiliar configuration.

Rollback handles ordinary returned I/O errors, not power loss or process termination. This version does not create persistent `.backup` files. Do not run multiple installer processes simultaneously. Steam restart failure after successful writes is reported as a warning and does not undo the installation.

See [architecture](docs/ARCHITECTURE.md) and [migration checklist](docs/MIGRATION.md).

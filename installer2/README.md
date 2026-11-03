# ZombieBuddy Installer 2

Tauri 2 + Rust migration of the [Go installer](../installer/install.go). The Windows, Linux and macOS installation paths are implemented, with platform fixture coverage on Windows. Native Linux/macOS and real Steam/game acceptance are still required before replacing the Go release. See the numbered [migration checklist](docs/MIGRATION.md).

## Develop

Use Node.js 22.12+, npm and stable Rust 1.89+ (dependencies may require a newer stable compiler). Windows needs the MSVC toolchain, Visual Studio C++ Build Tools and WebView2. Linux needs WebKitGTK 4.1 and the [Tauri native prerequisites](https://v2.tauri.app/start/prerequisites/); macOS needs Xcode command line tools.

```powershell
cd installer2
npm ci
npm run tauri dev
```

From the repository root in VS Code, use **F5 → Run ZombieBuddy Installer 2 (Tauri)**. It starts Vite and the Rust debug app. The Rust watcher is disabled to avoid locking the running Windows executable/PDB. Closing the main window ends the app; **Shift+F5** stops the debug session.

## Verify and package

```powershell
npm run build
npm run test:ui
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build
```

Native packages appear under `target/release/bundle/`: Windows NSIS, Linux DEB/AppImage, macOS APP/DMG. The [Installer 2 workflow](../.github/workflows/installer2.yml) runs the same checks on native runners and uploads packages. Its manual action can create a draft prerelease with an `installer2-v...` candidate tag. The existing Go tag release workflow remains in place until native acceptance is recorded. Signing credentials are not supplied by this repository.

`npm run dev` alone serves the view; native commands require the Tauri app. See [test coverage](tests/README.md) and [architecture](docs/ARCHITECTURE.md).

## Behavior

- Discovers Steam roots, additional libraries, game manifests, user configs and both Workshop branches.
- Uses the Go platform library names and launch flags; recursively discovers JSON launchers and selects the original Unix launcher candidates.
- Opens Workshop for unsubscribed branches. A subscribed item can wait for its non-empty payload and corresponding ACF record, if present, with a 60-second timeout and cancellation.
- Closes Steam gracefully before reading final configuration; patches DLL/SO/DYLIB/JAR, Steam options, JSON and BAT/shell launchers.
- Validates every transformation before writing; preserves first `.backup` files; atomically updates destinations; records a durable rollback journal and recovers unfinished changes on the next mutation.
- Locks native mutations across processes. Do not run the legacy Go installer concurrently because it cannot participate in the new lock.
- Reports progress and terminal errors, exposes diagnostics via F5/button, supports Escape and branch action menus, and keeps diagnostic history across restarts.
- Transactionally uninstalls managed files/flags while preserving unrelated current settings and original backups.

Close Project Zomboid before installing or uninstalling. If the Steam app entry has no `LaunchOptions`, set launch options in Steam first, matching the original installer requirement. Unsupported or ambiguous VDF is rejected.

Steam restart failure after a completed transaction is a warning; the successful installation remains. If rollback/recovery is incomplete, Steam remains stopped and the journal is preserved. Retry the installer before launching Steam; retain the journal and `.backup` files for manual recovery if retry fails.

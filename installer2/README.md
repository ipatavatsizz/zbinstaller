# ZombieBuddy Installer 2

Tauri 2 + Rust implementation of the [original Go installer](../installer/install.go). Rust contains the installer logic; the UI uses React/TypeScript in the native webview. Go is not required to build or run this application. Linux/macOS support is experimental; native CI includes real Tauri/WebKit UI tests. Installer acceptance checks file/configuration results while the game is closed; game or in-game mod launch is not an acceptance or public-release prerequisite. See the [test criteria and Steam safety precautions](tests/README.md) and [verified source/artifact evidence](docs/RELEASE.md).

## Develop

Use Node.js 22.12+, npm and stable Rust 1.89+ (dependencies may require a newer stable compiler). Windows needs the MSVC toolchain, Visual Studio C++ Build Tools and WebView2. Linux needs WebKitGTK 4.1 and the [Tauri native prerequisites](https://v2.tauri.app/start/prerequisites/); macOS needs Xcode command line tools.

```powershell
cd installer2
npm ci
npm run tauri dev
```

From the repository root in VS Code, use **F5 → Run ZombieBuddy Installer 2 (Tauri)**. It starts Vite and the Rust debug app. The Rust watcher is disabled to avoid locking the running Windows executable/PDB. Closing the main window ends the app; **Shift+F5** stops the debug session.

## Verify and build

```powershell
npm run build
npm run test:ui
npm run test:release
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build
```

This is a download-and-run tool: download the application, run it, then delete it when no longer needed. The tool itself needs no setup or uninstall wizard. Its Install/Uninstall actions manage the ZombieBuddy mod in the game. Diagnostic history and game backups may remain after deleting the tool.

`npm run tauri build` produces the following native applications:

| Platform | Local build output | CI download |
| --- | --- | --- |
| Windows x64 | `target/release/ZombieBuddyInstaller-26.11.3.exe` | `ZombieBuddyInstaller-windows-amd64.exe` |
| Linux x64 | `target/release/ZombieBuddyInstaller-26.11.3` | `ZombieBuddyInstaller-linux-amd64` |
| macOS arm64 | `target/release/bundle/macos/ZombieBuddy Installer.app` | `ZombieBuddyInstaller-macos-arm64.zip` containing the APP |

Windows requires the WebView2 runtime. On Linux, make the downloaded file executable (`chmod +x ZombieBuddyInstaller-linux-amd64`); the system must provide WebKitGTK 4.1 and its desktop dependencies. On macOS, unzip and open the APP. No setup installer, package-manager package or disk image is produced. A build with `--target` inserts the target triple between `target/` and `release/` in the paths above.

The [native workflow](../.github/workflows/installer2.yml) checks each native target and uploads the application downloads and separate UI evidence. Its manual action can create a draft prerelease with an `installer2-v...` candidate tag. The [tag release workflow](../.github/workflows/release.yml) reuses the same Rust checks/builds and prepares a draft with three applications and `SHA256SUMS`. Signing credentials are not supplied; see the [experimental release policy](docs/RELEASE.md). Public publishing is separate from creating the draft.

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

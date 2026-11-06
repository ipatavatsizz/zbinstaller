<p align="center">
  <img src="./website/assets/zombiebuddy.png" alt="ZombieBuddy Installer logo" width="160">
</p>

<h1 align="center">ZombieBuddy Installer</h1>

Desktop installer for the ZombieBuddy mod for **Project Zomboid**, implemented with Rust, Tauri and a React/TypeScript UI. Go is not required to build or run the Rust application. Linux and Apple Silicon macOS support is experimental.

This project builds on the original work by [zed-0xff](https://github.com/zed-0xff).

## Features

- Discovers Steam, additional libraries, Project Zomboid and both LTS/Beta Workshop branches.
- Opens the selected Workshop page and waits for completed payloads, with cancellation.
- Gracefully closes Steam before changing launch settings, then restarts it.
- Installs the platform native library and `ZombieBuddy.jar`; patches Steam options and JSON/BAT/shell launchers.
- Preserves original backups and unrelated user settings; applies atomic writes with rollback and crash recovery.
- Supports reinstall, branch changes and uninstall, with cross-process operation locks.
- Provides diagnostics, persistent logs, progress, keyboard navigation and branch menus.

## Download and run

The tool runs directly and does not install itself:

| Platform | Application | Requirement |
| --- | --- | --- |
| Windows x64 | `ZombieBuddyInstaller-windows-amd64.exe` | WebView2 runtime |
| Linux x64 (experimental) | `ZombieBuddyInstaller-linux-amd64` | `chmod +x`; WebKitGTK 4.1 and desktop dependencies |
| macOS arm64 (experimental) | `ZombieBuddyInstaller-macos-arm64.zip` | Unzip and open the APP |

Applications are unsigned; Windows SmartScreen/macOS Gatekeeper warnings may appear. New Rust candidates are prepared as draft prereleases. Existing public releases may still contain the legacy Go application until a Rust candidate is published. See the [release policy](docs/RELEASE.md).

Install Steam and Project Zomboid, close the game, run the tool and select LTS or Beta. Subscribe through the selected Workshop page if needed and wait for Steam's download. The native library and JAR must be supplied by Workshop; this tool does not create them.

Delete the downloaded tool when finished. Deleting it does not remove the mod, game backups or diagnostic history; use the mod uninstall action when needed.

## Development

Run all commands from the repository root.

### Run the app

Use Node.js 22.12+, npm and stable Rust 1.89+ (dependencies may require a newer stable compiler). Windows needs the MSVC toolchain, Visual Studio C++ Build Tools and WebView2. Linux needs WebKitGTK 4.1 and the [Tauri native prerequisites](https://v2.tauri.app/start/prerequisites/); macOS needs Xcode command line tools.

```powershell
npm ci
npm run tauri dev
```

From the repository root in VS Code, use **F5 → Run ZombieBuddy Installer 2 (Tauri)**. It starts Vite and the Rust debug app. The Rust watcher is disabled to avoid locking the running Windows executable/PDB. Closing the main window ends the app; **Shift+F5** stops the debug session.

### Verify and build

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

`npm run dev` alone serves the view; native commands require the Tauri app. See [test coverage](tests/README.md) and [architecture](docs/ARCHITECTURE.md).

Normal tests write only to temporary directories. Windows tests execute patched BAT launchers in real `cmd.exe` processes and exercise installation orchestration with an isolated child process. These tests do not modify the developer's Steam/game installation. Installer acceptance checks payload copies, launch options, reinstall/uninstall and backup/recovery results while the game is closed; launching Project Zomboid or the mod in-game is not an acceptance or public-release prerequisite. See the [test criteria and scoped Steam opt-in precautions](tests/README.md) and [verified release evidence](docs/RELEASE.md).

## Releases

The [native workflow](.github/workflows/installer2.yml) verifies frontend/Rust tests, lint, three native builds and experimental Linux/macOS UI acceptance. It uploads three directly runnable applications. A manual run can prepare an `installer2-v...` draft candidate.

The [version-tag workflow](.github/workflows/release.yml) uses the same Rust workflow and creates a draft prerelease containing the three applications and `SHA256SUMS`. It does not build Go or publish the draft automatically. Use a new tag whose commit contains the Rust application. Linux/macOS support remains experimental; Intel macOS is not included.

## Project layout

- [src/](src/): React/TypeScript application interface.
- [src-tauri/](src-tauri/): native application, capabilities and platform configuration.
- [crates/installer-core/](crates/installer-core/): Rust discovery, patching, transactions and recovery.
- [tests/](tests/): fixtures, UI, release and native acceptance checks.
- [scripts/](scripts/): release artifact validation.
- [docs/](docs/): architecture and historical release evidence.
- [website/](website/): independent static marketing page and its own assets. It needs no build; serve this folder when deploying the website. No Pages workflow is configured here.
- Root `index.html` and `diagnostics.html` are the Vite/Tauri application entry points.

The legacy Go source has been removed from the current tree; it remains available in repository history. Ignored local outputs in the former `installer/` and `installer2/` folders may remain on existing developer machines and are not part of the source layout.

## Behavior

- Discovers Steam roots, additional libraries, game manifests, user configs and both Workshop branches.
- Preserves the original platform library names and launch flags; recursively discovers JSON launchers and selects the original Unix launcher candidates.
- Opens Workshop for unsubscribed branches. A subscribed item can wait for its non-empty payload and corresponding ACF record, if present, with a 60-second timeout and cancellation.
- Closes Steam gracefully before reading final configuration; patches DLL/SO/DYLIB/JAR, Steam options, JSON and BAT/shell launchers.
- Validates every transformation before writing; preserves first `.backup` files; atomically updates destinations; records a durable rollback journal and recovers unfinished changes on the next mutation.
- Locks native mutations across processes. Do not run the legacy Go installer concurrently because it cannot participate in the new lock.
- Reports progress and terminal errors, exposes diagnostics via F5/button, supports Escape and branch action menus, and keeps diagnostic history across restarts.
- Transactionally uninstalls managed files/flags while preserving unrelated current settings and original backups.

Close Project Zomboid before installing or uninstalling. If the Steam app entry has no `LaunchOptions`, set launch options in Steam first, matching the original installer requirement. Unsupported or ambiguous VDF is rejected.

Steam restart failure after a completed transaction is a warning; the successful installation remains. If rollback/recovery is incomplete, Steam remains stopped and the journal is preserved. Retry the installer before launching Steam; retain the journal and `.backup` files for manual recovery if retry fails.


## Troubleshooting

Open diagnostics with F5 or the visible diagnostics button to inspect paths, Steam libraries, branch state and persistent logs. Ensure Steam has run at least once, the game is installed through Steam and the selected Workshop payload has finished downloading.

Close Project Zomboid before installing or uninstalling. If Steam's game entry lacks `LaunchOptions`, set launch options through Steam first, matching the original installer's requirement. Unsupported or ambiguous configuration is reported before writes.

If recovery is incomplete, Steam remains stopped and the transaction journal is preserved. Retry the installer before launching Steam. A Steam restart failure after a completed installation is reported without undoing that installation.

## Links

- [Project landing page](https://ipatavatsizz.github.io/zbinstaller)
- [Source repository](https://github.com/ipatavatsizz/zbinstaller)

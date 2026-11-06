<p align="center">
  <img src="./installer/icon.png" alt="ZombieBuddy Installer logo" width="160">
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

Applications are unsigned; Windows SmartScreen/macOS Gatekeeper warnings may appear. New Rust candidates are prepared as draft prereleases. Existing public releases may still contain the legacy Go application until a Rust candidate is published. See the [release policy](installer2/docs/RELEASE.md).

Install Steam and Project Zomboid, close the game, run the tool and select LTS or Beta. Subscribe through the selected Workshop page if needed and wait for Steam's download. The native library and JAR must be supplied by Workshop; this tool does not create them.

Delete the downloaded tool when finished. Deleting it does not remove the mod, game backups or diagnostic history; use the mod uninstall action when needed.

## Build and verify

The current application is in [installer2/](installer2/). Requirements: Node.js 22.12+, stable Rust and the platform's [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Windows needs Visual Studio C++ Build Tools and WebView2. Linux/macOS applications are built on their native GitHub Actions runners.

```powershell
cd installer2
npm ci
npm run build
npm run test:ui
npm run test:release
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build
```

On Windows, the executable is `installer2/target/release/ZombieBuddyInstaller-26.11.3.exe`. Run `npm run test:native:windows` inside the application directory to validate the real release WebView2 window. The [development guide](installer2/README.md) lists all native output paths and commands.

Normal tests write only to temporary directories. Windows tests execute patched BAT launchers in real `cmd.exe` processes and exercise installation orchestration with an isolated child process. These tests do not modify the developer's Steam/game installation. Installer acceptance checks payload copies, launch options, reinstall/uninstall and backup/recovery results while the game is closed; launching Project Zomboid or the mod in-game is not an acceptance or public-release prerequisite. See the [test criteria and scoped Steam opt-in precautions](installer2/tests/README.md) and [verified release evidence](installer2/docs/RELEASE.md).

## Releases

The [native workflow](.github/workflows/installer2.yml) verifies frontend/Rust tests, lint, three native builds and experimental Linux/macOS UI acceptance. It uploads three directly runnable applications. A manual run can prepare an `installer2-v...` draft candidate.

The [version-tag workflow](.github/workflows/release.yml) uses the same Rust workflow and creates a draft prerelease containing the three applications and `SHA256SUMS`. It does not build Go or publish the draft automatically. Use a new tag whose commit contains the Rust application. Linux/macOS support remains experimental; Intel macOS is not included.

## Project layout

- [installer2/crates/installer-core/](installer2/crates/installer-core/): Rust discovery, patching, process control, transactions and recovery.
- [installer2/src-tauri/](installer2/src-tauri/): native application and narrow Tauri commands.
- [installer2/src/](installer2/src/): React/TypeScript interface.
- [installer2/tests/](installer2/tests/): fixtures, process/launcher scenarios, UI and release tests.
- [installer/](installer/): legacy Go/Fyne source, kept as a reference and fallback. Its old Go/Rake build commands are not required by the Rust application.
- [index.html](index.html): project landing page.

## Troubleshooting

Open diagnostics with F5 or the visible diagnostics button to inspect paths, Steam libraries, branch state and persistent logs. Ensure Steam has run at least once, the game is installed through Steam and the selected Workshop payload has finished downloading.

Close Project Zomboid before installing or uninstalling. If Steam's game entry lacks `LaunchOptions`, set launch options through Steam first, matching the original installer's requirement. Unsupported or ambiguous configuration is reported before writes.

If recovery is incomplete, Steam remains stopped and the transaction journal is preserved. Retry the installer before launching Steam. A Steam restart failure after a completed installation is reported without undoing that installation.

## Links

- [Project landing page](https://ipatavatsizz.github.io/zbinstaller)
- [Source repository](https://github.com/ipatavatsizz/zbinstaller)

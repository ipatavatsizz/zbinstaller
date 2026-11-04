<p align="center">
  <img src="./installer/icon.png" alt="ZombieBuddy Installer logo" width="160">
</p>

<h1 align="center">ZombieBuddy Installer</h1>

Cross-platform desktop installer for the ZombieBuddy mod for **Project Zomboid**. The installer is built with [Fyne](https://fyne.io/) and integrates with Steam to discover the game installation, manage Workshop branches, and apply the required launch configuration.

This project is built on top of the original work by [zed-0xff](https://github.com/zed-0xff).

## Features

- Detects Steam and Project Zomboid installations automatically.
- Supports the **LTS** and **Beta** ZombieBuddy Workshop branches.
- Opens the selected branch's Workshop page in Steam when a subscription is required.
- Polls Steam Workshop status and waits for the selected item to finish downloading.
- Closes Steam before changing its launch configuration and starts it again after installation.
- Installs the native ZombieBuddy library and `ZombieBuddy.jar`.
- Patches:
  - Steam launch options.
  - `ProjectZomboid64.json`.
  - The Project Zomboid launcher script or batch file.
- Creates a rollback snapshot before modifying files.
- Restores the previous files automatically when installation fails.
- Includes a diagnostics window with installation status, paths, Steam libraries, and an installation log.
- Provides a context menu for opening each branch's Workshop page.
- Uses atomic file replacement to reduce the risk of partially written configuration files.

## Supported platforms

The installer contains platform-specific support for:

- Windows
- Linux
- macOS (Apple Silicon)

Steam discovery includes common installation locations on each platform, including standard Linux, Flatpak, Snap, and macOS locations where applicable.

The native ZombieBuddy libraries must be available in the selected Workshop branch. The installer does not build or create those libraries.

## Installation

1. Install Steam and Project Zomboid.
2. Download the installer binary for your platform.
3. Run the installer.
4. Select **LTS** or **Beta**.
5. If necessary, subscribe to the selected Workshop branch in Steam.
6. Wait while Steam finishes downloading the Workshop item.
7. Start the installation.

The downloaded tool runs directly; it does not install itself or require a setup wizard. Delete the tool when you no longer need it. Its install/uninstall actions manage the ZombieBuddy mod; deleting the tool does not remove the mod, game backups or diagnostic history.

The installer may close Steam while it updates launch settings. Steam is started again after a successful installation.

Before installing, close Project Zomboid and any other process that may be using its files. The installer creates a rollback snapshot, but keeping a personal backup of important files is still recommended.

## Building from source

The new Tauri + Rust implementation is in [`installer2/`](./installer2). Windows, Linux and macOS installation logic is implemented with fixture tests, persistent backups/recovery and process locks. See its [development guide](./installer2/README.md) and numbered [migration checklist](./installer2/docs/MIGRATION.md). The [Installer 2 workflow](./.github/workflows/installer2.yml) verifies and builds directly runnable applications: Windows EXE, Linux executable and macOS APP in a ZIP. Real Steam/game and native platform acceptance are required before replacing the Go release workflow below.

The Go module is located in [`installer/`](./installer).

Requirements:

- Go `1.25.5` or compatible
- A C toolchain and the platform dependencies required by Fyne/GLFW
- Ruby and Rake for the provided build tasks

From the repository root:

```powershell
cd installer
go run .
```

Run the test suite:

```powershell
cd installer
go test ./...
```

Run static analysis:

```powershell
cd installer
go vet ./...
```

## Build commands

The `Rakefile` provides these tasks from the `installer` directory:

```text
rake                Build for the current host and run tests
rake build          Build for the current host
rake build_windows  Build ZombieBuddyInstaller.exe
rake build_linux    Build ZombieBuddyInstaller-linux-amd64
rake build_macos    Build ZombieBuddyInstaller-macos-arm64
rake build_all      Build Windows, Linux, and macOS artifacts
rake test           Run go test ./...
```

The output binaries are written to the `installer` directory.

## Automated releases

Pushing a date-based version tag such as `v26.11.3` starts the GitHub Actions release workflow. It builds native Windows amd64, Linux amd64, and macOS arm64 artifacts on the current GitHub-hosted `latest` runners and publishes them to one GitHub Release.

Linux and macOS builds are experimental and may not work on every distribution or system. The macOS release targets Apple Silicon (arm64); Intel macOS is not included. A local Linux or macOS machine is not required for the release build.

The workflow can also be started manually for an existing tag from the **Actions** tab. This is useful for tags created before the workflow was added.

## Project layout

```text
.
├── index.html                    Project landing page
└── installer/
    ├── installer.go              Fyne UI and user interaction
    ├── install.go                Installation and rollback workflow
    ├── steam.go                  Shared Steam and Workshop logic
    ├── steam_windows.go          Windows Steam integration
    ├── steam_other.go            Linux/macOS Steam integration
    ├── patchers.go               Shared configuration patchers
    ├── patchers_windows.go       Windows launcher patching
    ├── patchers_unix.go          Linux/macOS launcher patching
    └── *_test.go                 Unit and integration-style tests
```

## Troubleshooting

Open the installer's diagnostics window to inspect detected paths, Steam libraries, launcher files, and the installation log.

If Steam or Project Zomboid is not detected:

- Make sure Steam is installed and has been launched at least once.
- Make sure Project Zomboid is installed through Steam.
- Check that the selected Workshop branch has finished downloading.
- On Linux or macOS, verify that the Steam and Project Zomboid files are accessible to the current user.

If the native library is missing, wait for the Workshop download to complete and try again. The installer requires the platform-specific native library and `ZombieBuddy.jar` before it starts modifying the game installation.

## Development notes

Tests are kept beside the Go source files so they can directly exercise package-private helpers with the standard `go test ./...` workflow.

Cross-compiling the Fyne application may require a native C/GUI toolchain for the target platform. A successful build on Windows alone does not replace testing the Linux and macOS binaries on their respective systems.

## Links

- [Project landing page](https://ipatavatsizz.github.io/zbinstaller)
- [Source repository](https://github.com/ipatavatsizz/zbinstaller)

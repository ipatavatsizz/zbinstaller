# Architecture

## Dependency direction

```text
React view → typed IPC adapter (src/api.ts) → thin Tauri commands → installer-core
                                                                  ├ discovery / target
                                                                  ├ platform processes
                                                                  ├ pure patchers / VDF
                                                                  ├ installation lock
                                                                  └ file transaction / recovery
```

The core has no Tauri dependency. Explicit target platforms and paths allow Windows, Linux and macOS layouts to be tested on one host without accessing its Steam installation. Native entry points select the current platform. Compatibility wrappers for the original Windows byte patchers remain available; application code uses the explicit target variants.

## Operation lifetime

Tauri owns a per-process atomic guard, blocking workers and progress events. Each native mutation also acquires an OS file lock in the Steam directory. Disk transactions acquire a game-directory lock, so independently launched installers cannot write the same installation concurrently. Lock files are kept in place; dropping the handle or process termination releases the lock. The legacy Go executable does not participate in these locks.

Native installation, uninstall and manual diagnostic patches share game checks, graceful Steam shutdown, post-shutdown rediscovery, transactional changes and Steam restart handling. Window close/Escape is rejected while an operation is running. Workshop launch also acquires the Steam lock. Read-only discovery remains available while waiting.

## Discovery and platform layout

Windows reads the Steam registry views. Linux includes conventional, Flatpak and Snap roots; macOS uses Application Support and system/user Steam.app executables. Steam library indices and game manifests are parsed without rewriting them. Missing game installations can still produce partial diagnostic reports.

`target.rs` selects native library names, JVM flags and launchers. JSON discovery searches the root first, then sorted descendants, without following directory symlinks. Unix launcher selection preserves the Go order: `.sh`, `.command`, then `start-server.sh`. Copy destinations remain in the game root, matching Go.

## Transaction boundary

1. Discover Steam/game/payload; ensure the game is closed. Poll for at most 60 seconds, with cancellation. Rescan libraries during the wait. Payload files must be non-empty; if an ACF exists, the installed record must be in the payload's own library.
2. Close Steam gracefully and recheck the game. Rediscover paths after Steam flushes its final configuration.
3. Recover any unfinished journal, validating its target paths against the managed game/config files.
4. Read all patch inputs and payloads. Prepare every transformation and any absent first-original `.backup` files before the first write.
5. Snapshot all destinations, including permissions and previously absent files, into a synced on-disk journal in the game directory.
6. Replace destinations via synced temporary files in their own directory. Preserve existing file permissions and sync parent directories on Unix.
7. On an error, restore all attempted destinations in reverse order and aggregate recovery failures. A successful rollback removes newly created payloads/backups.
8. Remove the journal after commit. If the process dies before commit, the next mutating operation restores the snapshots before preparing changes.
9. Restart Steam after success or complete rollback. A restart failure after commit is a warning. Incomplete recovery leaves Steam stopped and preserves the journal for retry/manual recovery.

The journal is not a general filesystem API. Recovery accepts only the selected game's known targets and discovered user configs/backups. As with any file transaction, storage hardware/filesystem durability limits still apply. The original `.backup` files remain for manual restoration and are never silently replaced.

## Patching and uninstall

JSON install changes only root `vmArgs`; uninstall removes matching mod arguments in root and nested `vmArgs` while preserving unrelated fields. BAT patching preserves line endings and existing `_JAVA_OPTIONS`. Unix injection preserves the shebang and carries the existing environment options forward.

VDF editing uses byte spans and preserves other apps/configuration bytes. Quoted/unquoted strings, BOM, comments and escaped quotes are supported. Ambiguous target objects and missing/duplicate `LaunchOptions` fail before writing; includes and conditional directives are rejected. Uninstall removes managed options rather than restoring old backups over later user edits.

## UI, diagnostics and permissions

The main window is fixed at 680 × 540 logical pixels, with resizing, maximizing and minimizing disabled. The author link is centered at the bottom of the content, immediately above the activity panel; both participate in the page layout so longer status messages do not cover the link. The main window owns the busy state for both branches, starts installation even when a subscribed payload is still downloading, supports cancellation during that wait, and converts the eight backend steps into percentages. Terminal results clear the progress state. Branch actions are reachable with context menus, hover, focus and an explicit menu button. F5 reopens the same native diagnostics window; Escape closes an idle window.

Diagnostics shows partial discovery, user config paths, library paths, launcher/payload presence, branch versions and session messages. Tauri persists bounded diagnostic history under its application log directory; clearing the log clears disk history too. No installation is performed automatically at startup.

The webview has no arbitrary filesystem or shell permission. Native inputs are typed branch/action enums. Browser URL permission is scoped to the three fixed project links.

The tool is distributed as a directly runnable application, without a setup wizard: Windows EXE, Linux executable, macOS APP archived in a ZIP. Tauri bundling is disabled on Windows/Linux and limited to the APP on macOS. CI collects only the application downloads; no system installer is generated. The tool can be deleted after use, while its diagnostic history, installed mod and game backups have separate lifetimes. WebView2 on Windows and system WebKitGTK 4.1 on Linux remain runtime prerequisites. Installer acceptance verifies payload/configuration results, idempotent reinstall, settings-preserving uninstall and backup/recovery with the game closed; game or in-game mod launch is not an acceptance or public-release prerequisite. See the [test criteria and scoped Steam safeguards](../tests/README.md) and [release policy/evidence](RELEASE.md).

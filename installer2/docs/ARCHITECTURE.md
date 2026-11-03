# Architecture

## Dependency direction

```text
TypeScript view -> typed IPC adapter -> Tauri commands -> installer-core
                                                        | discovery
                                                        | platform
                                                        | pure patchers / VDF
                                                        | file transaction
```

`installer-core` owns the install workflow. It receives a branch enum and a progress callback, returning a contextual error or completion message. It can be tested without building a webview. There are no shared mutable Steam/game path globals.

The Tauri layer owns operation lifetime and event transport. Its atomic operation guard moves into the blocking worker and resets on drop, including error/panic unwinding. Both installation and Workshop launch acquire this guard. The frontend disables controls for usability; the backend independently enforces exclusivity.

The resizable main window presents the install branches and a fixed activity panel. Diagnostics opens from its dedicated controls and uses a reusable secondary native window backed by a second Vite entry point. That window reads live install status and paths and exposes the manual patch actions.

## Transaction boundary

1. Discover the game and Workshop payload. Check the game is closed.
2. Poll each Steam library's `steamapps/workshop/appworkshop_108600.acf` every two seconds until the selected Workshop ID is present under `WorkshopItemsInstalled`, then confirm its DLL/JAR payload files are non-empty.
3. Request graceful Steam shutdown and verify the process exited.
4. Read all inputs and prepare all transformations without writing.
5. Snapshot every destination, including previously absent files.
6. Replace each destination using a synced temporary file in the same directory.
7. On failure, restore all attempted targets in reverse order; aggregate rollback failures.
8. Restart Steam after completion or rollback. A restart failure after commit does not roll back a successful installation.

Snapshots are in memory. Crash recovery and durable journaling are follow-up work, not guarantees of this implementation. Persistent Go `.backup` files are not reproduced yet.

## Patching

JSON transformations target only the root `vmArgs`. Batch patching preserves newline style and existing `_JAVA_OPTIONS`. VDF parsing tracks quoted value byte ranges, nesting, comments and escaped quotes; only the selected app's launch-option value is replaced. Unsupported or ambiguous data fails before any files change. VDF conditions/directives and unquoted tokens are intentionally unsupported in this initial strict reader.

## Security and platform boundary

The webview receives no general shell or filesystem permission. The backend accepts only a branch enum and derives paths from local Steam metadata. Workshop URLs use fixed IDs. Registry and process concerns stay in `platform.rs`. The current patch flags and payload names are Windows-specific; unsupported platforms return a clear error.

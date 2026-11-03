# Go to Rust migration

## Implemented for Windows

| Go responsibility | Rust owner |
| --- | --- |
| `InstallZB` orchestration | `install.rs` |
| `installSnapshot`, `CopyFile`, atomic writes | `transaction.rs` |
| Library, manifest and Workshop discovery | `discovery.rs`, `vdf.rs` |
| Registry and Steam process operations | `platform.rs` |
| Steam, JSON and batch patchers | `patch.rs` |
| Fyne controls and diagnostic display | Tauri commands + TypeScript view |

Deliberate changes: graceful shutdown replaces forced Steam termination; snapshots follow Steam shutdown; patch validation precedes writes; restart failure after commit leaves installed files intact; malformed launch settings fail explicitly.

## Acceptance still required

- [ ] Manual Windows WebView2/UI smoke test.
- [ ] Real installation in an expendable Steam/game environment, both branches and repeated installs.
- [ ] Verify Steam shutdown/restart and Workshop downloads with current Steam builds.
- [ ] Signed release, packaging and CI release workflow migration.
- [ ] Linux and macOS native adapters, library names, launch flags and script patching.
- [ ] Recursive launcher discovery for nonstandard game layouts (current port expects files at game root).
- [ ] Persistent original backups and crash-recovery journal.
- [ ] OS-wide single-instance/install lock (current guard is per process).
- [ ] Broader fixtures from real Steam VDF variants and Go tests.
- [ ] Persistent diagnostics across application restarts.

Do not replace the existing Go release pipeline until the required platform acceptance work is complete.

## Initial validation (2026-11-03, Windows x64)

- `npm run build`: passed (TypeScript and Vite production assets).
- `cargo fmt --all -- --check`: passed.
- `cargo test -p installer-core`: 6 tests passed.
- `cargo check -p installer2`: passed (Tauri application and Windows dependencies).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npx tauri info`: MSVC, Rust and WebView2 prerequisites detected.
- Follow-up UI work: legacy main-window layout, separate F5 diagnostics window and centered/focused startup are implemented; run a manual WebView2 smoke test to confirm Windows focus behavior.

No real Steam/game files were modified during validation. Native UI launch, NSIS packaging and end-to-end installation remain unverified.

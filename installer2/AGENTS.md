# Installer 2 development rules

- Keep Rust business logic in `crates/installer-core`; it must not depend on Tauri.
- Keep Tauri commands thin: validate typed inputs, enforce operation exclusivity, run blocking work off the UI thread, translate errors and emit events.
- Keep frontend native calls in `src/api.ts`. Never expose arbitrary filesystem, shell or URL execution to the webview.
- Separate pure byte/text transformations from filesystem writes and process management.
- Prepare and validate every patch before the first write. Capture snapshots after Steam closes. Every new write target must participate in rollback.
- Preserve unrelated launch options and configuration fields. Patches must be idempotent. Reject unsupported input rather than guessing.
- Avoid global mutable installation paths and monolithic utility modules. Use explicit parameters and narrow module responsibilities.
- Add regression tests for patch formats, discovery and transaction failure behavior. Use temporary directories, never the developer's Steam/game files.
- Run `npm run build`, `cargo fmt --all -- --check`, `cargo test -p installer-core`, and `cargo clippy --workspace --all-targets -- -D warnings` after relevant changes. Record any checks blocked by the environment.
- Update `docs/MIGRATION.md` when support or behavioral parity changes. Do not claim Linux/macOS or release readiness without native verification.
- Never test installation against the developer's actual game or close Steam as an automated test.

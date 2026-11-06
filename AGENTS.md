# Personal multi-agent development workflow

When the user requests a multi-agent development cycle, use the configured planner (gpt-6.1-sol, medium, read-only), coder (gpt-6.1-sol, medium) and tester (gpt-6.1-terra, medium) roles. If only generic delegation is available, explicitly select the matching model with reasoning_effort="medium" and fork_turns="none" and include role instructions and task context. Never request effort above medium or reuse a differently configured agent.

Obtain approval of the concrete plan unless already explicitly approved. Execute steps sequentially: Planner -> user approval -> Coder -> Tester -> Coder fixes -> Tester verifies. Do not start the next step before relevant acceptance scenarios pass. Coordinate feedback internally. If an essential check cannot run, report the blocker without claiming completion.

Keep the workflow scoped to the current task. This file alone does not authorize a development cycle, extra workers or unrelated changes. Preserve project instructions, unrelated edits and the user's latest corrections.

# Application development rules

- Distribute this tool as a directly runnable application, without installing the tool itself: Windows EXE, Linux executable, macOS APP in a ZIP. Do not add setup installers, package-manager packages or disk-image installation steps. Mod installation/uninstallation remains the tool's purpose. Persistent diagnostics and game backups are separate from the downloaded tool.

- Keep Rust business logic in `crates/installer-core`; it must not depend on Tauri.
- Keep Tauri commands thin: validate typed inputs, enforce operation exclusivity, run blocking work off the UI thread, translate errors and emit events.
- Keep frontend native calls in `src/api.ts`. Never expose arbitrary filesystem, shell or URL execution to the webview.
- Separate pure byte/text transformations from filesystem writes and process management.
- Prepare and validate every patch before the first write. Capture snapshots after Steam closes. Every new write target must participate in rollback.
- Preserve unrelated launch options and configuration fields. Patches must be idempotent. Reject unsupported input rather than guessing.
- Avoid global mutable installation paths and monolithic utility modules. Use explicit parameters and narrow module responsibilities.
- Add regression tests for patch formats, discovery and transaction failure behavior. Default tests use temporary directories. The explicitly authorized opt-in native Steam config test below is the only scoped exception for real Steam configuration; game files remain excluded.
- Run `npm run build`, `cargo fmt --all -- --check`, `cargo test -p installer-core`, and `cargo clippy --workspace --all-targets -- -D warnings` after relevant changes. Record any checks blocked by the environment.
- Update `tests/README.md` when installer behavior/test coverage changes and `docs/RELEASE.md` when platform support or release evidence changes. Keep source/artifact identity and actual native verification limits explicit.
- Never test installation against the developer's actual game or modify/launch the real game for tests. Game installation, patching, download and rollback test data must stay in isolated test directories. Installer acceptance verifies payload copies, launch options, idempotent reinstall, uninstall preserving unrelated settings, backups and recovery while the game is closed. Launching Project Zomboid or the mod in-game is not an installer-acceptance or public-release prerequisite.
- The user authorizes scoped Steam operations for the opt-in real Steam config roundtrip under the safeguards below. Predetermine and record the exact config and original backup targets before any write; reject unexpected targets. Do not change out-of-scope accounts, subscriptions or downloads, and do not touch game files.
- Before writing real Steam config, gracefully close Steam and capture independent, persistent, restorable backups outside the target tree. Record original file existence, bytes/hashes and relevant permissions for every config and original `.backup` target, including absent files. Verify the backup before the first write; an in-memory snapshot or the application's `.backup` alone is insufficient.
- Keep real Steam process/config tests explicitly opt-in and excluded from default tests/CI. Run only the exact approved test, never blanket `--ignored`. Restore all transient config changes and original backup files (or their original absence) after success or failure, verify restoration before reopening Steam, and retain independent recovery evidence if cleanup fails. A cleanup failure must fail the test and must not be hidden by a successful patch assertion.
- Restore Steam's initial running state using normal shutdown/startup, including on failure where possible; never force-kill it. Steam's own normal startup/shutdown writes are not installer test writes. If file restoration cannot be verified, report the recovery paths and failure and do not automatically reopen Steam over uncertain config.
- Linux/macOS native GitHub Actions runners are accepted native verification environments; separate manual desktop machines are not required. Keep WebDriver test-build UI results separate from production-artifact visible-window startup evidence. Do not claim unexecuted checks passed. Public publishing requires a separate decision.

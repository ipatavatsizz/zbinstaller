//! Production orchestration against disposable files and a real, isolated child
//! process. The child models shutdown-time config writes, not the Steam client.
use super::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    process::{Child, Command, Stdio},
};

const CONFIG: &str = r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "108600" { "LaunchOptions" "-debug" } "42" { "LaunchOptions" "keep-other-game" } } } } } }"#;

struct Fixture {
    directory: tempfile::TempDir,
    game: PathBuf,
    config: PathBuf,
    workshop: PathBuf,
}

impl Fixture {
    fn new(branch: Branch, ready: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let game = root.join("steamapps/common/Test Game");
        let config = root.join("userdata/42/config/localconfig.vdf");
        let workshop = root
            .join("steamapps/workshop/content/108600")
            .join(branch.workshop_id());
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::create_dir_all(workshop.join("mods/ZombieBuddy/libs")).unwrap();
        fs::write(
            root.join("steamapps/appmanifest_108600.acf"),
            "AppState { installdir \"Test Game\" }",
        )
        .unwrap();
        fs::write(&config, CONFIG).unwrap();
        fs::write(
            game.join("ProjectZomboid64.json"),
            br#"{"vmArgs":["-Xmx4G"],"userSetting":"keep"}"#,
        )
        .unwrap();
        fs::write(
            game.join("ProjectZomboid64.bat"),
            b"@echo off\r\nSET _JAVA_OPTIONS=-Xmx4G\r\n",
        )
        .unwrap();
        let fixture = Self {
            directory,
            game,
            config,
            workshop,
        };
        if ready {
            fixture.finish_download();
        }
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn finish_download(&self) {
        for name in TargetPlatform::Windows.files() {
            fs::write(self.workshop.join("mods/ZombieBuddy/libs").join(name), name).unwrap();
        }
    }

    fn state(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(root).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    visit(&entry.path(), output);
                } else if entry.file_name() != ".zombiebuddy-installer.lock" {
                    output.insert(entry.path(), fs::read(entry.path()).unwrap());
                }
            }
        }
        let mut output = BTreeMap::new();
        visit(&self.game, &mut output);
        visit(&self.root().join("userdata"), &mut output);
        output
    }
}

struct ProcessRuntime<'a> {
    fixture: &'a Fixture,
    child: RefCell<Option<Child>>,
    starts: Cell<usize>,
    stops: Cell<usize>,
    writes: Cell<usize>,
    fail_write: Cell<Option<usize>>,
    refuse_shutdown: Cell<bool>,
    fail_restart: Cell<bool>,
    game_running: Cell<bool>,
    game_starts_on_shutdown: Cell<bool>,
}

impl<'a> ProcessRuntime<'a> {
    fn new(fixture: &'a Fixture) -> Self {
        let runtime = Self {
            fixture,
            child: RefCell::new(None),
            starts: Cell::new(0),
            stops: Cell::new(0),
            writes: Cell::new(0),
            fail_write: Cell::new(None),
            refuse_shutdown: Cell::new(false),
            fail_restart: Cell::new(false),
            game_running: Cell::new(false),
            game_starts_on_shutdown: Cell::new(false),
        };
        runtime.spawn().unwrap();
        runtime
    }

    fn spawn(&self) -> Result<()> {
        let root = self.fixture.root();
        for name in ["actor-ready", "actor-stop"] {
            let _ = fs::remove_file(root.join(name));
        }
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args([
                "--exact",
                "install::scenario_tests::steam_fixture_process",
                "--nocapture",
            ])
            .env("ZB_SCENARIO_ROOT", root)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        *self.child.borrow_mut() = Some(command.spawn()?);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.join("actor-ready").exists() {
            ensure!(Instant::now() < deadline, "fixture process failed to start");
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    fn running(&self) -> bool {
        self.child
            .borrow_mut()
            .as_mut()
            .is_some_and(|child| child.try_wait().unwrap().is_none())
    }
}

impl InstallRuntime for ProcessRuntime<'_> {
    fn steam_root(&self) -> Result<PathBuf> {
        Ok(self.fixture.root().to_path_buf())
    }
    fn ensure_game_closed(&self) -> Result<()> {
        ensure!(
            !self.game_running.get(),
            "Close Project Zomboid before installing"
        );
        Ok(())
    }
    fn stop_steam(&self, _: &Path) -> Result<()> {
        self.stops.set(self.stops.get() + 1);
        ensure!(
            !self.refuse_shutdown.get(),
            "Steam is still running; close it manually and retry"
        );
        fs::write(self.fixture.root().join("actor-stop"), b"shutdown")?;
        if let Some(mut child) = self.child.borrow_mut().take() {
            ensure!(
                child.wait()?.success(),
                "fixture process failed during shutdown"
            );
        }
        self.game_running.set(self.game_starts_on_shutdown.get());
        Ok(())
    }
    fn start_steam(&self, _: &Path) -> Result<()> {
        self.starts.set(self.starts.get() + 1);
        ensure!(!self.fail_restart.get(), "injected restart failure");
        self.spawn()
    }
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        assert!(
            !self.running(),
            "no installer writes while the fixture process is running"
        );
        self.writes.set(self.writes.get() + 1);
        // Fail after an actual replacement to exercise rollback of attempted writes.
        transaction::atomic_write(path, bytes)?;
        ensure!(
            self.fail_write.get() != Some(self.writes.get()),
            "injected disk write failure"
        );
        Ok(())
    }
    fn download_timing(&self) -> (Duration, Duration) {
        (Duration::from_millis(100), Duration::from_millis(5))
    }
}

impl Drop for ProcessRuntime<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.fixture.root().join("flush-on-stop"));
        let _ = fs::write(self.fixture.root().join("actor-stop"), b"cleanup");
        if let Some(mut child) = self.child.get_mut().take() {
            let _ = child.wait();
        }
    }
}

#[test]
fn steam_fixture_process() {
    let Some(root) = std::env::var_os("ZB_SCENARIO_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    fs::write(root.join("actor-ready"), b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !root.join("actor-stop").exists() {
        assert!(Instant::now() < deadline, "fixture parent did not clean up");
        thread::sleep(Duration::from_millis(5));
    }
    if root.join("flush-on-stop").exists() {
        fs::write(
            root.join("userdata/42/config/localconfig.vdf"),
            CONFIG.replace("-debug", "-debug -last-steam-setting"),
        )
        .unwrap();
        let late = root.join("userdata/99/config/localconfig.vdf");
        fs::create_dir_all(late.parent().unwrap()).unwrap();
        fs::write(late, CONFIG.replace("-debug", "-late-user")).unwrap();
    }
}

fn install(fixture: &Fixture, branch: Branch, runtime: &ProcessRuntime<'_>) -> Result<String> {
    assert_eq!(fixture.root(), runtime.fixture.root());
    install_with_runtime(branch, TargetPlatform::Windows, |_| {}, || false, runtime)
}

#[test]
fn download_completion_flush_and_new_user_are_seen_before_installation() {
    for branch in [Branch::Lts, Branch::Beta] {
        let fixture = Fixture::new(branch, false);
        let runtime = ProcessRuntime::new(&fixture);
        fs::write(fixture.root().join("flush-on-stop"), b"yes").unwrap();
        let mut polls = 0;
        let result = install_with_runtime(
            branch,
            TargetPlatform::Windows,
            |progress| {
                if progress.message.starts_with("Waiting") {
                    polls += 1;
                    if polls == 1 {
                        fs::write(
                            fixture.workshop.join("mods/ZombieBuddy/libs/zbNative.dll"),
                            b"partial",
                        )
                        .unwrap();
                    } else {
                        fixture.finish_download();
                    }
                    assert_eq!(runtime.stops.get(), 0);
                    assert_eq!(runtime.writes.get(), 0);
                }
            },
            || false,
            &runtime,
        )
        .unwrap();
        assert!(result.contains("Steam is starting"));
        assert_eq!(polls, 2);
        assert_eq!(runtime.starts.get(), 1);
        assert!(runtime.running());
        for (path, setting) in [
            (&fixture.config, "-last-steam-setting"),
            (
                &fixture.root().join("userdata/99/config/localconfig.vdf"),
                "-late-user",
            ),
        ] {
            let patched = fs::read_to_string(path).unwrap();
            let original = fs::read_to_string(transaction::backup_path(path)).unwrap();
            assert!(patched.contains(setting) && patched.contains("-agentlib:zbNative --"));
            assert!(original.contains(setting) && !original.contains("-agentlib:zbNative"));
            assert!(patched.contains("keep-other-game"));
        }
        assert_eq!(
            fs::read(fixture.game.join("zbNative.dll")).unwrap(),
            b"zbNative.dll"
        );
        assert!(!fixture.game.join(transaction::JOURNAL).exists());
    }
}

#[test]
fn missing_subscription_cancel_and_download_timeout_do_not_stop_or_write() {
    for branch in [Branch::Lts, Branch::Beta] {
        for scenario in ["missing", "cancel", "timeout"] {
            let fixture = Fixture::new(branch, false);
            if scenario == "missing" {
                fs::remove_dir_all(&fixture.workshop).unwrap();
            }
            let runtime = ProcessRuntime::new(&fixture);
            let before = fixture.state();
            let cancel = Cell::new(false);
            let error = install_with_runtime(
                branch,
                TargetPlatform::Windows,
                |progress| {
                    if scenario == "cancel" && progress.message.starts_with("Waiting") {
                        cancel.set(true);
                    }
                },
                || cancel.get(),
                &runtime,
            )
            .unwrap_err();
            let expected = match scenario {
                "missing" => "not subscribed",
                "cancel" => "cancelled",
                _ => "did not finish",
            };
            assert!(error.to_string().contains(expected), "{error:#}");
            assert_eq!(fixture.state(), before);
            assert_eq!(
                (
                    runtime.stops.get(),
                    runtime.starts.get(),
                    runtime.writes.get()
                ),
                (0, 0, 0)
            );
            assert!(runtime.running());
        }
    }
}

#[test]
fn shutdown_timeout_and_game_start_race_never_patch_files() {
    for branch in [Branch::Lts, Branch::Beta] {
        for scenario in ["timeout", "game-before", "game-during-shutdown"] {
            let fixture = Fixture::new(branch, true);
            let runtime = ProcessRuntime::new(&fixture);
            runtime.refuse_shutdown.set(scenario == "timeout");
            runtime.game_running.set(scenario == "game-before");
            runtime
                .game_starts_on_shutdown
                .set(scenario == "game-during-shutdown");
            let before = fixture.state();
            assert!(install(&fixture, branch, &runtime).is_err());
            assert_eq!(fixture.state(), before);
            assert_eq!(runtime.writes.get(), 0);
            assert_eq!(
                runtime.starts.get(),
                usize::from(scenario == "game-during-shutdown")
            );
            assert!(runtime.running());
        }
    }
}

#[test]
fn disk_failure_rolls_back_every_attempted_write_and_restarts_the_process() {
    for branch in [Branch::Lts, Branch::Beta] {
        // Every install write, including payloads, backups and the last launcher.
        for failure in 1..=8 {
            let fixture = Fixture::new(branch, true);
            let runtime = ProcessRuntime::new(&fixture);
            let before = fixture.state();
            runtime.fail_write.set(Some(failure));
            let error = install(&fixture, branch, &runtime).unwrap_err();
            assert!(format!("{error:#}").contains("original files restored"));
            assert_eq!(
                fixture.state(),
                before,
                "branch {} write {failure}",
                branch.workshop_id()
            );
            assert_eq!(runtime.starts.get(), 1);
            assert!(runtime.running());
        }
    }
}

#[test]
fn rollback_preserves_shutdown_time_settings_and_reports_restart_errors() {
    for branch in [Branch::Lts, Branch::Beta] {
        let fixture = Fixture::new(branch, true);
        let runtime = ProcessRuntime::new(&fixture);
        fs::write(fixture.root().join("flush-on-stop"), b"yes").unwrap();
        runtime.fail_write.set(Some(4));
        runtime.fail_restart.set(true);
        let error = install(&fixture, branch, &runtime).unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains("original files restored")
                && message.contains("Steam also failed to restart")
        );
        assert_eq!(
            fs::read_to_string(&fixture.config).unwrap(),
            CONFIG.replace("-debug", "-debug -last-steam-setting")
        );
        assert_eq!(
            fs::read_to_string(fixture.root().join("userdata/99/config/localconfig.vdf")).unwrap(),
            CONFIG.replace("-debug", "-late-user")
        );
        assert!(!transaction::backup_path(&fixture.config).exists());
        assert!(!fixture.game.join("zbNative.dll").exists());
        assert!(!fixture.game.join("ZombieBuddy.jar").exists());
        assert!(!fixture.game.join(transaction::JOURNAL).exists());
        assert!(!runtime.running());
    }
}

#[test]
fn committed_install_survives_restart_failure_but_failed_recovery_stays_stopped() {
    for branch in [Branch::Lts, Branch::Beta] {
        let fixture = Fixture::new(branch, true);
        let runtime = ProcessRuntime::new(&fixture);
        runtime.fail_restart.set(true);
        let message = install(&fixture, branch, &runtime).unwrap();
        assert!(message.contains("Start Steam manually"));
        assert!(fixture.game.join("ZombieBuddy.jar").exists());
        assert!(!fixture.game.join(transaction::JOURNAL).exists());

        let fixture = Fixture::new(branch, true);
        let runtime = ProcessRuntime::new(&fixture);
        fs::write(fixture.game.join(transaction::JOURNAL), b"broken journal").unwrap();
        let before = fixture.state();
        let error = install(&fixture, branch, &runtime).unwrap_err();
        assert!(format!("{error:#}").contains("left closed because recovery is incomplete"));
        assert_eq!(fixture.state(), before);
        assert_eq!((runtime.starts.get(), runtime.writes.get()), (0, 0));
        assert!(!runtime.running());
    }
}

//! Execute the patched BAT using real cmd.exe and an isolated child executable.
//! This validates the launch contract, not Project Zomboid or ZombieBuddy code.
#![cfg(windows)]

use installer_core::{
    Branch, TargetPlatform, discover_from_for, install_from_paths_for, uninstall_from_paths_for,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CONFIG: &str = r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "108600" { "LaunchOptions" "-debug -Xmx4G" } "42" { "LaunchOptions" "keep-other-game" } } } } } }"#;
const LAUNCHER: &[u8] = b"@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\nSET _JAVA_OPTIONS=-Xmx4G -Duser.keep=true\r\n\"%ZB_PROBE_EXE%\" --exact launcher_probe --nocapture\r\nexit /b %ERRORLEVEL%\r\n";

#[test]
fn launcher_probe() {
    let Some(root) = std::env::var_os("ZB_LAUNCH_FIXTURE") else {
        return;
    };
    let root = PathBuf::from(root);
    assert_eq!(std::env::current_dir().unwrap(), root);
    let expected = std::env::var("ZB_EXPECT_MOD").unwrap();
    let options = std::env::var("_JAVA_OPTIONS").unwrap();
    assert!(options.contains("-Xmx4G") && options.contains("-Duser.keep=true"));
    if expected == "none" {
        assert!(!options.contains("-agentlib:zbNative"));
        assert!(!root.join("zbNative.dll").exists());
        assert!(!root.join("ZombieBuddy.jar").exists());
    } else {
        assert_eq!(options.matches("-agentlib:zbNative --").count(), 1);
        for name in TargetPlatform::Windows.files() {
            assert_eq!(
                fs::read_to_string(root.join(name)).unwrap(),
                format!("{expected}:{name}")
            );
        }
    }
    fs::write(
        root.join("launch-result.json"),
        serde_json::to_vec(&serde_json::json!({"options": options, "branch": expected})).unwrap(),
    )
    .unwrap();
}

fn launch(game: &Path, branch: &str, user_edit: bool) {
    let result_path = game.join("launch-result.json");
    let _ = fs::remove_file(&result_path);
    let mut command =
        Command::new(std::env::var_os("ComSpec").expect("Windows command interpreter"));
    command
        .args(["/d", "/c", "ProjectZomboid64.bat"])
        .current_dir(game)
        .env("ZB_PROBE_EXE", std::env::current_exe().unwrap())
        .env("ZB_LAUNCH_FIXTURE", game)
        .env("ZB_EXPECT_MOD", branch)
        .env_remove("_JAVA_OPTIONS");
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "launcher failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(result_path).unwrap()).unwrap();
    assert_eq!(report["branch"], branch);
    assert_eq!(
        report["options"]
            .as_str()
            .unwrap()
            .contains("-Duser.after=true"),
        user_edit
    );
}

fn payload(root: &Path, branch: Branch) -> PathBuf {
    let path = root
        .join("steamapps/workshop/content/108600")
        .join(branch.workshop_id());
    let libs = path.join("mods/ZombieBuddy/libs");
    fs::create_dir_all(&libs).unwrap();
    for name in TargetPlatform::Windows.files() {
        fs::write(libs.join(name), format!("{}:{name}", branch.workshop_id())).unwrap();
    }
    path
}

#[test]
fn both_branches_launch_after_install_repeat_switch_and_uninstall() {
    for branch in [Branch::Lts, Branch::Beta] {
        // Exercise real Windows path quoting, including whitespace and a shell metacharacter.
        let temporary = tempfile::Builder::new()
            .prefix("zb acceptance & space ")
            .tempdir()
            .unwrap();
        let root = temporary.path();
        let game = root.join("steamapps/common/Test Game");
        let config = root.join("userdata/42/config/localconfig.vdf");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(
            root.join("steamapps/appmanifest_108600.acf"),
            "AppState { installdir \"Test Game\" }",
        )
        .unwrap();
        fs::write(&config, CONFIG).unwrap();
        fs::write(game.join("ProjectZomboid64.bat"), LAUNCHER).unwrap();
        let json = game.join("ProjectZomboid64.json");
        let original_json = br#"{"vmArgs":["-Xmx4G","-Duser.keep=true"],"mainClass":"test/LaunchProbe","userSetting":"keep"}"#;
        fs::write(&json, original_json).unwrap();
        let workshop = payload(root, branch);
        let configs = [config.clone()];

        install_from_paths_for(&game, &configs, &workshop, TargetPlatform::Windows).unwrap();
        let discovered = discover_from_for(root, TargetPlatform::Windows).unwrap();
        assert_eq!(discovered.game_path, game);
        assert!(
            discovered
                .branches
                .iter()
                .any(|item| item.branch.workshop_id() == branch.workshop_id() && item.ready)
        );
        launch(&game, branch.workshop_id(), false);
        let installed: serde_json::Value =
            serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
        assert_eq!(installed["vmArgs"][0], "-agentlib:zbNative --");
        assert_eq!(installed["userSetting"], "keep");

        // User changes after installation must survive a repeat, branch switch and removal.
        let batch = fs::read_to_string(game.join("ProjectZomboid64.bat"))
            .unwrap()
            .replace("-Duser.keep=true", "-Duser.keep=true -Duser.after=true");
        fs::write(game.join("ProjectZomboid64.bat"), batch).unwrap();
        let steam = fs::read_to_string(&config)
            .unwrap()
            .replace("-debug", "-debug -user-after");
        fs::write(&config, steam).unwrap();
        let mut json_edit: serde_json::Value = installed;
        json_edit["vmArgs"]
            .as_array_mut()
            .unwrap()
            .push("-Duser.after=true".into());
        fs::write(&json, serde_json::to_vec(&json_edit).unwrap()).unwrap();
        install_from_paths_for(&game, &configs, &workshop, TargetPlatform::Windows).unwrap();
        launch(&game, branch.workshop_id(), true);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&json).unwrap()).unwrap(),
            json_edit
        );
        let other = match branch {
            Branch::Lts => Branch::Beta,
            Branch::Beta => Branch::Lts,
        };
        install_from_paths_for(
            &game,
            &configs,
            &payload(root, other),
            TargetPlatform::Windows,
        )
        .unwrap();
        launch(&game, other.workshop_id(), true);

        uninstall_from_paths_for(&game, &configs, TargetPlatform::Windows).unwrap();
        launch(&game, "none", true);
        let uninstalled: serde_json::Value =
            serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
        assert_eq!(
            uninstalled["vmArgs"],
            serde_json::json!(["-Xmx4G", "-Duser.keep=true", "-Duser.after=true"])
        );
        let steam = fs::read_to_string(&config).unwrap();
        assert!(steam.contains("-user-after") && steam.contains("keep-other-game"));
        assert!(!steam.contains("-agentlib:zbNative"));
        assert_eq!(
            fs::read(config.with_extension("vdf.backup")).unwrap(),
            CONFIG.as_bytes()
        );
        assert_eq!(
            fs::read(json.with_extension("json.backup")).unwrap(),
            original_json
        );
        assert_eq!(
            fs::read(game.join("ProjectZomboid64.bat.backup")).unwrap(),
            LAUNCHER
        );
        assert!(!game.join(".zombiebuddy-transaction.json").exists());
    }
}

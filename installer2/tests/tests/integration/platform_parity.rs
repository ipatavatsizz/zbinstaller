use installer_core::{
    Branch, DiagnosticAction, TargetPlatform, discover_from_for, inspect_from,
    install_from_paths_for, run_diagnostic_action_for, steam_candidates, uninstall_from_paths_for,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

const CONFIG: &[u8] = br#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "108600" { "LaunchOptions" "-debug" } "42" { "LaunchOptions" "keep" } } } } } }"#;

#[test]
fn diagnostics_returns_steam_details_when_the_game_is_missing() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("steamapps")).unwrap();
    let found = inspect_from(root.path(), TargetPlatform::Windows).unwrap();
    assert_eq!(found.steam_path, root.path());
    assert!(found.game_path.as_os_str().is_empty());
    assert!(!found.warnings.is_empty());
    assert!(found.files.iter().all(|file| !file.exists));
    assert!(discover_from_for(root.path(), TargetPlatform::Windows).is_err());
}

fn fixture(
    root: &Path,
    target: TargetPlatform,
    branch: Branch,
    launcher: &str,
) -> (PathBuf, PathBuf, PathBuf) {
    let game = root.join("steamapps/common/Custom Game");
    let workshop = root
        .join("steamapps/workshop/content/108600")
        .join(branch.workshop_id());
    let config = root.join("userdata/42/config/localconfig.vdf");
    fs::create_dir_all(game.join("Contents/Java")).unwrap();
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::create_dir_all(workshop.join("mods/ZombieBuddy/libs")).unwrap();
    fs::create_dir_all(workshop.join("mods/ZombieBuddy/java")).unwrap();
    fs::write(
        root.join("steamapps/appmanifest_108600.acf"),
        "AppState { installdir \"Custom Game\" }",
    )
    .unwrap();
    fs::write(&config, CONFIG).unwrap();
    fs::write(
        game.join("Contents/Java/ProjectZomboid64.json"),
        br#"{"vmArgs":["-Xmx4G"],"nested":{"vmArgs":["keep"]}}"#,
    )
    .unwrap();
    fs::write(
        game.join(launcher),
        if target == TargetPlatform::Windows {
            b"@echo off\r\nSET _JAVA_OPTIONS=-Xmx4G\r\n".as_slice()
        } else {
            b"#!/bin/sh\nexport KEEP=yes\necho start\n".as_slice()
        },
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(game.join(launcher), fs::Permissions::from_mode(0o755)).unwrap();
    }
    for file in target.files() {
        fs::write(workshop.join("mods/ZombieBuddy/libs").join(file), file).unwrap();
    }
    fs::write(workshop.join("mods/ZombieBuddy/java/VERSION"), " 1.2.3\n").unwrap();
    (game, config, workshop)
}

#[test]
fn both_branches_on_all_platforms_install_repeat_and_uninstall_with_nested_json() {
    for (target, launcher) in [
        (TargetPlatform::Windows, "ProjectZomboid64.bat"),
        (TargetPlatform::Linux, "ProjectZomboid64.sh"),
        (TargetPlatform::Macos, "ProjectZomboid64.command"),
    ] {
        for branch in [Branch::Lts, Branch::Beta] {
            let root = tempfile::tempdir().unwrap();
            let (game, config, workshop) = fixture(root.path(), target, branch, launcher);
            let json = game.join("Contents/Java/ProjectZomboid64.json");
            let original_json = fs::read(&json).unwrap();
            let original_launcher = fs::read(game.join(launcher)).unwrap();
            let found = discover_from_for(root.path(), target).unwrap();
            assert_eq!(found.game_path, game);
            assert!(
                found
                    .branches
                    .iter()
                    .find(|b| b.branch.workshop_id() == branch.workshop_id())
                    .unwrap()
                    .ready
            );
            assert!(
                found
                    .files
                    .iter()
                    .any(|f| f.name == "ProjectZomboid64.json" && f.exists)
            );

            install_from_paths_for(&game, std::slice::from_ref(&config), &workshop, target)
                .unwrap();
            let first_json = fs::read(&json).unwrap();
            let first_launcher = fs::read(game.join(launcher)).unwrap();
            let first_config = fs::read(&config).unwrap();
            assert!(String::from_utf8_lossy(&first_json).contains(target.flags()));
            assert!(String::from_utf8_lossy(&first_launcher).contains(target.flags()));
            assert!(String::from_utf8_lossy(&first_config).contains(target.flags()));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&first_json).unwrap()["nested"]["vmArgs"]
                    [0],
                "keep"
            );
            if target != TargetPlatform::Windows {
                assert!(first_launcher.starts_with(b"#!/bin/sh\n"));
            }
            for file in target.files() {
                assert_eq!(fs::read(game.join(file)).unwrap(), file.as_bytes());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(game.join(launcher))
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o755
                );
            }

            install_from_paths_for(&game, std::slice::from_ref(&config), &workshop, target)
                .unwrap();
            assert_eq!(fs::read(&json).unwrap(), first_json);
            assert_eq!(fs::read(game.join(launcher)).unwrap(), first_launcher);
            assert_eq!(fs::read(&config).unwrap(), first_config);
            assert_eq!(
                fs::read(json.with_extension("json.backup")).unwrap(),
                original_json
            );
            assert_eq!(
                fs::read(config.with_extension("vdf.backup")).unwrap(),
                CONFIG
            );
            assert!(!game.join(".zombiebuddy-transaction.json").exists());

            // User edits after installation must survive uninstall; original backups remain available.
            let edited = String::from_utf8(first_config)
                .unwrap()
                .replace("-debug", "-debug -user-option");
            fs::write(&config, edited).unwrap();
            uninstall_from_paths_for(&game, std::slice::from_ref(&config), target).unwrap();
            for file in target.files() {
                assert!(!game.join(file).exists());
            }
            assert!(
                !String::from_utf8_lossy(&fs::read(&json).unwrap())
                    .contains(target.flags().trim_end_matches(" --"))
            );
            assert!(
                String::from_utf8_lossy(&fs::read(&config).unwrap())
                    .contains("-debug -user-option")
            );
            if target != TargetPlatform::Windows {
                assert_eq!(fs::read(game.join(launcher)).unwrap(), original_launcher);
            }
            assert_eq!(
                fs::read(config.with_extension("vdf.backup")).unwrap(),
                CONFIG
            );
        }
    }
}

#[test]
fn server_launcher_fallback_and_existing_go_backups_are_preserved() {
    let root = tempfile::tempdir().unwrap();
    let (game, config, workshop) = fixture(
        root.path(),
        TargetPlatform::Linux,
        Branch::Lts,
        "start-server.sh",
    );
    let backup = game.join("Contents/Java/ProjectZomboid64.json.backup");
    fs::write(&backup, b"original Go backup").unwrap();
    install_from_paths_for(&game, &[config], &workshop, TargetPlatform::Linux).unwrap();
    assert!(
        String::from_utf8_lossy(&fs::read(game.join("start-server.sh")).unwrap())
            .contains(TargetPlatform::Linux.flags())
    );
    assert_eq!(fs::read(backup).unwrap(), b"original Go backup");
}

#[test]
fn invalid_second_user_prevents_all_diagnostic_writes_and_backups() {
    let root = tempfile::tempdir().unwrap();
    let (game, first, _) = fixture(
        root.path(),
        TargetPlatform::Windows,
        Branch::Lts,
        "ProjectZomboid64.bat",
    );
    let second = root.path().join("second.vdf");
    fs::write(&second, b"\"broken\" {").unwrap();
    assert!(
        run_diagnostic_action_for(
            DiagnosticAction::Steam,
            &game,
            &[first.clone(), second],
            TargetPlatform::Windows
        )
        .is_err()
    );
    assert_eq!(fs::read(&first).unwrap(), CONFIG);
    assert!(!first.with_extension("vdf.backup").exists());
    assert!(!game.join(".zombiebuddy-transaction.json").exists());
}

#[test]
fn candidates_include_standard_flatpak_snap_and_macos_locations() {
    let home = Path::new("fixture-home");
    let linux = steam_candidates(home, TargetPlatform::Linux);
    assert!(linux.contains(&home.join(".steam/steam")));
    assert!(linux.contains(&home.join(".local/share/Steam")));
    assert!(linux.contains(&home.join(".var/app/com.valvesoftware.Steam/data/Steam")));
    assert!(linux.contains(&home.join("snap/steam/common/.steam/steam")));
    assert_eq!(
        steam_candidates(home, TargetPlatform::Macos),
        vec![home.join("Library/Application Support/Steam")]
    );
}

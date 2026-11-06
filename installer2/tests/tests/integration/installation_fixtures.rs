use installer_core::{
    DiagnosticAction, TargetPlatform, discover_from_for, install_from_paths_for,
    run_diagnostic_action_for, uninstall_from_paths_for,
};
use installer2_tests::fixture;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn write_fixture(path: &Path, name: &str) {
    fs::write(path, fixture(name)).unwrap();
}

fn prepare_fixture_install(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let game = root.join("game");
    let steam = root.join("steam");
    let workshop = root.join("workshop");
    let config = steam.join("userdata/123/config/localconfig.vdf");
    fs::create_dir_all(&game).unwrap();
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::create_dir_all(workshop.join("mods/ZombieBuddy/libs")).unwrap();
    write_fixture(&config, "localconfig.vdf");
    write_fixture(&game.join("ProjectZomboid64.json"), "ProjectZomboid64.json");
    write_fixture(&game.join("ProjectZomboid64.bat"), "ProjectZomboid64.bat");
    fs::write(
        workshop.join("mods/ZombieBuddy/libs/zbNative.dll"),
        b"dll-payload",
    )
    .unwrap();
    fs::write(
        workshop.join("mods/ZombieBuddy/libs/ZombieBuddy.jar"),
        b"jar-payload",
    )
    .unwrap();
    (game, config, workshop)
}

#[test]
fn installation_applies_all_fixture_patches_copies_payloads_and_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let (game, config, workshop) = prepare_fixture_install(root.path());
    let config_paths = vec![config.clone()];

    install_from_paths_for(&game, &config_paths, &workshop, TargetPlatform::Windows).unwrap();
    let first_pass: Vec<_> = [
        config.clone(),
        game.join("ProjectZomboid64.json"),
        game.join("ProjectZomboid64.bat"),
        game.join("zbNative.dll"),
        game.join("ZombieBuddy.jar"),
    ]
    .into_iter()
    .map(fs::read)
    .collect::<Result<_, _>>()
    .unwrap();

    assert!(String::from_utf8_lossy(&first_pass[0]).contains("-agentlib:zbNative -- -Xmx16G --"));
    assert!(String::from_utf8_lossy(&first_pass[1]).contains("\"-agentlib:zbNative\""));
    assert!(
        String::from_utf8_lossy(&first_pass[2])
            .lines()
            .any(|line| line == "SET _JAVA_OPTIONS=-agentlib:zbNative")
    );
    assert_eq!(first_pass[3], b"dll-payload");
    assert_eq!(first_pass[4], b"jar-payload");

    install_from_paths_for(&game, &config_paths, &workshop, TargetPlatform::Windows).unwrap();
    let second_pass: Vec<_> = [
        config,
        game.join("ProjectZomboid64.json"),
        game.join("ProjectZomboid64.bat"),
        game.join("zbNative.dll"),
        game.join("ZombieBuddy.jar"),
    ]
    .into_iter()
    .map(fs::read)
    .collect::<Result<_, _>>()
    .unwrap();
    assert_eq!(first_pass, second_pass);
}

#[test]
fn invalid_launcher_fails_before_any_installation_file_is_modified() {
    let root = tempfile::tempdir().unwrap();
    let (game, config, workshop) = prepare_fixture_install(root.path());
    let original_config = fs::read(&config).unwrap();
    fs::write(game.join("ProjectZomboid64.json"), b"not-json").unwrap();

    assert!(
        install_from_paths_for(
            &game,
            std::slice::from_ref(&config),
            &workshop,
            TargetPlatform::Windows
        )
        .is_err()
    );
    assert_eq!(fs::read(config).unwrap(), original_config);
    assert!(!game.join("zbNative.dll").exists());
    assert!(!game.join("ZombieBuddy.jar").exists());
}

#[test]
fn uninstall_removes_payloads_and_only_the_managed_launch_flags() {
    let root = tempfile::tempdir().unwrap();
    let (game, config, workshop) = prepare_fixture_install(root.path());
    let original_markers = String::from_utf8_lossy(&fs::read(&config).unwrap())
        .matches("-agentlib:zbNative --")
        .count();
    install_from_paths_for(
        &game,
        std::slice::from_ref(&config),
        &workshop,
        TargetPlatform::Windows,
    )
    .unwrap();

    uninstall_from_paths_for(
        &game,
        std::slice::from_ref(&config),
        TargetPlatform::Windows,
    )
    .unwrap();

    assert!(!game.join("zbNative.dll").exists());
    assert!(!game.join("ZombieBuddy.jar").exists());
    assert!(String::from_utf8_lossy(&fs::read(&config).unwrap()).contains("-Xmx16G --"));
    assert_eq!(
        String::from_utf8_lossy(&fs::read(&config).unwrap())
            .matches("-agentlib:zbNative --")
            .count(),
        original_markers,
        "uninstall must restore the original flags and preserve unrelated Steam options"
    );
    assert!(
        !String::from_utf8_lossy(&fs::read(game.join("ProjectZomboid64.json")).unwrap())
            .contains("-agentlib:zbNative")
    );
    assert!(
        !String::from_utf8_lossy(&fs::read(game.join("ProjectZomboid64.bat")).unwrap())
            .contains("-agentlib:zbNative")
    );
}

#[test]
fn diagnostics_patch_actions_create_first_backup_and_preserve_fixture_content() {
    let root = tempfile::tempdir().unwrap();
    let (game, config, _) = prepare_fixture_install(root.path());
    let config_paths = vec![config.clone()];
    let json = game.join("ProjectZomboid64.json");
    let batch = game.join("ProjectZomboid64.bat");
    let original_config = fs::read(&config).unwrap();
    let original_json = fs::read(&json).unwrap();
    let original_batch = fs::read(&batch).unwrap();

    for action in [
        DiagnosticAction::Steam,
        DiagnosticAction::Json,
        DiagnosticAction::Launcher,
    ] {
        run_diagnostic_action_for(action, &game, &config_paths, TargetPlatform::Windows).unwrap();
    }

    assert_eq!(
        fs::read(config.with_extension("vdf.backup")).unwrap(),
        original_config
    );
    assert_eq!(
        fs::read(json.with_extension("json.backup")).unwrap(),
        original_json
    );
    assert_eq!(
        fs::read(batch.with_extension("bat.backup")).unwrap(),
        original_batch
    );
    let patched_json = fs::read(&json).unwrap();
    let patched_batch = fs::read(&batch).unwrap();
    run_diagnostic_action_for(
        DiagnosticAction::Json,
        &game,
        &config_paths,
        TargetPlatform::Windows,
    )
    .unwrap();
    run_diagnostic_action_for(
        DiagnosticAction::Launcher,
        &game,
        &config_paths,
        TargetPlatform::Windows,
    )
    .unwrap();
    assert_eq!(fs::read(json).unwrap(), patched_json);
    assert_eq!(fs::read(batch).unwrap(), patched_batch);
    assert_eq!(
        fs::read(config.with_extension("vdf.backup")).unwrap(),
        original_config
    );
}

#[test]
fn discovery_uses_library_manifest_workshop_and_fixture_files() {
    let root = tempfile::tempdir().unwrap();
    let steam = root.path().join("Steam");
    let library = root.path().join("Game Library");
    let game = library.join("steamapps/common/ProjectZomboid");
    fs::create_dir_all(steam.join("steamapps")).unwrap();
    fs::create_dir_all(steam.join("userdata/42/config")).unwrap();
    fs::create_dir_all(game.join("..")).unwrap();
    fs::write(
        steam.join("steamapps/libraryfolders.vdf"),
        format!(
            "\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} }}",
            library.display()
        ),
    )
    .unwrap();
    fs::write(
        library.join("steamapps/appmanifest_108600.acf"),
        "\"AppState\" { \"installdir\" \"ProjectZomboid\" }",
    )
    .unwrap();
    write_fixture(
        &steam.join("userdata/42/config/localconfig.vdf"),
        "localconfig.vdf",
    );
    write_fixture(&game.join("ProjectZomboid64.json"), "ProjectZomboid64.json");
    write_fixture(&game.join("ProjectZomboid64.bat"), "ProjectZomboid64.bat");
    let branch = library.join("steamapps/workshop/content/108600/3619862853");
    let libs = branch.join("mods/ZombieBuddy/libs");
    fs::create_dir_all(&libs).unwrap();
    fs::write(libs.join("zbNative.dll"), b"dll").unwrap();
    fs::write(libs.join("ZombieBuddy.jar"), b"jar").unwrap();
    fs::create_dir_all(branch.join("mods/ZombieBuddy/java")).unwrap();
    fs::write(branch.join("mods/ZombieBuddy/java/VERSION"), "2.3.4\n").unwrap();

    let found = discover_from_for(&steam, TargetPlatform::Windows).unwrap();
    assert_eq!(found.game_path, game);
    assert_eq!(found.libraries, vec![steam, library]);
    assert_eq!(found.config_paths.len(), 1);
    assert_eq!(found.branches[0].version.as_deref(), Some("2.3.4"));
    assert!(found.branches[0].ready);
    assert!(!found.branches[1].ready);
    assert!(
        found
            .files
            .iter()
            .any(|file| file.name == "ProjectZomboid64.json" && file.exists)
    );
}

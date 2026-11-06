use installer_core::{
    Branch, launcher_bat, launcher_json, steam_config, workshop_manifest_contains_item,
};
use installer2_tests::fixture;

#[test]
fn branch_ids_are_stable_and_serialized_names_are_ui_friendly() {
    assert_eq!(Branch::Lts.workshop_id(), "3619862853");
    assert_eq!(Branch::Beta.workshop_id(), "3812624292");
    assert_eq!(serde_json::to_string(&Branch::Lts).unwrap(), "\"lts\"");
    assert_eq!(serde_json::to_string(&Branch::Beta).unwrap(), "\"beta\"");
}

#[test]
fn json_fixture_gets_agent_only_once_and_preserves_settings() {
    let input = fixture("ProjectZomboid64.json");
    let output = launcher_json(input).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let args = parsed["vmArgs"].as_array().unwrap();

    assert_eq!(args[0], "-agentlib:zbNative");
    assert_eq!(
        args.iter()
            .filter(|value| value.as_str() == Some("-agentlib:zbNative"))
            .count(),
        1
    );
    assert_eq!(parsed["mainClass"], "zombie/gameStates/MainScreenState");
    assert_eq!(
        parsed["classpath"],
        serde_json::json!([".", "projectzomboid.jar"])
    );
    assert_eq!(parsed["windows"]["10.0.17134"]["vmArgs"][1], "-XX:+UseZGC");
    assert_eq!(launcher_json(&output).unwrap(), output);
}

#[test]
fn batch_fixture_preserves_launcher_commands_and_normalizes_agent_once() {
    // Exercise both encodings explicitly, independent of Git checkout settings.
    let fixture = std::str::from_utf8(fixture("ProjectZomboid64.bat"))
        .unwrap()
        .replace("\r\n", "\n");
    for newline in ["\n", "\r\n"] {
        let input = fixture.replace('\n', newline);
        let output = launcher_bat(input.as_bytes()).unwrap();
        let text = String::from_utf8(output.clone()).unwrap();

        assert!(text.contains(&format!("SET _JAVA_OPTIONS=-agentlib:zbNative{newline}")));
        assert_eq!(text.matches("-agentlib:zbNative").count(), 1);
        assert!(text.contains("SET PZ_CLASSPATH=./;projectzomboid.jar"));
        assert!(text.contains("IF %ERRORLEVEL% NEQ 0"));
        if newline == "\r\n" {
            assert!(!text.replace("\r\n", "").contains('\n'));
        } else {
            assert!(!text.contains('\r'));
        }
        assert_eq!(launcher_bat(&output).unwrap(), output);
    }
}

#[test]
fn steam_fixture_only_adds_flags_to_project_zomboid_launch_options() {
    let input = fixture("localconfig.vdf");
    let output = steam_config(input)
        .unwrap()
        .expect("fixture has PZ app settings");
    let text = String::from_utf8(output.clone()).unwrap();

    assert!(text.contains("-agentlib:zbNative -- -Xmx16G --"));
    assert!(text.contains("\"328379447\""));
    assert_eq!(steam_config(&output).unwrap().unwrap(), output);
}

#[test]
fn patchers_reject_invalid_json_and_malformed_vdf_without_output() {
    assert!(launcher_json(b"{invalid").is_err());
    assert!(steam_config(b"\"UserLocalConfigStore\" {").is_err());
}

#[test]
fn appworkshop_manifest_confirms_only_items_fully_recorded_as_installed() {
    let manifest = std::str::from_utf8(fixture("appworkshop_108600.acf")).unwrap();
    assert!(workshop_manifest_contains_item(manifest, Branch::Lts).unwrap());
    assert!(workshop_manifest_contains_item(manifest, Branch::Beta).unwrap());
    assert!(
        !workshop_manifest_contains_item(
            &manifest.replace("3619862853", "9999999999"),
            Branch::Lts
        )
        .unwrap()
    );
    assert!(workshop_manifest_contains_item("malformed", Branch::Lts).is_err());
}

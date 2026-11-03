//! Pure transformations: no filesystem access or Tauri dependencies.
use crate::TargetPlatform;
use crate::vdf::{self, Value};
use anyhow::{Context, Result, ensure};

pub const FLAGS: &str = "-agentlib:zbNative --";

pub fn launcher_json(input: &[u8]) -> Result<Vec<u8>> {
    launcher_json_for(input, TargetPlatform::Windows)
}

pub fn launcher_json_for(input: &[u8], target: TargetPlatform) -> Result<Vec<u8>> {
    let flags = target.flags();
    let mut root: serde_json::Value = serde_json::from_slice(input)?;
    let args = root
        .get_mut("vmArgs")
        .and_then(|v| v.as_array_mut())
        .context("Root vmArgs array was not found")?;
    ensure!(
        args.iter().all(serde_json::Value::is_string),
        "vmArgs must contain strings"
    );
    if args.iter().any(|v| v.as_str() == Some(flags)) {
        return Ok(input.to_vec());
    }
    if let Some(agent) = args.iter_mut().find(|value| {
        value
            .as_str()
            .is_some_and(|text| text == flags.trim_end_matches(" --"))
    }) {
        *agent = flags.into();
    } else {
        args.insert(0, flags.into());
    }
    Ok(serde_json::to_vec_pretty(&root)?)
}

pub fn uninstall_launcher_json(input: &[u8], target: TargetPlatform) -> Result<Vec<u8>> {
    let mut root: serde_json::Value = serde_json::from_slice(input)?;
    ensure!(
        root.get("vmArgs").is_some_and(|args| args.is_array()),
        "Root vmArgs array was not found"
    );
    fn clean(value: &mut serde_json::Value, flags: &str) -> bool {
        let mut changed = false;
        match value {
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    if key == "vmArgs"
                        && let Some(args) = value.as_array_mut()
                    {
                        let previous = args.len();
                        args.retain(|arg| {
                            !arg.as_str().is_some_and(|arg| {
                                arg == flags || arg == flags.trim_end_matches(" --")
                            })
                        });
                        changed |= args.len() != previous;
                    } else {
                        changed |= clean(value, flags);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    changed |= clean(value, flags);
                }
            }
            _ => {}
        }
        changed
    }
    if !clean(&mut root, target.flags()) {
        return Ok(input.to_vec());
    }
    Ok(serde_json::to_vec_pretty(&root)?)
}

pub fn launcher_bat(input: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(input).context("Launcher must be UTF-8")?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let normalized = text.replace("\r\n", "\n");
    let mut found = false;
    let mut lines: Vec<String> = normalized
        .split('\n')
        .map(|line| {
            if line
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("SET _JAVA_OPTIONS=")
            {
                found = true;
                let (key, value) = line.split_once('=').expect("matched assignment");
                if !value.contains(FLAGS) {
                    let value = value.trim();
                    if value == "-agentlib:zbNative" {
                        return format!("{key}={FLAGS}");
                    }
                    if let Some(suffix) = value.strip_prefix("-agentlib:zbNative ") {
                        return format!(
                            "{key}={FLAGS}{}{}",
                            if suffix.is_empty() { "" } else { " " },
                            suffix.trim()
                        );
                    }
                    return format!("{key}={FLAGS} {value}");
                }
            }
            line.to_owned()
        })
        .collect();
    if !found {
        lines.insert(0, format!("SET _JAVA_OPTIONS={FLAGS}"));
    }
    Ok(lines.join(newline).into_bytes())
}

pub fn uninstall_launcher_bat(input: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(input).context("Launcher must be UTF-8")?;
    let output: String = text
        .split_inclusive('\n')
        .map(|line| {
            if line
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("SET _JAVA_OPTIONS=")
            {
                let (key, value) = line.split_once('=').expect("matched assignment");
                let ending = if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                format!(
                    "{key}={}{ending}",
                    remove_flags(value.trim_end_matches(['\r', '\n']), FLAGS)
                )
            } else {
                line.to_owned()
            }
        })
        .collect();
    Ok(output.into_bytes())
}

pub fn launcher_shell(input: &[u8], target: TargetPlatform) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(input).context("Launcher must be UTF-8")?;
    if text.contains(target.flags()) {
        return Ok(input.to_vec());
    }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let line = format!(
        "export _JAVA_OPTIONS=\"{} ${{_JAVA_OPTIONS:-}}\"{newline}",
        target.flags()
    );
    // A shebang must remain the first line for direct execution.
    let position = if text.starts_with("#!") {
        text.find('\n').map_or(text.len(), |p| p + 1)
    } else {
        0
    };
    let separator = if position > 0 && !text[..position].ends_with('\n') {
        newline
    } else {
        ""
    };
    Ok(format!(
        "{}{separator}{line}{}",
        &text[..position],
        &text[position..]
    )
    .into_bytes())
}

pub fn uninstall_launcher_shell(input: &[u8], target: TargetPlatform) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(input).context("Launcher must be UTF-8")?;
    let managed = format!(
        "export _JAVA_OPTIONS=\"{} ${{_JAVA_OPTIONS:-}}\"",
        target.flags()
    );
    Ok(text
        .split_inclusive('\n')
        .filter(|line| line.trim_end_matches(['\r', '\n']) != managed)
        .collect::<String>()
        .into_bytes())
}

fn remove_flags(value: &str, flags: &str) -> String {
    let mut output = value.to_owned();
    for flag in [flags, flags.trim_end_matches(" --")] {
        // Only remove whole arguments; leave substrings in unrelated options intact.
        while let Some(index) = output.match_indices(flag).find_map(|(i, _)| {
            let end = i + flag.len();
            let before = i == 0 || output.as_bytes()[i - 1].is_ascii_whitespace();
            let after = end == output.len() || output.as_bytes()[end].is_ascii_whitespace();
            (before && after).then_some(i)
        }) {
            let mut end = index + flag.len();
            if output.as_bytes().get(end) == Some(&b' ') {
                end += 1;
            }
            output.replace_range(index..end, "");
        }
    }
    output.trim().to_owned()
}

pub fn steam_config(input: &[u8]) -> Result<Option<Vec<u8>>> {
    steam_config_for(input, TargetPlatform::Windows)
}

pub fn steam_config_for(input: &[u8], target: TargetPlatform) -> Result<Option<Vec<u8>>> {
    let flags = target.flags();
    let text = std::str::from_utf8(input)?;
    let entries = vdf::parse(text)?;
    let Some(app) = vdf::unique_object_at(
        &entries,
        &[
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
            "108600",
        ],
    )?
    else {
        return Ok(None);
    };
    let options: Vec<_> = app
        .iter()
        .filter(|e| e.key.eq_ignore_ascii_case("LaunchOptions"))
        .collect();
    ensure!(
        options.len() == 1,
        "Project Zomboid must have exactly one LaunchOptions entry; set launch options in Steam first"
    );
    let Value::Text(value, span) = &options[0].value else {
        anyhow::bail!("LaunchOptions must be a string")
    };
    if value.contains(flags) {
        return Ok(Some(input.to_vec()));
    }
    let desired = format!("{flags} {}", value.trim()).trim_end().to_owned();
    let mut output = text.to_owned();
    output.replace_range(span.clone(), &vdf::quote(&desired));
    Ok(Some(output.into_bytes()))
}

pub fn uninstall_steam_config(input: &[u8], target: TargetPlatform) -> Result<Option<Vec<u8>>> {
    let text = std::str::from_utf8(input)?;
    let entries = vdf::parse(text)?;
    let Some(app) = vdf::unique_object_at(
        &entries,
        &[
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
            "108600",
        ],
    )?
    else {
        return Ok(None);
    };
    let options: Vec<_> = app
        .iter()
        .filter(|e| e.key.eq_ignore_ascii_case("LaunchOptions"))
        .collect();
    ensure!(
        options.len() <= 1,
        "Project Zomboid has multiple LaunchOptions entries"
    );
    if options.is_empty() {
        return Ok(Some(input.to_vec()));
    }
    let Value::Text(value, span) = &options[0].value else {
        anyhow::bail!("LaunchOptions must be a string")
    };
    let cleaned = remove_flags(value, target.flags());
    if cleaned == value.as_str() {
        return Ok(Some(input.to_vec()));
    }
    let mut output = text.to_owned();
    output.replace_range(span.clone(), &vdf::quote(&cleaned));
    Ok(Some(output.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_preserves_nested_settings_and_is_idempotent() {
        let first =
            launcher_json(br#"{"vmArgs":["-Xmx4g"],"nested":{"vmArgs":["keep"]}}"#).unwrap();
        assert_eq!(first, launcher_json(&first).unwrap());
        let value: serde_json::Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(value["nested"]["vmArgs"][0], "keep");
        assert!(launcher_json(br#"{"vmArgs":"bad"}"#).is_err());
    }

    #[test]
    fn batch_preserves_crlf_and_existing_options() {
        let first = launcher_bat(b"@echo off\r\nSET _JAVA_OPTIONS=-Xmx4g\r\n").unwrap();
        assert_eq!(first, launcher_bat(&first).unwrap());
        assert!(String::from_utf8(first).unwrap().contains("-- -Xmx4g\r\n"));
    }

    #[test]
    fn vdf_edits_only_target_value_and_preserves_escaped_quotes() {
        let input = r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "42" { "LaunchOptions" "keep" } "108600" { "LaunchOptions" "-Dname=\"hello\"" } } } } } }"#;
        let first = steam_config(input.as_bytes()).unwrap().unwrap();
        assert_eq!(first, steam_config(&first).unwrap().unwrap());
        let output = String::from_utf8(first).unwrap();
        assert!(output.contains(r#""42" { "LaunchOptions" "keep" }"#));
        assert!(output.contains(r#"-Dname=\"hello\""#));
        assert!(steam_config(b"\"broken\" {").is_err());
    }

    #[test]
    fn vdf_accepts_bom_comments_and_unquoted_tokens_without_reformatting_neighbors() {
        let input = "\u{feff}/* keep */ UserLocalConfigStore { Software { Valve { Steam { apps { 108600 { LaunchOptions -debug } 42 { LaunchOptions untouched } } } } } } // keep\n";
        let first = steam_config(input.as_bytes()).unwrap().unwrap();
        assert!(String::from_utf8_lossy(&first).contains("LaunchOptions untouched"));
        assert!(String::from_utf8_lossy(&first).starts_with("\u{feff}/* keep */"));
        assert_eq!(first, steam_config(&first).unwrap().unwrap());
        assert!(steam_config(b"/* unclosed").is_err());
    }

    #[test]
    fn ambiguous_apps_and_launch_options_are_rejected() {
        let duplicate = br#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "108600" { "LaunchOptions" "one" } "108600" { "LaunchOptions" "two" } } } } } }"#;
        assert!(steam_config(duplicate).is_err());
        let duplicate_option = br#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "108600" { "LaunchOptions" "one" "launchoptions" "two" } } } } } }"#;
        assert!(steam_config(duplicate_option).is_err());
    }

    #[test]
    fn unix_launcher_preserves_shebang_existing_options_and_roundtrips_legacy_injection() {
        let target = TargetPlatform::Linux;
        let input = b"#!/bin/sh\nexport _JAVA_OPTIONS=\"-Xmx4G\"\necho start\n";
        let first = launcher_shell(input, target).unwrap();
        assert!(first.starts_with(b"#!/bin/sh\n"));
        assert_eq!(launcher_shell(&first, target).unwrap(), first);
        assert_eq!(uninstall_launcher_shell(&first, target).unwrap(), input);
        let legacy = format!(
            "export _JAVA_OPTIONS=\"{} ${{_JAVA_OPTIONS:-}}\"\n{}",
            target.flags(),
            std::str::from_utf8(input).unwrap()
        );
        assert_eq!(
            uninstall_launcher_shell(legacy.as_bytes(), target).unwrap(),
            input
        );
    }

    #[test]
    fn uninstall_preserves_unrelated_launch_options_and_batch_comments() {
        let input = b"REM -agentlib:zbNative -- keep this comment\r\nSET _JAVA_OPTIONS=-agentlib:zbNative -- -Xmx4G\r\n";
        let cleaned = uninstall_launcher_bat(input).unwrap();
        assert_eq!(
            cleaned,
            b"REM -agentlib:zbNative -- keep this comment\r\nSET _JAVA_OPTIONS=-Xmx4G\r\n"
        );
        assert_eq!(
            remove_flags("-Dvalue=prefix-agentlib:zbNative -- -debug", FLAGS),
            "-Dvalue=prefix-agentlib:zbNative -- -debug"
        );
    }
}

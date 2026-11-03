//! Pure transformations: no filesystem access or Tauri dependencies.
use crate::vdf::{self, Value};
use anyhow::{Context, Result, ensure};

pub const FLAGS: &str = "-agentlib:zbNative --";

pub fn launcher_json(input: &[u8]) -> Result<Vec<u8>> {
    let mut root: serde_json::Value = serde_json::from_slice(input)?;
    let args = root
        .get_mut("vmArgs")
        .and_then(|v| v.as_array_mut())
        .context("Root vmArgs array was not found")?;
    ensure!(
        args.iter().all(serde_json::Value::is_string),
        "vmArgs must contain strings"
    );
    if args.iter().any(|v| v.as_str() == Some(FLAGS)) {
        return Ok(input.to_vec());
    }
    if let Some(agent) = args.iter_mut().find(|value| {
        value
            .as_str()
            .is_some_and(|text| text == "-agentlib:zbNative")
    }) {
        *agent = FLAGS.into();
    } else {
        args.insert(0, FLAGS.into());
    }
    Ok(serde_json::to_vec_pretty(&root)?)
}

pub fn uninstall_launcher_json(input: &[u8]) -> Result<Vec<u8>> {
    let mut root: serde_json::Value = serde_json::from_slice(input)?;
    let args = root
        .get_mut("vmArgs")
        .and_then(|v| v.as_array_mut())
        .context("Root vmArgs array was not found")?;
    let mut changed = false;
    for arg in args {
        if arg.as_str() == Some(FLAGS) {
            *arg = "-agentlib:zbNative".into();
            changed = true;
        }
    }
    if !changed {
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
    if !text.contains(FLAGS) {
        return Ok(input.to_vec());
    }
    Ok(text.replace(FLAGS, "-agentlib:zbNative").into_bytes())
}

pub fn steam_config(input: &[u8]) -> Result<Option<Vec<u8>>> {
    let text = std::str::from_utf8(input)?;
    let entries = vdf::parse(text)?;
    let Some(app) = vdf::object_at(
        &entries,
        &[
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
            "108600",
        ],
    ) else {
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
    if value.contains(FLAGS) {
        return Ok(Some(input.to_vec()));
    }
    let desired = format!("{FLAGS} {}", value.trim()).trim_end().to_owned();
    let mut output = text.to_owned();
    output.replace_range(span.clone(), &vdf::quote(&desired));
    Ok(Some(output.into_bytes()))
}

pub fn uninstall_steam_config(input: &[u8]) -> Result<Option<Vec<u8>>> {
    let text = std::str::from_utf8(input)?;
    let entries = vdf::parse(text)?;
    let Some(app) = vdf::object_at(
        &entries,
        &[
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
            "108600",
        ],
    ) else {
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
    let cleaned = value.replace(FLAGS, "").trim().to_owned();
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
}

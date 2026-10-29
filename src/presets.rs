use crate::errors::DevinitError;

use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn config_error(message: impl Into<String>) -> DevinitError {
    DevinitError::ConfigurationError(message.into())
}

fn key(name: &str) -> Value {
    Value::String(name.to_owned())
}

fn global_config_path() -> Result<PathBuf, DevinitError> {
    if let Some(xdg_home) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg_home.is_empty()
    {
        let base = PathBuf::from(xdg_home);

        if !base.is_absolute() {
            return Err(DevinitError::ConfigurationError(
                "XDG_CONFIG_HOME must be an absolute path".into(),
            ));
        }

        return Ok(base.join("devinit/devinit.yaml"));
    }

    let home = std::env::var_os("HOME").ok_or_else(|| {
        DevinitError::ConfigurationError("HOME environment variable is not defined".into())
    })?;

    Ok(PathBuf::from(home).join(".config/devinit/devinit.yaml"))
}

fn read_presets_from(path: &Path) -> Result<Option<Value>, DevinitError> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path).map_err(DevinitError::FileIOError)?;

    let document: Value = serde_yaml::from_str(&contents)?;

    if !document.is_mapping() {
        return Err(config_error("devinit.yaml must contain a YAML mapping"));
    }

    Ok(Some(document))
}

fn read_presets(_project_root: &Path) -> Result<Option<Value>, DevinitError> {
    let path = global_config_path()?;
    read_presets_from(&path)
}

/// Returns the configured backend URL.
///
/// Priority:
/// 1. DEVINIT_API_URL environment variable
/// 2. devinit.yaml default_url
/// 3. Built-in default
pub fn default_url(project_root: &Path) -> Result<String, DevinitError> {
    if let Ok(url) = std::env::var("DEVINIT_API_URL") {
        return validate_url(&url);
    }

    let Some(document) = read_presets(project_root)? else {
        return Ok("http://localhost:5167".to_owned());
    };

    match document.get("default_url") {
        Some(Value::String(url)) => validate_url(url),
        Some(_) => Err(config_error("default_url must be a string")),
        None => Ok("http://localhost:5167".to_owned()),
    }
}

fn validate_url(url: &str) -> Result<String, DevinitError> {
    let url = url.trim().trim_end_matches('/');

    if url.is_empty() {
        return Err(config_error("default_url cannot be empty"));
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(config_error(
            "default_url must start with http:// or https://",
        ));
    }

    if url.chars().any(char::is_whitespace) {
        return Err(config_error("default_url must not contain whitespace"));
    }

    Ok(url.to_owned())
}

/// Expands service presets before normal service resolution.
///
/// A preset reference must have this exact shape:
///
/// - use: db
///
/// Presets contain ordinary service declarations.
fn expand_compose_presets_from(
    document: &mut Value,
    preset_path: &Path,
) -> Result<(), DevinitError> {
    let Some(services) = document
        .get_mut("services")
        .and_then(Value::as_sequence_mut)
    else {
        return Ok(());
    };

    let uses_presets = services.iter().any(|service| {
        service
            .as_mapping()
            .is_some_and(|mapping| mapping.contains_key(key("use")))
    });

    if !uses_presets {
        return Ok(());
    }

    let presets = read_presets_from(preset_path)?.ok_or_else(|| {
        config_error("compile.yaml references presets, but devinit.yaml was not found")
    })?;

    let compose_presets = presets
        .get("docker_compose")
        .and_then(Value::as_mapping)
        .ok_or_else(|| config_error("devinit.yaml must define docker_compose presets"))?;

    let mut expanded = Vec::new();

    for (index, service) in services.iter().enumerate() {
        let Some(mapping) = service.as_mapping() else {
            expanded.push(service.clone());
            continue;
        };

        let Some(preset_name) = mapping.get(key("use")) else {
            expanded.push(service.clone());
            continue;
        };

        if mapping.len() != 1 {
            return Err(config_error(format!(
                "services[{index}]: preset reference must contain only 'use'",
            )));
        }

        let name = preset_name
            .as_str()
            .ok_or_else(|| config_error(format!("services[{index}].use must be a string",)))?;

        let preset = compose_presets
            .get(key(name))
            .ok_or_else(|| config_error(format!("Unknown docker_compose preset: '{name}'",)))?;

        let preset_services = preset.as_sequence().ok_or_else(|| {
            config_error(format!("Preset '{name}' must contain a list of services",))
        })?;

        if preset_services.is_empty() {
            return Err(config_error(format!("Preset '{name}' cannot be empty",)));
        }

        for (preset_index, item) in preset_services.iter().enumerate() {
            let Some(item_mapping) = item.as_mapping() else {
                return Err(config_error(format!(
                    "Preset '{name}' item {preset_index} must be a service mapping",
                )));
            };

            if item_mapping.contains_key(key("use")) {
                return Err(config_error(format!(
                    "Nested preset references are not supported: '{name}'",
                )));
            }

            if !item_mapping.contains_key(key("name")) {
                return Err(config_error(format!(
                    "Preset '{name}' item {preset_index} requires 'name'",
                )));
            }

            expanded.push(item.clone());
        }
    }

    *services = expanded;

    Ok(())
}

pub fn expand_compose_presets(
    document: &mut Value,
    _project_root: &Path,
) -> Result<(), DevinitError> {
    let path = global_config_path()?;

    expand_compose_presets_from(document, &path)
}

/// Reads the raw devinit.yaml mapping.
///
/// This helper is intended for future Kubernetes and CI/CD presets.
pub fn load_preset_document(project_root: &Path) -> Result<Option<Mapping>, DevinitError> {
    read_presets(project_root)?
        .map(|document| {
            document
                .as_mapping()
                .cloned()
                .ok_or_else(|| config_error("Invalid devinit.yaml structure"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);

        let dir = std::env::temp_dir().join(format!("devinit-presets-{}-{id}", std::process::id()));

        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn expands_database_preset() {
        let dir = test_dir();

        fs::write(
            dir.join("devinit.yaml"),
            r#"
docker_compose:
  db:
    - name: postgresql
      version: "17"
    - name: redis
      version: "7"
"#,
        )
        .unwrap();

        let mut document: Value = serde_yaml::from_str(
            r#"
services:
  - use: db
  - name: backend
    image: backend:latest
"#,
        )
        .unwrap();

        expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).unwrap();

        let services = document["services"].as_sequence().unwrap();

        assert_eq!(services.len(), 3);
        assert_eq!(services[0]["name"].as_str(), Some("postgresql"));
        assert_eq!(services[1]["name"].as_str(), Some("redis"));
        assert_eq!(services[2]["name"].as_str(), Some("backend"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_unknown_preset() {
        let dir = test_dir();

        fs::write(dir.join("devinit.yaml"), "docker_compose: {}").unwrap();

        let mut document: Value = serde_yaml::from_str("services:\n  - use: missing\n").unwrap();

        assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml"),).is_err());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_nested_presets() {
        let dir = test_dir();

        fs::write(
            dir.join("devinit.yaml"),
            r#"
docker_compose:
  db:
    - use: another
"#,
        )
        .unwrap();

        let mut document: Value = serde_yaml::from_str("services:\n  - use: db\n").unwrap();

        assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).is_err());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ordinary_services_work_without_preset_file() {
        let dir = test_dir();

        let mut document: Value = serde_yaml::from_str(
            r#"
services:
  - name: redis
    image: redis:7
"#,
        )
        .unwrap();

        expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).unwrap();

        assert_eq!(document["services"][0]["name"].as_str(), Some("redis"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_invalid_default_url() {
        assert!(validate_url("localhost:3003").is_err());
        assert!(validate_url("http://localhost:3003").is_ok());
        assert!(validate_url("").is_err());
    }
}

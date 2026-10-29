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

/// Expands global Kubernetes and CI/CD presets.
///
/// This function only transforms YAML in memory.
/// It does not generate infrastructure files or execute commands.
pub fn expand_infrastructure_presets(
    document: &mut Value,
    project_root: &Path,
) -> Result<(), DevinitError> {
    let Some(root) = document.as_mapping() else {
        return Err(config_error("compile.yaml must contain a YAML mapping"));
    };

    let k8s_key = key("k8s");
    let cicd_key = key("cicd");
    let use_key = key("use");

    // Detect references without loading the global configuration.
    let needs_k8s = root
        .get(&k8s_key)
        .and_then(Value::as_mapping)
        .is_some_and(|mapping| mapping.contains_key(&use_key));

    let needs_cicd = root
        .get(&cicd_key)
        .and_then(Value::as_mapping)
        .is_some_and(|mapping| mapping.contains_key(&use_key));

    if !needs_k8s && !needs_cicd {
        return Ok(());
    }

    let presets = read_presets(project_root)?.ok_or_else(|| {
        config_error("compile.yaml references presets, but global devinit.yaml was not found")
    })?;

    let root = document
        .as_mapping_mut()
        .ok_or_else(|| config_error("compile.yaml must contain a YAML mapping"))?;

    if needs_k8s {
        let configured = presets
            .get("k8s")
            .and_then(Value::as_mapping)
            .ok_or_else(|| config_error("Global devinit.yaml does not define k8s presets"))?;

        let section = root
            .get_mut(&k8s_key)
            .and_then(Value::as_mapping_mut)
            .ok_or_else(|| config_error("k8s must be a mapping"))?;

        expand_k8s_section(section, configured)?;
    }

    if needs_cicd {
        let configured = presets
            .get("cicd")
            .and_then(Value::as_mapping)
            .ok_or_else(|| config_error("Global devinit.yaml does not define cicd presets"))?;

        let reference = root
            .get(&cicd_key)
            .and_then(Value::as_mapping)
            .ok_or_else(|| config_error("cicd must be a mapping"))?;

        if reference.len() != 1 {
            return Err(config_error(
                "cicd preset reference must contain only 'use'",
            ));
        }

        let name = reference
            .get(&use_key)
            .and_then(Value::as_str)
            .ok_or_else(|| config_error("cicd.use must be a string"))?;

        let pipeline = configured
            .get(key(name))
            .ok_or_else(|| config_error(format!("Unknown CI/CD preset: '{name}'")))?
            .clone();

        if !pipeline.is_mapping() {
            return Err(config_error(format!(
                "CI/CD preset '{name}' must be a mapping"
            )));
        }

        if pipeline.get("use").is_some() {
            return Err(config_error("Nested CI/CD presets are not supported"));
        }

        root.insert(cicd_key, pipeline);
    }

    Ok(())
}

fn expand_k8s_section(section: &mut Mapping, configured: &Mapping) -> Result<(), DevinitError> {
    let reference = section
        .remove(key("use"))
        .ok_or_else(|| config_error("k8s.use is missing"))?;

    let names: Vec<String> = match reference {
        Value::String(name) => vec![name],

        Value::Sequence(values) => {
            let mut names = Vec::with_capacity(values.len());

            for value in values {
                let name = value
                    .as_str()
                    .ok_or_else(|| config_error("k8s.use entries must be strings"))?;

                names.push(name.to_owned());
            }

            names
        }

        _ => {
            return Err(config_error("k8s.use must be a string or list of strings"));
        }
    };

    if names.is_empty() {
        return Err(config_error("k8s.use cannot be empty"));
    }

    for name in names {
        if !valid_preset_name(&name) {
            return Err(config_error(format!(
                "Invalid Kubernetes preset name: '{name}'"
            )));
        }

        let group = configured
            .get(key(&name))
            .and_then(Value::as_mapping)
            .ok_or_else(|| {
                config_error(format!("Unknown or invalid Kubernetes preset: '{name}'"))
            })?;

        if group.is_empty() {
            return Err(config_error(format!("Kubernetes preset '{name}' is empty")));
        }

        for (file_key, resource) in group {
            let file_key = file_key.as_str().ok_or_else(|| {
                config_error(format!(
                    "Kubernetes preset '{name}' has a non-string manifest name"
                ))
            })?;

            if !valid_preset_name(file_key) {
                return Err(config_error(format!(
                    "Invalid manifest name '{file_key}' in preset '{name}'"
                )));
            }

            if !resource.is_mapping() {
                return Err(config_error(format!(
                    "Kubernetes preset '{name}.{file_key}' must be a mapping"
                )));
            }

            if resource.get("use").is_some() {
                return Err(config_error("Nested Kubernetes presets are not supported"));
            }

            // Prefix file names to avoid collisions between groups.
            let output_name = format!("{name}-{file_key}");

            let output_key = key(&output_name);

            if section.contains_key(&output_key) {
                return Err(config_error(format!(
                    "Duplicate Kubernetes manifest: '{output_name}'"
                )));
            }

            section.insert(output_key, resource.clone());
        }
    }

    Ok(())
}

fn valid_preset_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
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

    #[test]
    fn expands_kubernetes_preset() {
        let configured: Mapping = serde_yaml::from_str(
            r#"
backend:
  deployment:
    template: deployment
    name: backend
    image: backend:latest

  service:
    template: service
    name: backend
    target: backend
"#,
        )
        .unwrap();

        let mut section: Mapping = serde_yaml::from_str(
            r#"
use:
  - backend
"#,
        )
        .unwrap();

        expand_k8s_section(&mut section, &configured).unwrap();

        assert!(section.contains_key(key("backend-deployment")));
        assert!(section.contains_key(key("backend-service")));
        assert!(!section.contains_key(key("use")));
    }

    #[test]
    fn rejects_unknown_kubernetes_preset() {
        let configured = Mapping::new();

        let mut section: Mapping = serde_yaml::from_str("use: missing\n").unwrap();

        assert!(expand_k8s_section(&mut section, &configured).is_err());
    }

    #[test]
    fn rejects_duplicate_kubernetes_output() {
        let configured: Mapping = serde_yaml::from_str(
            r#"
backend:
  deployment:
    template: deployment
    name: backend
    image: backend:latest
"#,
        )
        .unwrap();

        let mut section: Mapping = serde_yaml::from_str(
            r#"
use: backend

backend-deployment:
  apiVersion: v1
  kind: ConfigMap
  metadata:
    name: existing
"#,
        )
        .unwrap();

        assert!(expand_k8s_section(&mut section, &configured).is_err());
    }
}

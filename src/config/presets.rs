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
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA").ok_or_else(|| {
            DevinitError::ConfigurationError("APPDATA environment variable is not defined".into())
        })?;

        Ok(PathBuf::from(appdata).join("DevInit").join("devinit.yaml"))
    }

    #[cfg(not(target_os = "windows"))]
    {
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
        config_error("compile.yaml references presets, but global devinit.yaml was not found")
    })?;

    let compose_presets = presets
        .get("docker_compose")
        .and_then(Value::as_mapping)
        .ok_or_else(|| config_error("Global devinit.yaml must define docker_compose presets"))?;

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

        for field in mapping.keys() {
            let Some(field) = field.as_str() else {
                return Err(config_error(format!(
                    "services[{index}]: preset reference fields must be strings"
                )));
            };

            if field != "use" && field != "overrides" {
                return Err(config_error(format!(
                    "services[{index}]: unsupported preset field '{field}'"
                )));
            }
        }

        let name = preset_name
            .as_str()
            .ok_or_else(|| config_error(format!("services[{index}].use must be a string")))?;

        let preset = compose_presets
            .get(key(name))
            .ok_or_else(|| config_error(format!("Unknown docker_compose preset: '{name}'")))?;

        let preset_services = preset.as_sequence().ok_or_else(|| {
            config_error(format!("Preset '{name}' must contain a list of services"))
        })?;

        if preset_services.is_empty() {
            return Err(config_error(format!("Preset '{name}' cannot be empty")));
        }

        let overrides = match mapping.get(key("overrides")) {
            Some(value) => Some(value.as_mapping().ok_or_else(|| {
                config_error(format!("services[{index}].overrides must be a mapping"))
            })?),
            None => None,
        };

        // Check that every override targets a service in this preset.
        if let Some(overrides) = overrides {
            for (override_name, override_value) in overrides {
                let service_name = override_name.as_str().ok_or_else(|| {
                    config_error("Compose override service names must be strings")
                })?;

                if !override_value.is_mapping() {
                    return Err(config_error(format!(
                        "Overrides for '{name}.{service_name}' must be a mapping"
                    )));
                }

                let exists = preset_services
                    .iter()
                    .any(|item| item.get("name").and_then(Value::as_str) == Some(service_name));

                if !exists {
                    return Err(config_error(format!(
                        "Unknown service '{service_name}' in overrides for preset '{name}'"
                    )));
                }

                // The alias selects the service; changing its identity
                // would make dependency references ambiguous.
                if override_value.get("name").is_some() || override_value.get("use").is_some() {
                    return Err(config_error(format!(
                        "Cannot override 'name' or 'use' in preset '{name}.{service_name}'"
                    )));
                }
            }
        }

        for (preset_index, item) in preset_services.iter().enumerate() {
            let Some(item_mapping) = item.as_mapping() else {
                return Err(config_error(format!(
                    "Preset '{name}' item {preset_index} must be a service mapping"
                )));
            };

            if item_mapping.contains_key(key("use")) {
                return Err(config_error(format!(
                    "Nested preset references are not supported: '{name}'"
                )));
            }

            let service_name = item_mapping
                .get(key("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    config_error(format!(
                        "Preset '{name}' item {preset_index} requires a string 'name'"
                    ))
                })?;

            let mut resolved = item.clone();

            if let Some(overrides) = overrides
                && let Some(override_value) = overrides.get(key(service_name))
            {
                merge_yaml(&mut resolved, override_value);
            }

            expanded.push(resolved);
        }
    }

    // Replace the source list only after all presets resolve.
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

fn resolve_cicd_preset(reference: &Mapping, configured: &Mapping) -> Result<Value, DevinitError> {
    for field in reference.keys() {
        let Some(field) = field.as_str() else {
            return Err(config_error(
                "CI/CD preset reference fields must be strings",
            ));
        };

        if field != "use" && field != "overrides" {
            return Err(config_error(format!(
                "Unsupported CI/CD preset reference field: '{field}'"
            )));
        }
    }

    let name = reference
        .get(key("use"))
        .and_then(Value::as_str)
        .ok_or_else(|| config_error("cicd.use must be a string"))?;

    let mut pipeline = configured
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

    if let Some(overrides) = reference.get(key("overrides")) {
        if !overrides.is_mapping() {
            return Err(config_error("cicd.overrides must be a mapping"));
        }

        if overrides.get("use").is_some() {
            return Err(config_error("CI/CD overrides cannot contain 'use'"));
        }

        merge_yaml(&mut pipeline, overrides);
    }

    Ok(pipeline)
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

        let resolved = resolve_cicd_preset(reference, configured)?;

        root.insert(cicd_key, resolved);
    }

    Ok(())
}

/// Recursively merges YAML mappings.
///
/// - Mapping + Mapping: recursively merge keys.
/// - Other values: override replaces original.
/// - Sequences are replaced, not concatenated.
///
/// The original global preset is never modified.
fn merge_yaml(base: &mut Value, overrides: &Value) {
    match (base, overrides) {
        (Value::Mapping(base_map), Value::Mapping(override_map)) => {
            for (key, value) in override_map {
                if let Some(existing) = base_map.get_mut(key) {
                    merge_yaml(existing, value);
                } else {
                    base_map.insert(key.clone(), value.clone());
                }
            }
        }

        (base, overrides) => {
            *base = overrides.clone();
        }
    }
}

fn expand_k8s_section(section: &mut Mapping, configured: &Mapping) -> Result<(), DevinitError> {
    let use_key = key("use");

    // Work on a clone so errors don't partially modify the document.
    let mut result = section.clone();

    let reference = result
        .remove(&use_key)
        .ok_or_else(|| config_error("k8s.use is missing"))?;

    let entries = match reference {
        Value::Sequence(entries) => entries,
        Value::String(name) => vec![Value::String(name)],
        _ => {
            return Err(config_error("k8s.use must be a string or sequence"));
        }
    };

    if entries.is_empty() {
        return Err(config_error("k8s.use cannot be empty"));
    }

    for entry in entries {
        let (name, overrides) = match entry {
            Value::String(name) => (name, None),

            Value::Mapping(mapping) => {
                for field in mapping.keys() {
                    let Some(field) = field.as_str() else {
                        return Err(config_error(
                            "Kubernetes preset reference contains a non-string field",
                        ));
                    };

                    if field != "name" && field != "overrides" {
                        return Err(config_error(format!(
                            "Unsupported Kubernetes preset reference field: '{field}'"
                        )));
                    }
                }

                let name = mapping
                    .get(key("name"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        config_error("Kubernetes preset reference requires a string 'name'")
                    })?
                    .to_owned();

                let overrides = mapping.get(key("overrides")).cloned();

                (name, overrides)
            }

            _ => {
                return Err(config_error("k8s.use entries must be strings or mappings"));
            }
        };

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

        let override_map = match overrides.as_ref() {
            Some(value) => Some(value.as_mapping().ok_or_else(|| {
                config_error(format!(
                    "Overrides for Kubernetes preset '{name}' must be a mapping"
                ))
            })?),
            None => None,
        };

        // Every override must target an existing manifest in the group.
        if let Some(overrides) = override_map {
            for manifest_name in overrides.keys() {
                let manifest_name = manifest_name.as_str().ok_or_else(|| {
                    config_error("Kubernetes override manifest names must be strings")
                })?;

                if !group.contains_key(key(manifest_name)) {
                    return Err(config_error(format!(
                        "Unknown manifest '{manifest_name}' in overrides for preset '{name}'"
                    )));
                }

                let override_value = overrides
                    .get(key(manifest_name))
                    .expect("validated override key");

                if !override_value.is_mapping() {
                    return Err(config_error(format!(
                        "Overrides for '{name}.{manifest_name}' must be a mapping"
                    )));
                }
            }
        }

        for (file_key, resource) in group {
            let file_name = file_key.as_str().ok_or_else(|| {
                config_error(format!(
                    "Kubernetes preset '{name}' contains a non-string manifest name"
                ))
            })?;

            if !valid_preset_name(file_name) {
                return Err(config_error(format!(
                    "Invalid manifest name '{file_name}' in preset '{name}'"
                )));
            }

            if !resource.is_mapping() {
                return Err(config_error(format!(
                    "Kubernetes preset '{name}.{file_name}' must be a mapping"
                )));
            }

            let mut resolved = resource.clone();

            if let Some(overrides) = override_map
                && let Some(override_value) = overrides.get(key(file_name))
            {
                merge_yaml(&mut resolved, override_value);
            }

            // Nested references remain unsupported.
            if resolved.get("use").is_some() {
                return Err(config_error("Nested Kubernetes presets are not supported"));
            }

            let output_name = format!("{name}-{file_name}");
            let output_key = key(&output_name);

            if result.contains_key(&output_key) {
                return Err(config_error(format!(
                    "Duplicate Kubernetes manifest: '{output_name}'"
                )));
            }

            result.insert(output_key, resolved);
        }
    }

    *section = result;

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
#[path = "../tests/presets_tests.rs"]
mod tests;

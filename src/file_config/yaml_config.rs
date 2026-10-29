use crate::errors::DevinitError;
use crate::errors::ValidationError;
use crate::file_config::file_validator::{ensure_file_not_empty, ensure_images_are_new};
use crate::models::compile_struct::CompileSpec;
use crate::models::docker_compose_struct::{ConfigsList, RecipeCompose};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Loads and parses a compile specification from YAML.
pub fn load_compile(path: &Path) -> Result<CompileSpec, DevinitError> {
    // Report a clear error when the requested file is missing.
    if !path.exists() {
        return Err(DevinitError::ConfigurationError(format!(
            "compile file '{}' was not found",
            path.display()
        )));
    }
    ensure_file_not_empty(path)?;
    let contents = std::fs::read_to_string(path).map_err(DevinitError::FileIOError)?;
    Ok(serde_yaml::from_str(&contents)?)
}

/// Appends one recipe to a YAML configuration file.
pub fn yaml_data(config_struct: &RecipeCompose, path: &PathBuf) -> std::io::Result<()> {
    append_recipes(std::slice::from_ref(config_struct), path)
}

/// Appends all recipes from a configuration list to a YAML file.
pub fn yaml_configs_data(configs_list: &ConfigsList, path: &PathBuf) -> std::io::Result<()> {
    append_recipes(&configs_list.configs, path)
}

/// Serializes recipes and appends them to the target file.
fn append_recipes(recipes: &[RecipeCompose], path: &PathBuf) -> std::io::Result<()> {
    // Nothing needs to be written for an empty recipe list.
    if recipes.is_empty() {
        return Ok(());
    }
    ensure_images_are_new(path, recipes)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    // Separate appended YAML documents from existing content.
    if path.metadata()?.len() > 0 {
        file.write_all(b"\n")?;
    }

    // Write each recipe under its service name.
    for recipe in recipes {
        let mut recipe_map = HashMap::new();
        let mut compose_recipe = recipe.clone();
        compose_recipe.env = None;
        recipe_map.insert(&recipe.name, &compose_recipe);
        let yaml = serde_yaml::to_string(&recipe_map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(yaml.as_bytes())?;
    }

    Ok(())
}

/// Loads compile.yaml and resolves service references
/// from the local registry before validation.
pub fn load_compile_with_registry(path: &Path) -> Result<CompileSpec, DevinitError> {
    if !path.exists() {
        return Err(DevinitError::ConfigurationError(format!(
            "compile file '{}' was not found",
            path.display()
        )));
    }

    ensure_file_not_empty(path)?;

    let contents = std::fs::read_to_string(path).map_err(DevinitError::FileIOError)?;

    let mut document: serde_yaml::Value = serde_yaml::from_str(&contents)?;

    crate::validator::project_validator::validate_declarations(&document).map_err(|message| {
        DevinitError::ValidationErrors(vec![ValidationError {
            path: "services".to_string(),
            message,
        }])
    })?;

    let project_root = path.parent().unwrap_or_else(|| Path::new("."));

    if let Some(services) = document
        .get_mut("services")
        .and_then(serde_yaml::Value::as_sequence_mut)
    {
        for (index, service) in services.iter_mut().enumerate() {
            let Some(mapping) = service.as_mapping() else {
                // Preserve existing scalar service syntax.
                continue;
            };

            // Explicit custom recipes already have an image.
            // They do not need registry resolution.
            if mapping.contains_key(serde_yaml::Value::String("image".into())) {
                continue;
            }

            let name = mapping
                .get(serde_yaml::Value::String("name".into()))
                .and_then(serde_yaml::Value::as_str)
                .map(str::to_owned);

            let Some(name) = name else {
                continue;
            };

            let version_value = mapping.get(serde_yaml::Value::String("version".into()));

            let version = match version_value {
                None => None,
                Some(value) => Some(value.as_str().ok_or_else(|| {
                    DevinitError::ConfigurationError(format!(
                        "services[{index}].version must be a string"
                    ))
                })?),
            };

            if mapping.len() != (1 + usize::from(version.is_some())) {
                return Err(DevinitError::ConfigurationError(format!(
                    "services[{index}]: registry references only support name and version"
                )));
            }

            let is_builtin = crate::service_registry::builtin_recipe(&name).is_some();

            let recipe = if is_builtin {
                // Reject unsupported versions of built-in services.
                let expected = crate::service_registry::builtin_recipe_version(&name, version)
                    .ok_or_else(|| {
                        DevinitError::ConfigurationError(format!(
                            "Unsupported version for service '{name}'"
                        ))
                    })?;

                crate::service_registry::cached_recipe_version(project_root, &name, version)
                    .map_err(DevinitError::FileIOError)?
                    .unwrap_or(expected)
            } else {
                crate::service_registry::cached_recipe_version(project_root, &name, version)
                    .map_err(DevinitError::FileIOError)?
                    .ok_or_else(|| {
                        DevinitError::ServiceNotFound(format!(
                            "{name} (version: {})",
                            version.unwrap_or("default")
                        ))
                    })?
            };

            *service = serde_yaml::to_value(recipe)?;
        }
    }

    crate::validator::project_validator::validate_resolved_services(&document).map_err(
        |message| {
            DevinitError::ValidationErrors(vec![ValidationError {
                path: "services".to_string(),
                message,
            }])
        },
    )?;

    Ok(serde_yaml::from_value(document)?)
}

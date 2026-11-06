use crate::errors::DevinitError;
use crate::models::docker_compose_struct::File;
use crate::models::docker_compose_struct::RecipeCompose;
use crate::service_registry;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;

const OVERRIDE_FIELDS: &[&str] = &[
    "description",
    "ports",
    "environment",
    "volumes",
    "networks",
    "depends_on",
    "restart",
    "command",
    "files",
    "env",
    "notes",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectorySelection {
    dir: String,
    files: Vec<String>,
}

fn key(name: &str) -> Value {
    Value::String(name.to_owned())
}

fn field<'a>(map: &'a Mapping, name: &str) -> Option<&'a Value> {
    map.get(key(name))
}

fn configuration_error(message: impl Into<String>) -> DevinitError {
    DevinitError::ConfigurationError(message.into())
}

/// Resolves all registry references in a compile document.
///
/// Custom services containing an explicit image remain unchanged.
/// Registry services are replaced with complete recipes.
pub fn resolve_document(document: &mut Value, project_root: &Path) -> Result<(), DevinitError> {
    let Some(services) = document
        .get_mut("services")
        .and_then(Value::as_sequence_mut)
    else {
        return Ok(());
    };

    for (index, service) in services.iter_mut().enumerate() {
        // Resolve shorthand service declarations using the registry.
        //
        // Example:
        // services:
        //   - postgresql
        //   - redis
        //
        // Every shorthand service is converted into a complete
        // RecipeCompose before CompileSpec deserialization.
        if let Some(name) = service.as_str() {
            let name = name.trim().to_ascii_lowercase();

            if name.is_empty() {
                return Err(configuration_error(format!(
                    "services[{index}]: service name cannot be empty"
                )));
            }

            let recipe = resolve_recipe(project_root, &name, None)?;

            *service = serde_yaml::to_value(recipe)?;
            continue;
        }

        let Some(mapping) = service.as_mapping() else {
            // Leave unsupported value types to the existing
            // configuration deserializer and validators.
            continue;
        };

        // Explicit images define custom services.
        if field(mapping, "image").is_some() {
            continue;
        }

        let Some(name) = field(mapping, "name").and_then(Value::as_str) else {
            // Existing validators handle missing names.
            continue;
        };

        let name = name.to_owned();

        let version = match field(mapping, "version") {
            Some(Value::String(version)) => Some(version.clone()),

            Some(_) => {
                return Err(configuration_error(format!(
                    "services[{index}].version must be a string"
                )));
            }

            None => None,
        };

        // Reject unknown declaration fields.
        for field_name in mapping.keys() {
            let Some(field_name) = field_name.as_str() else {
                return Err(configuration_error(format!(
                    "services[{index}] contains a non-string field"
                )));
            };

            if field_name == "name"
                || field_name == "version"
                || field_name == "directories"
                || OVERRIDE_FIELDS.contains(&field_name)
            {
                continue;
            }

            return Err(configuration_error(format!(
                "services[{index}]: unknown field '{field_name}'"
            )));
        }

        let version_ref = version.as_deref();

        let recipe = resolve_recipe(project_root, &name, version_ref)?;

        if field(mapping, "directories").is_some()
            && (field(mapping, "files").is_some() || field(mapping, "volumes").is_some())
        {
            return Err(configuration_error(format!(
                "services[{index}]: directories cannot be combined \
                with files or volumes overrides"
            )));
        }

        let mut resolved = apply_overrides(recipe, mapping, index)?;

        if let Some(directories) = field(mapping, "directories") {
            resolved = apply_directory_overrides(resolved, directories, index)?;
        }

        // Replace the shorthand declaration with a full recipe.
        *service = serde_yaml::to_value(resolved)?;
    }

    Ok(())
}

/// Resolves a service from the installed YAML registry
/// or the local recipe cache.
///
/// Installed YAML definitions always take priority.
/// Cached recipes are used only for unknown services.
pub fn resolve_recipe(
    project_root: &Path,
    name: &str,
    version: Option<&str>,
) -> Result<RecipeCompose, DevinitError> {
    let name = name.trim().to_ascii_lowercase();

    // 1. Installed YAML registry is authoritative.
    if let Some(recipe) = service_registry::packaged_recipe_version(&name, version)
        .map_err(DevinitError::FileIOError)?
    {
        return Ok(recipe);
    }

    // 2. Local cache for previously installed remote recipes.
    service_registry::cached_recipe_version(project_root, &name, version)
        .map_err(DevinitError::FileIOError)?
        .ok_or_else(|| {
            DevinitError::ServiceNotFound(format!(
                "{} (version: {})",
                name,
                version.unwrap_or("default")
            ))
        })
}

fn apply_directory_overrides(
    mut recipe: RecipeCompose,
    value: &Value,
    index: usize,
) -> Result<RecipeCompose, DevinitError> {
    let selections: Vec<DirectorySelection> = serde_yaml::from_value(value.clone())
        .map_err(|error| configuration_error(format!("services[{index}].directories: {error}")))?;

    let service_name = recipe.name.clone();
    let original_files = recipe.files.take().unwrap_or_default();
    let original_volumes = recipe.volumes.take().unwrap_or_default();

    let mut generated_sources = BTreeSet::new();

    for file in &original_files {
        generated_sources.insert(format!("./{}", file.path));
    }

    // Retain volumes that are not associated with generated files.
    let mut volumes: Vec<String> = original_volumes
        .iter()
        .filter(|volume| {
            let source = volume.split(':').next().unwrap_or_default();
            !generated_sources.contains(source)
        })
        .cloned()
        .collect();

    let mut files = Vec::<File>::new();
    let mut used_names = BTreeSet::new();
    let mut used_paths = BTreeSet::new();

    for selection in selections {
        let directory = Path::new(&selection.dir);

        if selection.dir.trim().is_empty()
            || selection.dir.contains('\\')
            || selection.dir.contains(':')
            || !directory
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(configuration_error(format!(
                "services[{index}]: invalid directory '{}'. \
                 Use a relative path inside the project.",
                selection.dir
            )));
        }

        for name in selection.files {
            let filename = Path::new(&name);

            if name.trim().is_empty()
                || name.contains('\\')
                || name.contains(':')
                || filename.components().count() != 1
                || !filename
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err(configuration_error(format!(
                    "services[{index}]: invalid file name '{name}'"
                )));
            }

            if !used_names.insert(name.clone()) {
                return Err(configuration_error(format!(
                    "services[{index}]: file '{name}' \
                     was selected more than once"
                )));
            }

            let templates: Vec<&File> = original_files
                .iter()
                .filter(|file| {
                    Path::new(&file.path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        == Some(name.as_str())
                })
                .collect();

            let template = match templates.as_slice() {
                [] => {
                    let available = original_files
                        .iter()
                        .filter_map(|file| {
                            Path::new(&file.path)
                                .file_name()
                                .and_then(|value| value.to_str())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    return Err(configuration_error(format!(
                        "Service '{service_name}' has no file '{name}'. \
                         Available files: {available}"
                    )));
                }
                [template] => *template,
                _ => {
                    return Err(configuration_error(format!(
                        "Service '{service_name}' has multiple \
                         templates named '{name}'. \
                         Use a unique file name in the recipe."
                    )));
                }
            };

            let destination = directory.join(&name);
            let destination = destination.to_string_lossy().into_owned();

            if !used_paths.insert(destination.clone()) {
                return Err(configuration_error(format!(
                    "services[{index}]: duplicate output path '{destination}'"
                )));
            }

            // Reuse content from the built-in recipe.
            files.push(File {
                path: destination.clone(),
                content: template.content.clone(),
            });

            // Relocate only mounts belonging to this template.
            let old_source = format!("./{}", template.path);

            for volume in &original_volumes {
                if let Some((source, target)) = volume.split_once(':')
                    && source == old_source
                {
                    volumes.push(format!("./{destination}:{target}"));
                }
            }
        }
    }

    // Every file provided by a built-in recipe is required
    // when directories are explicitly overridden.
    //
    // Users may relocate files, but cannot silently remove
    // files referenced by the default service configuration.
    let missing_files: Vec<String> = original_files
        .iter()
        .filter_map(|file| {
            let name = Path::new(&file.path).file_name()?.to_str()?;

            if used_names.contains(name) {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect();

    if !missing_files.is_empty() {
        return Err(configuration_error(format!(
            "Service '{service_name}': missing required configuration files: {}. \
             Include these files in directories, or remove the \
             directories field to use the default layout.",
            missing_files.join(", ")
        )));
    }

    recipe.files = if files.is_empty() { None } else { Some(files) };

    recipe.volumes = if volumes.is_empty() {
        None
    } else {
        Some(volumes)
    };

    Ok(recipe)
}

/// Applies user overrides to a resolved service recipe.
///
/// Arrays are replaced, not concatenated.
/// Missing fields retain their registry defaults.
/// Explicit null removes an optional setting.
pub fn apply_overrides(
    recipe: RecipeCompose,
    declaration: &Mapping,
    index: usize,
) -> Result<RecipeCompose, DevinitError> {
    let original_name = recipe.name.clone();

    let mut resolved = serde_yaml::to_value(recipe)?;

    let resolved_map = resolved
        .as_mapping_mut()
        .ok_or_else(|| configuration_error("Invalid internal recipe representation"))?;

    for field_name in OVERRIDE_FIELDS {
        if let Some(value) = field(declaration, field_name) {
            resolved_map.insert(key(field_name), value.clone());
        }
    }

    // Deserialize into the existing strongly typed model.
    let result: RecipeCompose = serde_yaml::from_value(resolved).map_err(|error| {
        configuration_error(format!(
            "services[{index}] ('{original_name}'): \
                 invalid override: {error}"
        ))
    })?;

    Ok(result)
}

#[cfg(test)]
#[path = "../tests/resolver_tests.rs"]
mod tests;

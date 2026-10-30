use crate::errors::DevinitError;
use crate::models::docker_compose_struct::RecipeCompose;
use crate::service_registry;

use serde_yaml::{Mapping, Value};
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
                || OVERRIDE_FIELDS.contains(&field_name)
            {
                continue;
            }

            return Err(configuration_error(format!(
                "services[{index}]: unknown field '{field_name}'"
            )));
        }

        let version_ref = version.as_deref();

        // Resolve the immutable base recipe.
        let recipe = resolve_recipe(project_root, &name, version_ref)?;

        // Apply explicitly declared overrides.
        let resolved = apply_overrides(recipe, mapping, index)?;

        // Replace the shorthand declaration with a full recipe.
        *service = serde_yaml::to_value(resolved)?;
    }

    Ok(())
}

/// Resolves a service from the built-in registry or local cache.
///
/// The built-in registry is authoritative for supported
/// built-in versions. Unknown services require a cached recipe.
pub fn resolve_recipe(
    project_root: &Path,
    name: &str,
    version: Option<&str>,
) -> Result<RecipeCompose, DevinitError> {
    // Prefer service definitions distributed as YAML files.
    //
    // Invalid registry files and unsupported versions must produce
    // explicit errors rather than silently falling back to Rust defaults.
    if let Some(packaged_recipe) = service_registry::packaged_recipe_version(name, version)
        .map_err(DevinitError::FileIOError)?
    {
        let normalized_name = name.trim().to_ascii_lowercase();

        let cached =
            service_registry::cached_recipe_version(project_root, &normalized_name, version)
                .map_err(DevinitError::FileIOError)?;

        return Ok(cached.unwrap_or(packaged_recipe));
    }

    let builtin = service_registry::builtin_recipe(name);

    if builtin.is_some() {
        let expected =
            service_registry::builtin_recipe_version(name, version).ok_or_else(|| {
                configuration_error(format!(
                    "Unsupported version '{version:?}' \
                         for built-in service '{name}'"
                ))
            })?;

        let cached = service_registry::cached_recipe_version(project_root, name, version)
            .map_err(DevinitError::FileIOError)?;

        return Ok(cached.unwrap_or(expected));
    }

    service_registry::cached_recipe_version(project_root, name, version)
        .map_err(DevinitError::FileIOError)?
        .ok_or_else(|| {
            DevinitError::ServiceNotFound(format!(
                "{name} (version: {})",
                version.unwrap_or("default")
            ))
        })
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

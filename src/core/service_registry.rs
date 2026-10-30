use crate::models::docker_compose_struct::RecipeCompose;
use crate::models::services_struct::Services;
use crate::service_definitions::load_service_registry;
use crate::service_paths::find_services_directory;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const VERSIONED: &[(&str, &[&str])] =
    &[("postgresql", &["15", "16", "17"]), ("redis", &["6", "7"])];

pub fn supported_versions(name: &str) -> Option<&'static [&'static str]> {
    VERSIONED
        .iter()
        .find(|(service, _)| *service == name)
        .map(|(_, versions)| *versions)
}

/// Loads a service recipe from the installed YAML registry.
///
/// Returns:
/// - Ok(Some(recipe)) if the service exists.
/// - Ok(None) if the service is not defined in the YAML registry.
/// - Err(error) if the registry cannot be loaded or is invalid.
///
/// Registry errors are never silently ignored.
pub fn packaged_recipe_version(
    name: &str,
    version: Option<&str>,
) -> io::Result<Option<RecipeCompose>> {
    let directory = find_services_directory()?;
    let registry = load_service_registry(&directory)?;

    let normalized_name = name.trim().to_ascii_lowercase();

    let Some(definition) = registry.get(&normalized_name) else {
        return Ok(None);
    };

    definition.to_recipe(version).map(Some)
}

pub fn builtin_recipe(name: &str) -> Option<RecipeCompose> {
    builtin_recipe_version(name, None)
}

pub fn builtin_recipe_version(name: &str, version: Option<&str>) -> Option<RecipeCompose> {
    let name = name.trim().to_ascii_lowercase();

    let service: Services = serde_json::from_value(serde_json::Value::String(name.clone())).ok()?;

    let mut recipe = service.to_recipe();

    match (name.as_str(), version) {
        ("postgresql", Some(v)) if ["15", "16", "17"].contains(&v) => {
            recipe.image = format!("postgres:{v}");
        }
        ("redis", Some(v)) if ["6", "7"].contains(&v) => {
            recipe.image = format!("redis:{v}");
        }
        (_, Some(v)) if recipe.image.rsplit(':').next() != Some(v) => {
            return None;
        }
        (_, Some(_)) => {}
        _ => {}
    }

    Some(recipe)
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.bytes().all(|b| {
            b.is_ascii_lowercase()
                || b.is_ascii_uppercase()
                || b.is_ascii_digit()
                || b == b'-'
                || b == b'_'
                || b == b'.'
        })
        && value != "."
        && value != ".."
}

fn cache_path(root: &Path, name: &str, version: Option<&str>) -> io::Result<PathBuf> {
    if !valid_component(name) || version.is_some_and(|v| !valid_component(v)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid service name or version",
        ));
    }

    let base = root.join(".devinit/cache/recipes");

    Ok(match version {
        Some(v) => base.join(name).join(format!("{v}.yaml")),
        None => base.join(format!("{name}.yaml")),
    })
}

pub fn cache_recipe(root: &Path, recipe: &RecipeCompose) -> io::Result<PathBuf> {
    cache_recipe_version(root, recipe, None)
}

pub fn cache_recipe_version(
    root: &Path,
    recipe: &RecipeCompose,
    version: Option<&str>,
) -> io::Result<PathBuf> {
    let path = cache_path(root, &recipe.name, version)?;

    let yaml =
        serde_yaml::to_string(recipe).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Never silently overwrite cached recipes.
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return match cached_recipe_version(root, &recipe.name, version)? {
                Some(existing) if existing.image == recipe.image => Ok(path),
                _ => Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Cached recipe differs from requested recipe",
                )),
            };
        }
        Err(e) => return Err(e),
    };

    if let Err(e) = file.write_all(yaml.as_bytes()) {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(e);
    }

    Ok(path)
}

pub fn cached_recipe(root: &Path, name: &str) -> io::Result<Option<RecipeCompose>> {
    cached_recipe_version(root, name, None)
}

pub fn cached_recipe_version(
    root: &Path,
    name: &str,
    version: Option<&str>,
) -> io::Result<Option<RecipeCompose>> {
    let path = cache_path(root, name, version)?;

    let yaml = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };

    let recipe: RecipeCompose =
        serde_yaml::from_str(&yaml).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if recipe.name != name {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cached recipe name mismatch",
        ));
    }

    if let Some(v) = version {
        // Built-in recipes must resolve to their pinned image tag.
        if let Some(expected) = builtin_recipe_version(name, Some(v))
            && recipe.image != expected.image
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Cached image does not match the requested version",
            ));
        }
    }

    Ok(Some(recipe))
}

#[cfg(test)]
#[path = "../tests/service_registry_tests.rs"]
mod tests;

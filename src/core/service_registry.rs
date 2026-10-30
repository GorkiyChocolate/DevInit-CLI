use crate::models::docker_compose_struct::RecipeCompose;
use crate::service_definitions::load_service_registry;
use crate::service_paths::find_services_directory;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    version: Option<String>,
    sha256: String,
}

fn metadata_path(recipe_path: &Path) -> PathBuf {
    recipe_path.with_extension("meta.json")
}

fn recipe_checksum(contents: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents);
    format!("{:x}", hasher.finalize())
}

fn invalid_cache(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
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
    let meta_path = metadata_path(&path);

    let yaml = serde_yaml::to_string(recipe).map_err(|e| invalid_cache(e.to_string()))?;

    let metadata = CacheMetadata {
        version: version.map(str::to_owned),
        sha256: recipe_checksum(yaml.as_bytes()),
    };

    let metadata_json =
        serde_json::to_vec_pretty(&metadata).map_err(|e| invalid_cache(e.to_string()))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // An existing recipe must be identical in every field.
    // Never overwrite it silently.
    if path.exists() {
        let existing = cached_recipe_version(root, &recipe.name, version)?
            .ok_or_else(|| invalid_cache("Cached recipe disappeared"))?;

        let existing_yaml =
            serde_yaml::to_string(&existing).map_err(|e| invalid_cache(e.to_string()))?;

        if existing_yaml == yaml {
            return Ok(path);
        }

        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Cached recipe differs from requested recipe",
        ));
    }

    // Prevent silent replacement of an existing metadata file.
    if meta_path.exists() {
        return Err(invalid_cache("Orphaned cache metadata already exists"));
    }

    let mut recipe_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;

    if let Err(error) = recipe_file.write_all(yaml.as_bytes()) {
        drop(recipe_file);
        let _ = fs::remove_file(&path);
        return Err(error);
    }

    if let Err(error) = recipe_file.sync_all() {
        drop(recipe_file);
        let _ = fs::remove_file(&path);
        return Err(error);
    }

    drop(recipe_file);

    // Write metadata after the recipe.
    // A missing metadata file makes the cache invalid.
    let metadata_result = (|| -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&meta_path)?;

        file.write_all(&metadata_json)?;
        file.sync_all()?;

        Ok(())
    })();

    if let Err(error) = metadata_result {
        let _ = fs::remove_file(&meta_path);
        let _ = fs::remove_file(&path);
        return Err(error);
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
    let meta_path = metadata_path(&path);

    let yaml = match fs::read(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if meta_path.exists() {
                return Err(invalid_cache("Cache metadata exists without recipe"));
            }

            return Ok(None);
        }
        Err(error) => return Err(error),
    };

    let metadata_bytes = fs::read(&meta_path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            invalid_cache("Missing cache metadata; recipe must be re-added")
        } else {
            error
        }
    })?;

    let metadata: CacheMetadata =
        serde_json::from_slice(&metadata_bytes).map_err(|e| invalid_cache(e.to_string()))?;

    // Verify the requested cache version.
    if metadata.version.as_deref() != version {
        return Err(invalid_cache("Cached recipe version mismatch"));
    }

    // Verify the recipe bytes before deserialization.
    if recipe_checksum(&yaml) != metadata.sha256 {
        return Err(invalid_cache("Cached recipe checksum mismatch"));
    }

    let recipe: RecipeCompose =
        serde_yaml::from_slice(&yaml).map_err(|e| invalid_cache(e.to_string()))?;

    if recipe.name != name {
        return Err(invalid_cache("Cached recipe name mismatch"));
    }

    Ok(Some(recipe))
}

#[cfg(test)]
#[path = "../tests/service_registry_tests.rs"]
mod tests;

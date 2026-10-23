use crate::models::docker_compose_struct::RecipeCompose;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

/// Ensures that a file contains non-whitespace content.
pub fn ensure_file_not_empty(path: &Path) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Reject files that contain no meaningful content.
    if contents.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file '{}' is empty", path.display()),
        ));
    }

    Ok(())
}

/// Ensures that all recipe images are new and unique.
pub fn ensure_images_are_new(path: &Path, recipes: &[RecipeCompose]) -> io::Result<()> {
    let existing_images = read_existing_images(path)?;
    let mut images_to_add = HashSet::new();

    // Check every requested image against existing and current entries.
    for recipe in recipes {
        // Reject duplicate image references.
        if existing_images.contains(&recipe.image) || !images_to_add.insert(recipe.image.clone()) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Image '{}' already exists in YAML file", recipe.image),
            ));
        }
    }

    Ok(())
}

/// Ensures that a configuration name is not already recorded.
pub fn ensure_name_is_new(path: &Path, name: &str) -> io::Result<()> {
    // Reject a name already present in the environment file.
    if read_existing_names(path)?
        .iter()
        .any(|existing| existing == name)
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Config '{}' already exists in env file", name),
        ));
    }

    Ok(())
}

/// Reads image names from an existing YAML configuration file.
fn read_existing_images(path: &Path) -> io::Result<HashSet<String>> {
    // A missing file has no existing images.
    if !path.exists() {
        return Ok(HashSet::new());
    }

    let mut contents = String::new();
    File::open(path)?.read_to_string(&mut contents)?;
    // An empty file has no existing images.
    if contents.trim().is_empty() {
        return Ok(HashSet::new());
    }

    let recipes: std::collections::HashMap<String, RecipeCompose> = serde_yaml::from_str(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(recipes.into_values().map(|recipe| recipe.image).collect())
}

/// Reads configuration names from marker lines in an environment file.
fn read_existing_names(path: &Path) -> io::Result<Vec<String>> {
    // A missing file has no existing names.
    if !path.exists() {
        return Ok(Vec::new());
    }

    BufReader::new(File::open(path)?)
        .lines()
        .filter_map(|line| match line {
            Ok(value) => value
                .strip_prefix("# devinit config: ")
                .map(|name| Ok(name.trim().to_owned())),
            Err(error) => Some(Err(error)),
        })
        .collect()
}

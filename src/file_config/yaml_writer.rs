use crate::errors::GenerationError;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Serializes a value to YAML and atomically replaces the target file.
pub fn write_yaml<T: Serialize>(value: &T, path: &Path) -> Result<PathBuf, GenerationError> {
    // Create the output directory when the path has a parent.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let yaml = serde_yaml::to_string(value)?;
    let temporary_path = path.with_extension("tmp");
    fs::write(&temporary_path, yaml)?;
    fs::rename(temporary_path, path)?;
    Ok(path.to_path_buf())
}

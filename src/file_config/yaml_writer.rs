use crate::errors::GenerationError;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// Writes YAML without replacing an existing output file.
/// Existing files are deliberately treated as a conflict, even when identical.
pub fn write_yaml<T: Serialize>(value: &T, path: &Path) -> Result<PathBuf, GenerationError> {
    write_yaml_with_options(value, path, false)
}

/// When `force` is true, replaces an existing file using a temporary file.
/// When false, installs the complete file with a no-clobber hard link.
pub fn write_yaml_with_options<T: Serialize>(
    value: &T,
    path: &Path,
    force: bool,
) -> Result<PathBuf, GenerationError> {
    // Serialize *before* touching the filesystem.
    let yaml = serde_yaml::to_string(value)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create a unique temporary file in the same directory as the destination.
    let (temp_path, mut file) = loop {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing file name"))?
            .to_string_lossy();
        let candidate =
            path.with_file_name(format!(".{name}.devinit-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    };

    let result = (|| -> io::Result<()> {
        file.write_all(yaml.as_bytes())?;
        file.sync_all()?;
        drop(file);
        if force {
            fs::rename(&temp_path, path)?;
        } else {
            // Atomic no-clobber installation. Unlike rename, hard_link will
            // return AlreadyExists if the destination already exists.
            fs::hard_link(&temp_path, path)?;
        }
        Ok(())
    })();

    // With no-clobber installation the temporary name is no longer needed;
    // on error it must also be removed so that failed writes leave no junk.
    let _ = fs::remove_file(&temp_path);
    result?;
    Ok(path.to_path_buf())
}

#[cfg(test)]
#[path = "../tests/yaml_writer_tests.rs"]
mod tests;

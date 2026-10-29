use crate::errors::GenerationError;

use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WRITE_ID: AtomicU64 = AtomicU64::new(0);

/// A file produced by an infrastructure generator.
///
/// The path is relative to the output directory.
/// No filesystem write occurs during construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

impl GeneratedFile {
    pub fn new(
        relative_path: impl Into<PathBuf>,
        contents: impl Into<String>,
    ) -> Result<Self, GenerationError> {
        let relative_path = relative_path.into();

        validate_relative_path(&relative_path)?;

        Ok(Self {
            relative_path,
            contents: contents.into(),
        })
    }
}

/// A collection of generated files ready for preflight checks.
///
/// Files are stored in deterministic path order.
#[derive(Debug, Default)]
pub struct GenerationPlan {
    files: BTreeMap<PathBuf, GeneratedFile>,
}

impl GenerationPlan {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to the plan without touching the filesystem.
    ///
    /// Adding the same path twice is an error, even when the
    /// generated contents are identical.
    pub fn add(&mut self, file: GeneratedFile) -> Result<(), GenerationError> {
        let path = file.relative_path.clone();

        if self.files.contains_key(&path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Duplicate generated output path: {}", path.display()),
            )
            .into());
        }

        self.files.insert(path, file);

        Ok(())
    }

    /// Returns generated files in deterministic path order.
    pub fn files(&self) -> impl Iterator<Item = &GeneratedFile> {
        self.files.values()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Checks destination conflicts before writing.
    ///
    /// Existing files are rejected unless force=true.
    /// This method does not create or modify files.
    pub fn preflight(&self, output_dir: &Path, force: bool) -> Result<(), GenerationError> {
        for file in self.files.values() {
            validate_parent_directories(output_dir, &file.relative_path)?;

            let destination = output_dir.join(&file.relative_path);

            // Never silently overwrite directory entries.
            match fs::symlink_metadata(&destination) {
                Ok(metadata) => {
                    if metadata.is_dir() {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            format!("Output path is a directory: {}", destination.display()),
                        )
                        .into());
                    }

                    if metadata.file_type().is_symlink() {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            format!("Output path is a symbolic link: {}", destination.display()),
                        )
                        .into());
                    }

                    if !metadata.is_file() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!(
                                "Output path is not a regular file: {}",
                                destination.display()
                            ),
                        )
                        .into());
                    }

                    if !force {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            format!(
                                "Refusing to overwrite {} (use --force)",
                                destination.display()
                            ),
                        )
                        .into());
                    }
                }

                Err(error) if error.kind() == io::ErrorKind::NotFound => {}

                Err(error) => return Err(error.into()),
            }
        }

        Ok(())
    }

    /// Writes all planned files after global preflight.
    ///
    /// Files are written through temporary files.
    /// Existing destinations are protected unless force is enabled.
    ///
    /// Note: this is not a multi-file transaction.
    pub fn write(&self, output_dir: &Path, force: bool) -> Result<Vec<PathBuf>, GenerationError> {
        // Check all destinations before starting any writes.
        self.preflight(output_dir, force)?;

        let mut written = Vec::with_capacity(self.len());

        for file in self.files.values() {
            let destination = output_dir.join(&file.relative_path);

            // Recheck parent paths before creating directories.
            validate_parent_directories(output_dir, &file.relative_path)?;

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }

            // Recheck the destination immediately before writing.
            // This reduces, but does not eliminate, filesystem races.
            check_destination(&destination, force)?;

            write_generated_file(&destination, &file.contents, force)?;

            written.push(destination);
        }

        Ok(written)
    }
}

/// Prevents absolute paths, parent traversal and empty paths.
///
/// This validates the lexical path only. Filesystem symlink
/// resolution is handled separately.
fn validate_relative_path(path: &Path) -> Result<(), GenerationError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Generated file path must be relative and non-empty",
        )
        .into());
    }

    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid generated file path: {}", path.display()),
            )
            .into());
        }
    }

    Ok(())
}

/// Rejects symbolic links and non-directory entries in generated
/// output paths.
///
/// Only the path components below output_dir are checked.
fn validate_parent_directories(
    output_dir: &Path,
    relative_path: &Path,
) -> Result<(), GenerationError> {
    let mut current = output_dir.to_path_buf();

    let Some(parent) = relative_path.parent() else {
        return Ok(());
    };

    for component in parent.components() {
        current.push(component);

        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Output parent is a symbolic link: {}", current.display()),
                    )
                    .into());
                }

                if !metadata.is_dir() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Output parent is not a directory: {}", current.display()),
                    )
                    .into());
                }
            }

            Err(error) if error.kind() == io::ErrorKind::NotFound => {}

            Err(error) => return Err(error.into()),
        }
    }

    Ok(())
}

fn check_destination(destination: &Path, force: bool) -> Result<(), GenerationError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "Refusing to write through symbolic link: {}",
                        destination.display()
                    ),
                )
                .into());
            }

            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Output destination is not a regular file: {}",
                        destination.display()
                    ),
                )
                .into());
            }

            if !force {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "Refusing to overwrite {} (use --force)",
                        destination.display()
                    ),
                )
                .into());
            }
        }

        Err(error) if error.kind() == io::ErrorKind::NotFound => {}

        Err(error) => return Err(error.into()),
    }

    Ok(())
}

/// Installs the contents of one generated file.
///
/// With force=false, hard_link ensures that an existing destination
/// cannot be silently replaced.
///
/// With force=true, rename replaces the destination on supported
/// filesystems.
fn write_generated_file(
    destination: &Path,
    contents: &str,
    force: bool,
) -> Result<(), GenerationError> {
    let file_name = destination
        .file_name()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Generated destination has no file name",
            )
        })?
        .to_string_lossy();

    let (temp_path, mut temp_file) = loop {
        let id = NEXT_WRITE_ID.fetch_add(1, Ordering::Relaxed);

        let candidate = destination.with_file_name(format!(
            ".{file_name}.devinit-{}-{id}.tmp",
            std::process::id()
        ));

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),

            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                continue;
            }

            Err(error) => return Err(error.into()),
        }
    };

    let result = (|| -> io::Result<()> {
        temp_file.write_all(contents.as_bytes())?;
        temp_file.sync_all()?;

        drop(temp_file);

        if force {
            fs::rename(&temp_path, destination)?;
        } else {
            fs::hard_link(&temp_path, destination)?;
        }

        Ok(())
    })();

    // Remove the temporary name on success or failure.
    let _ = fs::remove_file(&temp_path);

    result.map_err(GenerationError::from)
}
#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);

        let path = std::env::temp_dir().join(format!(
            "devinit-generation-plan-{}-{id}",
            std::process::id()
        ));

        fs::create_dir_all(&path).unwrap();

        path
    }

    #[test]
    fn adds_generated_files() {
        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("docker-compose.yaml", "services: {}").unwrap())
            .unwrap();

        assert_eq!(plan.len(), 1);
        assert!(!plan.is_empty());
    }

    #[test]
    fn rejects_duplicate_paths() {
        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("k8s/api.yaml", "first").unwrap())
            .unwrap();

        let result = plan.add(GeneratedFile::new("k8s/api.yaml", "second").unwrap());

        assert!(result.is_err());
        assert_eq!(plan.len(), 1);
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(GeneratedFile::new("../secret.yaml", "data").is_err());

        assert!(GeneratedFile::new("/etc/example.yaml", "data").is_err());
    }

    #[test]
    fn orders_paths_deterministically() {
        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("k8s/z.yaml", "").unwrap())
            .unwrap();

        plan.add(GeneratedFile::new("k8s/a.yaml", "").unwrap())
            .unwrap();

        let paths: Vec<_> = plan
            .files()
            .map(|file| file.relative_path.clone())
            .collect();

        assert_eq!(
            paths,
            vec![PathBuf::from("k8s/a.yaml"), PathBuf::from("k8s/z.yaml"),]
        );
    }

    #[test]
    fn preflight_rejects_existing_file() {
        let dir = test_dir();

        fs::write(dir.join("compose.yaml"), "original").unwrap();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("compose.yaml", "generated").unwrap())
            .unwrap();

        assert!(plan.preflight(&dir, false).is_err());

        assert_eq!(
            fs::read_to_string(dir.join("compose.yaml")).unwrap(),
            "original"
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn preflight_allows_force_without_writing() {
        let dir = test_dir();

        fs::write(dir.join("compose.yaml"), "original").unwrap();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("compose.yaml", "generated").unwrap())
            .unwrap();

        assert!(plan.preflight(&dir, true).is_ok());

        // Preflight must never modify the destination.
        assert_eq!(
            fs::read_to_string(dir.join("compose.yaml")).unwrap(),
            "original"
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writer_creates_multiple_files() {
        let dir = test_dir();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("docker-compose.yaml", "services: {}\n").unwrap())
            .unwrap();

        plan.add(GeneratedFile::new("k8s/backend.yaml", "kind: Deployment\n").unwrap())
            .unwrap();

        let written = plan.write(&dir, false).unwrap();

        assert_eq!(written.len(), 2);

        assert_eq!(
            fs::read_to_string(dir.join("docker-compose.yaml")).unwrap(),
            "services: {}\n"
        );

        assert_eq!(
            fs::read_to_string(dir.join("k8s/backend.yaml")).unwrap(),
            "kind: Deployment\n"
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writer_refuses_existing_files_without_force() {
        let dir = test_dir();

        fs::write(dir.join("existing.yaml"), "original\n").unwrap();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("new.yaml", "new\n").unwrap())
            .unwrap();

        plan.add(GeneratedFile::new("existing.yaml", "modified\n").unwrap())
            .unwrap();

        assert!(plan.write(&dir, false).is_err());

        // Global preflight prevents partial generation on conflicts.
        assert!(!dir.join("new.yaml").exists());

        assert_eq!(
            fs::read_to_string(dir.join("existing.yaml")).unwrap(),
            "original\n"
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writer_overwrites_only_with_force() {
        let dir = test_dir();

        fs::write(dir.join("compose.yaml"), "old\n").unwrap();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("compose.yaml", "new\n").unwrap())
            .unwrap();

        plan.write(&dir, true).unwrap();

        assert_eq!(
            fs::read_to_string(dir.join("compose.yaml")).unwrap(),
            "new\n"
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn writer_rejects_symlink_destination() {
        use std::os::unix::fs::symlink;

        let dir = test_dir();

        let target = dir.join("original.yaml");
        let link = dir.join("linked.yaml");

        fs::write(&target, "original\n").unwrap();
        symlink(&target, &link).unwrap();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("linked.yaml", "modified\n").unwrap())
            .unwrap();

        assert!(plan.write(&dir, true).is_err());

        assert_eq!(fs::read_to_string(&target).unwrap(), "original\n");

        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn writer_rejects_symlink_parent() {
        use std::os::unix::fs::symlink;

        let dir = test_dir();

        let real_dir = dir.join("real");
        let linked_dir = dir.join("k8s");

        fs::create_dir_all(&real_dir).unwrap();
        symlink(&real_dir, &linked_dir).unwrap();

        let mut plan = GenerationPlan::new();

        plan.add(GeneratedFile::new("k8s/backend.yaml", "kind: Deployment\n").unwrap())
            .unwrap();

        assert!(plan.write(&dir, false).is_err());

        assert!(!real_dir.join("backend.yaml").exists());

        fs::remove_dir_all(dir).unwrap();
    }
}

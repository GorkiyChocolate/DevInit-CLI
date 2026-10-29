use crate::errors::GenerationError;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

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
}

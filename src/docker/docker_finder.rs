use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Lists directories that should be skipped during traversal.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".github",
    ".gitlab",
    ".svn",
    ".hg",
    "node_modules",
    "target",
    "vendor",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    "build",
    "dist",
    "bin",
    "out",
    ".cache",
    ".idea",
    ".vscode",
];

/// Lists markers used to identify the project root.
const ROOT_MARKERS: &[&str] = &[
    ".git",
    "Cargo.toml",
    "package.json",
    "go.mod",
    "pom.xml",
    "docker-compose.yml",
    "docker-compose.yaml",
];

/// Checks whether a file name matches Dockerfile or Containerfile conventions.
fn is_dockerfile_name(name: &str) -> bool {
    let lower = name.to_lowercase();

    // Accept exact Dockerfile and Containerfile names.
    if lower == "dockerfile" || lower == "containerfile" {
        return true;
    }

    // Accept files with Dockerfile or Containerfile prefixes.
    if lower.starts_with("dockerfile.") || lower.starts_with("containerfile.") {
        return true;
    }

    // Accept files with Dockerfile or Containerfile suffixes.
    if lower.ends_with(".dockerfile") || lower.ends_with(".containerfile") {
        return true;
    }

    false
}

/// Finds the project root by walking upward from a starting directory.
fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        // Check whether the current directory contains a root marker.
        for marker in ROOT_MARKERS {
            // Stop when a project root marker is found.
            if current.join(marker).exists() {
                return Some(current);
            }
        }

        // Move upward and stop when the filesystem root is reached.
        if !current.pop() {
            break;
        }
    }

    None
}

/// Recursively walks a directory while filtering ignored entries.
fn walk_dir(dir: &Path, ignored: &HashSet<&str>, results: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip symlinks to avoid cycles and traversal outside the project.
        if entry.file_type()?.is_symlink() {
            continue;
        }

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            // Recurse into directories and collect matching files.
            if path.is_dir() {
                // Skip the entire subtree when the directory is ignored.
                if ignored.contains(file_name) {
                    continue;
                }
                walk_dir(&path, ignored, results)?;
            // Collect regular files with Dockerfile-like names.
            } else if path.is_file() && is_dockerfile_name(file_name) {
                results.push(path);
            }
        }
    }

    Ok(())
}

/// Finds the project root and returns all matching Dockerfile paths.
pub fn find_all_dockerfiles() -> io::Result<Vec<PathBuf>> {
    let current_dir = env::current_dir()?;

    // Use the current directory when no project root marker is found.
    let root_dir = find_project_root(&current_dir).unwrap_or(current_dir);

    let ignored_set: HashSet<&str> = IGNORED_DIRS.iter().copied().collect();
    let mut dockerfiles = Vec::new();

    walk_dir(&root_dir, &ignored_set, &mut dockerfiles)?;

    Ok(dockerfiles)
}

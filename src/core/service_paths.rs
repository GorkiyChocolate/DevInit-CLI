use std::env;
use std::io;
use std::path::{Path, PathBuf};

const SERVICES_ENV: &str = "DEVINIT_SERVICES_DIR";

fn not_found(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, message.into())
}

fn validate_directory(path: PathBuf) -> io::Result<PathBuf> {
    if !path.is_dir() {
        return Err(not_found(format!(
            "DevInit service directory not found: {}",
            path.display()
        )));
    }

    path.canonicalize()
}

/// Returns candidate paths relative to the executable.
///
/// Supported layouts:
///
/// Portable:
///   devinit/resources/services/
///
/// Packaged:
///   bin/devinit
///   resources/services/
///
/// Linux system installation:
///   bin/devinit
///   share/devinit/services/
fn executable_candidates(executable: &Path) -> Vec<PathBuf> {
    let Some(bin_dir) = executable.parent() else {
        return Vec::new();
    };

    let mut candidates = vec![bin_dir.join("resources/services")];

    if let Some(prefix) = bin_dir.parent() {
        candidates.push(prefix.join("resources/services"));

        candidates.push(prefix.join("share/devinit/services"));
    }

    candidates
}

/// Locates the installed built-in service definitions.
///
/// DEVINIT_SERVICES_DIR is an explicit override.
/// If provided, it must point to a valid directory.
///
/// The current working directory is deliberately
/// not searched.
pub fn find_services_directory() -> io::Result<PathBuf> {
    if let Some(override_path) = env::var_os(SERVICES_ENV) {
        if override_path.is_empty() {
            return Err(not_found("DEVINIT_SERVICES_DIR cannot be empty"));
        }

        return validate_directory(PathBuf::from(override_path));
    }

    let executable = env::current_exe()?;

    for candidate in executable_candidates(&executable) {
        if candidate.is_dir() {
            return validate_directory(candidate);
        }
    }

    // Development-only fallback.
    // Release packages must ship their service definitions.
    #[cfg(debug_assertions)]
    {
        let development_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/services");

        if development_path.is_dir() {
            return validate_directory(development_path);
        }
    }

    Err(not_found(
        "DevInit built-in service definitions were not found. \
         Verify the installation or set DEVINIT_SERVICES_DIR.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_portable_layout() {
        let executable = Path::new("/opt/devinit/devinit");
        let candidates = executable_candidates(executable);

        assert_eq!(
            candidates[0],
            PathBuf::from("/opt/devinit/resources/services")
        );
    }

    #[test]
    fn supports_packaged_layout() {
        let executable = Path::new("/opt/devinit/bin/devinit");
        let candidates = executable_candidates(executable);

        assert_eq!(
            candidates[1],
            PathBuf::from("/opt/devinit/resources/services")
        );
    }

    #[test]
    fn supports_linux_system_layout() {
        let executable = Path::new("/usr/local/bin/devinit");
        let candidates = executable_candidates(executable);

        assert_eq!(
            candidates[2],
            PathBuf::from("/usr/local/share/devinit/services")
        );
    }

    #[test]
    fn rejects_missing_directory() {
        let path = PathBuf::from("/devinit-nonexistent-test-directory/services");

        assert!(validate_directory(path).is_err());
    }

    #[test]
    fn finds_development_services() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/services");

        assert!(path.is_dir());
    }
}

use super::load_sources;
use crate::errors::DevinitError;

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use reqwest::{Url, redirect::Policy};
use serde_json::Value;
use sha2::{Digest, Sha256};

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_METADATA: u64 = 1024 * 1024;
const MAX_BINARY: u64 = 100 * 1024 * 1024;
const MAX_ARCHIVE: u64 = 128 * 1024 * 1024;

fn error(message: impl Into<String>) -> DevinitError {
    DevinitError::ConfigurationError(message.into())
}

pub fn is_supported_tool(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "kubectl" | "helm"
    )
}

fn platform() -> Result<(&'static str, &'static str), DevinitError> {
    if std::env::consts::OS != "linux" {
        return Err(error("Binary downloads currently support Linux only"));
    }

    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => {
            return Err(error(format!("Unsupported architecture: {other}")));
        }
    };

    Ok(("linux", arch))
}

fn valid_version(version: &str) -> bool {
    let Some(value) = version.strip_prefix('v') else {
        return false;
    };

    let parts: Vec<_> = value.split('.').collect();

    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 8 && part.bytes().all(|c| c.is_ascii_digit())
        })
}

fn allowed_host(host: &str) -> bool {
    matches!(
        host,
        "dl.k8s.io" | "cdn.dl.k8s.io" | "api.github.com" | "get.helm.sh"
    )
}

fn http_client() -> Result<Client, DevinitError> {
    Client::builder()
        .user_agent("DevInit/0.0.4")
        .timeout(Duration::from_secs(120))
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("Too many redirects");
            }

            let url = attempt.url();

            if url.scheme() != "https" || !url.host_str().is_some_and(allowed_host) {
                return attempt.error("Unapproved download redirect");
            }

            attempt.follow()
        }))
        .build()
        .map_err(|e| error(format!("Cannot initialize HTTP client: {e}")))
}

fn download_bytes(client: &Client, address: &str, limit: u64) -> Result<Vec<u8>, DevinitError> {
    let url = Url::parse(address).map_err(|e| error(format!("Invalid URL: {e}")))?;

    if url.scheme() != "https" || !url.host_str().is_some_and(allowed_host) {
        return Err(error(format!("Download source is not approved: {address}")));
    }

    let mut response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|e| error(format!("Download failed: {e}")))?;

    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(error("Download exceeds size limit"));
    }

    let mut bytes = Vec::new();
    let mut buffer = [0u8; 64 * 1024];

    let show_progress = limit > MAX_METADATA;
    let mut next_report = 5 * 1024 * 1024;

    loop {
        let count = response
            .read(&mut buffer)
            .map_err(|e| error(format!("Cannot read downloaded data: {e}")))?;

        if count == 0 {
            break;
        }

        if bytes.len() as u64 + count as u64 > limit {
            return Err(error("Download exceeds size limit"));
        }

        bytes.extend_from_slice(&buffer[..count]);

        if show_progress && bytes.len() >= next_report {
            println!("Downloaded: {} MiB", bytes.len() / (1024 * 1024));

            next_report = bytes.len() + 5 * 1024 * 1024;
        }
    }

    Ok(bytes)
}

fn download_text(client: &Client, url: &str) -> Result<String, DevinitError> {
    let bytes = download_bytes(client, url, MAX_METADATA)?;

    String::from_utf8(bytes).map_err(|_| error("Download contains invalid UTF-8"))
}

fn verify_sha256(contents: &[u8], checksum: &str) -> Result<(), DevinitError> {
    let expected = checksum
        .split_whitespace()
        .next()
        .ok_or_else(|| error("Empty SHA-256 checksum"))?;

    if expected.len() != 64 || !expected.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(error("Invalid SHA-256 checksum"));
    }

    let actual = format!("{:x}", Sha256::digest(contents));

    if !actual.eq_ignore_ascii_case(expected) {
        return Err(error("SHA-256 mismatch: downloaded file rejected"));
    }

    Ok(())
}

fn latest_version(client: &Client, tool: &str, release_url: &str) -> Result<String, DevinitError> {
    let text = download_text(client, release_url)?;

    let version = if tool == "kubectl" {
        text.trim().to_owned()
    } else {
        let response: Value = serde_json::from_str(&text)
            .map_err(|e| error(format!("Invalid Helm release metadata: {e}")))?;

        response
            .get("tag_name")
            .and_then(Value::as_str)
            .ok_or_else(|| error("Helm release metadata contains no tag_name"))?
            .to_owned()
    };

    if !valid_version(&version) {
        return Err(error(format!("Unsupported release version: {version}")));
    }

    Ok(version)
}

fn extract_helm(archive_bytes: &[u8], os: &str, arch: &str) -> Result<Vec<u8>, DevinitError> {
    let decoder = GzDecoder::new(archive_bytes);
    let mut archive = tar::Archive::new(decoder);

    let expected = format!("{os}-{arch}/helm");

    let entries = archive
        .entries()
        .map_err(|e| error(format!("Cannot open Helm archive: {e}")))?;

    for entry in entries {
        let entry = entry.map_err(|e| error(format!("Invalid Helm archive entry: {e}")))?;

        let path = entry
            .path()
            .map_err(|e| error(format!("Invalid archive path: {e}")))?;

        if path.as_ref() != Path::new(&expected) {
            continue;
        }

        if !entry.header().entry_type().is_file() {
            return Err(error("Helm binary is not a regular file"));
        }

        let mut binary = Vec::new();

        entry
            .take(MAX_BINARY + 1)
            .read_to_end(&mut binary)
            .map_err(|e| error(format!("Cannot extract Helm binary: {e}")))?;

        if binary.is_empty() || binary.len() as u64 > MAX_BINARY {
            return Err(error("Invalid Helm binary size"));
        }

        return Ok(binary);
    }

    Err(error("Helm binary not found in archive"))
}

fn tools_directory() -> Result<PathBuf, DevinitError> {
    let home =
        std::env::var_os("HOME").ok_or_else(|| error("HOME environment variable is not set"))?;

    let home = PathBuf::from(home);

    if !home.is_absolute() {
        return Err(error("HOME must be an absolute path"));
    }

    Ok(home.join(".local/share/devinit/tools"))
}

fn save_binary(name: &str, bytes: &[u8]) -> Result<PathBuf, DevinitError> {
    let directory = tools_directory()?;

    fs::create_dir_all(&directory).map_err(DevinitError::FileIOError)?;

    // Reject a symlink as the final tools directory.
    let metadata = fs::symlink_metadata(&directory).map_err(DevinitError::FileIOError)?;

    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(error("Invalid DevInit tools directory"));
    }

    let target = directory.join(name);

    // create_new refuses to replace an existing file.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| {
            error(format!(
                "Cannot create '{}': {e}. \
             Existing tools are not overwritten.",
                target.display()
            ))
        })?;

    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&target, fs::Permissions::from_mode(0o700))?;
        }

        Ok(())
    })();

    drop(file);

    if let Err(e) = result {
        let _ = fs::remove_file(&target);

        return Err(error(format!("Cannot save downloaded binary: {e}")));
    }

    Ok(target)
}

/// Downloads an official CLI tool without installing or running it.
pub fn download(name: &str) -> Result<(PathBuf, String), DevinitError> {
    let name = name.trim().to_ascii_lowercase();

    if !is_supported_tool(&name) {
        return Err(error(format!("Unsupported CLI tool: '{name}'")));
    }

    let sources = load_sources().map_err(DevinitError::FileIOError)?;

    let source = sources
        .cli_tools
        .get(&name)
        .ok_or_else(|| error(format!("No approved source for '{name}' in sources.yaml")))?;

    let (os, arch) = platform()?;
    let client = http_client()?;

    println!("Resolving latest {name} release...");
    // Resolve one version for both the artifact and checksum.
    let version = latest_version(&client, &name, &source.release_url)?;

    println!("✓ Found version: {version}");
    println!("Downloading {name} for {os}/{arch}...");

    let binary = if name == "kubectl" {
        let url = format!("{}/{version}/bin/{os}/{arch}/kubectl", source.download_base);

        let bytes = download_bytes(&client, &url, MAX_BINARY)?;

        let checksum = download_text(&client, &format!("{url}.sha256"))?;

        verify_sha256(&bytes, &checksum)?;

        bytes
    } else {
        let filename = format!("helm-{version}-{os}-{arch}.tar.gz");

        let url = format!("{}/{}", source.download_base, filename);

        let archive = download_bytes(&client, &url, MAX_ARCHIVE)?;

        let checksum = download_text(&client, &format!("{url}.sha256sum"))?;

        verify_sha256(&archive, &checksum)?;

        extract_helm(&archive, os, arch)?
    };
    println!("✓ SHA-256 verified");
    println!("Saving {name}...");

    save_binary(&name, &binary).map(|path| (path, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_supported_tools() {
        assert!(is_supported_tool("kubectl"));
        assert!(is_supported_tool("helm"));
        assert!(is_supported_tool("HELM"));
        assert!(!is_supported_tool("terraform"));
    }

    #[test]
    fn validates_versions() {
        assert!(valid_version("v1.37.0"));
        assert!(valid_version("v4.0.0"));
        assert!(!valid_version("latest"));
        assert!(!valid_version("v1.2.3/evil"));
    }

    #[test]
    fn verifies_correct_sha256() {
        let checksum = format!("{:x}", Sha256::digest(b"test"));

        assert!(verify_sha256(b"test", &checksum).is_ok());
    }

    #[test]
    fn rejects_incorrect_sha256() {
        assert!(verify_sha256(b"test", &"0".repeat(64)).is_err());
    }

    #[test]
    fn rejects_unapproved_domains() {
        assert!(!allowed_host("example.com"));
        assert!(allowed_host("get.helm.sh"));
        assert!(allowed_host("dl.k8s.io"));
    }
}

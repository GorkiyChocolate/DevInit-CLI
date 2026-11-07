use super::load_sources;
use crate::errors::DevinitError;

use flate2::read::GzDecoder;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::{Url, redirect::Policy};
use serde_json::Value;
use sha2::{Digest, Sha256};

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::io::{Seek, SeekFrom};

#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;

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
        .user_agent(concat!("DevInit/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(3600))
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

fn parse_content_range(value: &str, existing: u64, limit: u64) -> Result<u64, DevinitError> {
    let range = value
        .strip_prefix("bytes ")
        .ok_or_else(|| error("Invalid Content-Range"))?;

    let (bounds, total) = range
        .split_once('/')
        .ok_or_else(|| error("Invalid Content-Range"))?;

    let (start, end) = bounds
        .split_once('-')
        .ok_or_else(|| error("Invalid Content-Range"))?;

    let start: u64 = start.parse().map_err(|_| error("Invalid range start"))?;

    let end: u64 = end.parse().map_err(|_| error("Invalid range end"))?;

    let total: u64 = total.parse().map_err(|_| error("Invalid range total"))?;

    if start != existing || end < start || end.checked_add(1) != Some(total) || total > limit {
        return Err(error("Server returned an inconsistent Content-Range"));
    }

    Ok(total - existing)
}

fn download_resumable(
    client: &Client,
    url: &str,
    checksum_url: &str,
    name: &str,
    version: &str,
    platform: (&str, &str),
    limit: u64,
) -> Result<(Vec<u8>, PathBuf), DevinitError> {
    let (os, arch) = platform;
    let checksum = download_text(client, checksum_url)?;

    let expected = checksum
        .split_whitespace()
        .next()
        .ok_or_else(|| error("Empty SHA-256 checksum"))?
        .to_ascii_lowercase();

    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(error("Invalid SHA-256 checksum"));
    }

    let directory = tools_directory()?;

    fs::create_dir_all(&directory).map_err(DevinitError::FileIOError)?;

    // The current MVP rejects a symlink at the tools directory.
    let metadata = fs::symlink_metadata(&directory).map_err(DevinitError::FileIOError)?;

    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(error("Invalid DevInit tools directory"));
    }

    // Include the release and checksum in the partial filename.
    // This prevents mixing different versions or artifacts.
    let part = directory.join(format!(".{name}-{version}-{os}-{arch}-{expected}.part"));

    let (mut file, existing) = open_partial_file(&part, limit)?;

    if existing > limit {
        return Err(error("Partial download exceeds size limit"));
    }

    // First verify a complete cached part, if it already exists.
    if existing > 0 {
        file.seek(SeekFrom::Start(0))
            .map_err(DevinitError::FileIOError)?;

        let mut cached = Vec::new();

        file.read_to_end(&mut cached)
            .map_err(DevinitError::FileIOError)?;

        if verify_sha256(&cached, &expected).is_ok() {
            println!("✓ Existing complete download verified");
            return Ok((cached, part));
        }

        if existing == limit {
            return Err(error(
                "Partial file reached the size limit but its \
                 checksum is invalid",
            ));
        }

        println!("Resuming from {} MiB...", existing / (1024 * 1024));
    }

    let parsed = Url::parse(url).map_err(|e| error(format!("Invalid URL: {e}")))?;

    if parsed.scheme() != "https" || !parsed.host_str().is_some_and(allowed_host) {
        return Err(error("Unapproved download source"));
    }

    let mut request = client.get(parsed);

    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
    }

    let mut response = request
        .send()
        .map_err(|e| error(format!("Download failed: {e}")))?;

    let status = response.status();

    // A resumed request MUST receive 206, never 200.
    if existing > 0 && status != StatusCode::PARTIAL_CONTENT {
        return Err(error(
            "Server did not support resuming this download. \
             The existing partial file was preserved.",
        ));
    }

    if existing == 0 && status != StatusCode::OK {
        return Err(error(format!("Unexpected download HTTP status: {status}")));
    }

    let expected_remaining = if existing > 0 {
        let range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| error("Missing Content-Range"))?;

        Some(parse_content_range(range, existing, limit)?)
    } else {
        if response.content_length().is_some_and(|size| size > limit) {
            return Err(error("Download exceeds size limit"));
        }

        response.content_length()
    };

    if let (Some(expected), Some(actual)) = (expected_remaining, response.content_length())
        && expected != actual
    {
        return Err(error(
            "HTTP Content-Length does not match expected download size",
        ));
    }

    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded = existing;
    let mut received = 0u64;
    let mut next_report = downloaded + 5 * 1024 * 1024;

    loop {
        let count = response.read(&mut buffer).map_err(|e| {
            error(format!(
                "Download interrupted: {e}. \
                 Progress was preserved."
            ))
        })?;

        if count == 0 {
            break;
        }

        let count = count as u64;

        if downloaded + count > limit {
            return Err(error("Download exceeds size limit"));
        }

        file.write_all(&buffer[..count as usize])
            .map_err(DevinitError::FileIOError)?;

        downloaded += count;
        received += count;

        if downloaded >= next_report {
            println!("Downloaded: {} MiB", downloaded / (1024 * 1024));
            next_report = downloaded + 5 * 1024 * 1024;
        }
    }

    file.sync_all().map_err(DevinitError::FileIOError)?;

    if expected_remaining.is_some_and(|size| size != received) {
        return Err(error(
            "Incomplete HTTP response. Partial download preserved.",
        ));
    }

    file.seek(SeekFrom::Start(0))
        .map_err(DevinitError::FileIOError)?;

    let mut bytes = Vec::new();

    file.read_to_end(&mut bytes)
        .map_err(DevinitError::FileIOError)?;

    verify_sha256(&bytes, &expected)?;

    drop(file);

    Ok((bytes, part))
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

#[cfg(target_os = "linux")]
fn open_partial_file(path: &Path, limit: u64) -> Result<(fs::File, u64), DevinitError> {
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| {
            error(format!(
                "Cannot safely open partial download '{}': {e}",
                path.display()
            ))
        })?;

    let metadata = file.metadata().map_err(DevinitError::FileIOError)?;

    if !metadata.is_file() {
        return Err(error("Partial download must be a regular file"));
    }

    if metadata.len() > limit {
        return Err(error("Partial download exceeds size limit"));
    }

    Ok((file, metadata.len()))
}

#[cfg(not(target_os = "linux"))]
fn open_partial_file(_path: &Path, _limit: u64) -> Result<(fs::File, u64), DevinitError> {
    Err(error(
        "Resumable CLI tool downloads are currently supported on Linux only",
    ))
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

    let target = tools_directory()?.join(&name);

    match fs::symlink_metadata(&target) {
        Ok(_) => {
            return Err(error(format!(
                "Tool '{name}' already exists at {}. \
                Existing tools are never overwritten.",
                target.display()
            )));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(DevinitError::FileIOError(e)),
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

    let (binary, part) = if name == "kubectl" {
        let url = format!("{}/{version}/bin/{os}/{arch}/kubectl", source.download_base);

        download_resumable(
            &client,
            &url,
            &format!("{url}.sha256"),
            &name,
            &version,
            (os, arch),
            MAX_BINARY,
        )?
    } else {
        let filename = format!("helm-{version}-{os}-{arch}.tar.gz");
        let url = format!("{}/{}", source.download_base, filename);

        let (archive, part) = download_resumable(
            &client,
            &url,
            &format!("{url}.sha256sum"),
            &name,
            &version,
            (os, arch),
            MAX_ARCHIVE,
        )?;

        (extract_helm(&archive, os, arch)?, part)
    };

    println!("✓ SHA-256 verified");
    println!("Saving {name}...");

    let path = save_binary(&name, &binary)?;

    fs::remove_file(&part).map_err(DevinitError::FileIOError)?;

    Ok((path, version))
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

#[cfg(target_os = "linux")]
#[test]
fn partial_file_rejects_symlink() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!("devinit-part-symlink-{}", std::process::id()));

    fs::create_dir_all(&root).unwrap();

    let target = root.join("target");
    let part = root.join("download.part");

    fs::write(&target, b"do not modify").unwrap();
    symlink(&target, &part).unwrap();

    assert!(open_partial_file(&part, 1024).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"do not modify");

    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn partial_file_preserves_existing_content() {
    let root = std::env::temp_dir().join(format!("devinit-part-existing-{}", std::process::id()));

    fs::create_dir_all(&root).unwrap();

    let part = root.join("download.part");

    fs::write(&part, b"already-downloaded").unwrap();

    let (mut file, size) = open_partial_file(&part, 1024).unwrap();

    assert_eq!(size, 18);

    file.write_all(b"-more").unwrap();
    drop(file);

    assert_eq!(fs::read(&part).unwrap(), b"already-downloaded-more");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn accepts_valid_content_range() {
    assert_eq!(
        parse_content_range("bytes 1024-4095/4096", 1024, 8192).unwrap(),
        3072
    );
}

#[test]
fn rejects_incorrect_range_start() {
    assert!(parse_content_range("bytes 0-4095/4096", 1024, 8192).is_err());
}

#[test]
fn rejects_inconsistent_range_total() {
    assert!(parse_content_range("bytes 1024-2047/4096", 1024, 8192).is_err());
}

#[test]
fn rejects_range_exceeding_limit() {
    assert!(parse_content_range("bytes 1024-8191/8192", 1024, 4096).is_err());
}

#[test]
fn rejects_invalid_content_range() {
    assert!(parse_content_range("invalid", 1024, 8192).is_err());
    assert!(parse_content_range("bytes */4096", 1024, 8192).is_err());
}

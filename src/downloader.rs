use crate::errors::DevinitError;
use crate::service_paths::find_services_directory;
use crate::service_registry;

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

const SOURCES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DownloadSources {
    schema_version: u32,
    docker_images: BTreeMap<String, DockerImageSource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DockerImageSource {
    repository: String,
}

fn invalid_source(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Locate sources.yaml next to the packaged services directory.
fn sources_path() -> io::Result<PathBuf> {
    let services_dir = find_services_directory()?;

    let resources_dir = services_dir
        .parent()
        .ok_or_else(|| invalid_source("Cannot locate the DevInit resources directory"))?;

    Ok(resources_dir.join("sources.yaml"))
}

/// Parse and validate the packaged source registry.
fn parse_sources(contents: &str) -> io::Result<DownloadSources> {
    let sources: DownloadSources = serde_yaml::from_str(contents)
        .map_err(|error| invalid_source(format!("Invalid sources.yaml: {error}")))?;

    if sources.schema_version != SOURCES_SCHEMA_VERSION {
        return Err(invalid_source(format!(
            "Unsupported sources.yaml schema version: {}",
            sources.schema_version
        )));
    }

    if sources.docker_images.is_empty() {
        return Err(invalid_source(
            "sources.yaml contains no Docker image sources",
        ));
    }

    for (name, source) in &sources.docker_images {
        if name.is_empty()
            || !name.bytes().all(|value| {
                value.is_ascii_lowercase()
                    || value.is_ascii_digit()
                    || value == b'-'
                    || value == b'_'
            })
        {
            return Err(invalid_source(format!(
                "Invalid service name in sources.yaml: '{name}'"
            )));
        }

        let Some(repository) = source.repository.strip_prefix("docker.io/") else {
            return Err(invalid_source(format!(
                "Service '{name}' must use an explicit \
                 docker.io repository"
            )));
        };

        if repository.is_empty()
            || repository.split('/').any(|part| {
                part.is_empty()
                    || part == "."
                    || part == ".."
                    || !part.bytes().all(|value| {
                        value.is_ascii_lowercase()
                            || value.is_ascii_digit()
                            || value == b'-'
                            || value == b'_'
                            || value == b'.'
                    })
            })
            || repository.contains(':')
            || repository.contains('@')
        {
            return Err(invalid_source(format!(
                "Invalid repository for '{name}': '{}'",
                source.repository
            )));
        }
    }

    Ok(sources)
}

fn load_sources() -> io::Result<DownloadSources> {
    let path = sources_path()?;
    let contents = fs::read_to_string(&path)?;

    parse_sources(&contents)
}

/// Normalize supported Docker Hub image references.
///
/// Examples:
/// redis       -> library/redis
/// postgres    -> library/postgres
/// apache/kafka -> apache/kafka
fn normalize_repository(repository: &str) -> String {
    let repository = repository.strip_prefix("docker.io/").unwrap_or(repository);

    if repository.contains('/') {
        repository.to_string()
    } else {
        format!("library/{repository}")
    }
}

fn validate_official_image(
    service: &str,
    image: &str,
    sources: &DownloadSources,
) -> Result<(), DevinitError> {
    let source = sources.docker_images.get(service).ok_or_else(|| {
        DevinitError::ConfigurationError(format!(
            "Download is not supported for '{service}'. \
             No approved source is configured in sources.yaml."
        ))
    })?;

    let (repository, tag) = image.rsplit_once(':').ok_or_else(|| {
        DevinitError::ConfigurationError(format!("Service '{service}' has no explicit image tag"))
    })?;

    let valid_tag = !tag.is_empty()
        && tag.len() <= 128
        && tag
            .bytes()
            .next()
            .is_some_and(|value| value.is_ascii_alphanumeric() || value == b'_')
        && tag.bytes().all(|value| {
            value.is_ascii_alphanumeric() || value == b'_' || value == b'-' || value == b'.'
        });

    if !valid_tag
        || image.contains('@')
        || image.contains(char::is_whitespace)
        || normalize_repository(repository) != normalize_repository(&source.repository)
    {
        return Err(DevinitError::ConfigurationError(format!(
            "Service '{service}' uses image '{image}', \
             which does not match its approved source \
             '{}' in sources.yaml.",
            source.repository
        )));
    }

    Ok(())
}

/// Resolve an image from packaged recipes and approved sources.
pub fn resolve_download_image(name: &str) -> Result<String, DevinitError> {
    let name = name.trim().to_ascii_lowercase();

    let sources = load_sources().map_err(DevinitError::FileIOError)?;

    // Unknown services must fail before any Docker operation.
    if !sources.docker_images.contains_key(&name) {
        return Err(DevinitError::ConfigurationError(format!(
            "Download is not supported for '{name}'. \
             No approved source is configured in sources.yaml."
        )));
    }

    let recipe = service_registry::packaged_recipe_version(&name, None)
        .map_err(DevinitError::FileIOError)?
        .ok_or_else(|| DevinitError::ServiceNotFound(name.clone()))?;

    validate_official_image(&name, &recipe.image, &sources)?;

    Ok(recipe.image)
}

/// Download an approved Docker image without starting a container.
pub fn download(name: &str) -> Result<String, DevinitError> {
    let image = resolve_download_image(name)?;

    println!("Downloading approved Docker image: {image}");

    let status = Command::new("docker")
        .args(["image", "pull", &image])
        .status()
        .map_err(|error| {
            DevinitError::ConfigurationError(format!(
                "Unable to start Docker: {error}. \
                 Install or start Docker Engine and try again."
            ))
        })?;

    if !status.success() {
        return Err(DevinitError::ConfigurationError(format!(
            "Failed to download '{image}'. \
             Check Docker Engine, network access, \
             registry authentication and image availability."
        )));
    }

    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sources() -> DownloadSources {
        parse_sources(
            r#"
schema_version: 1
docker_images:
  redis:
    repository: docker.io/library/redis
  kafka:
    repository: docker.io/apache/kafka
"#,
        )
        .unwrap()
    }

    #[test]
    fn parses_official_sources() {
        let sources = test_sources();

        assert_eq!(
            sources.docker_images["redis"].repository,
            "docker.io/library/redis"
        );
    }

    #[test]
    fn rejects_unsupported_schema() {
        let yaml = r#"
schema_version: 99
docker_images:
  redis:
    repository: docker.io/library/redis
"#;

        assert!(parse_sources(yaml).is_err());
    }

    #[test]
    fn rejects_non_docker_hub_sources() {
        let yaml = r#"
schema_version: 1
docker_images:
  redis:
    repository: example.com/untrusted/redis
"#;

        assert!(parse_sources(yaml).is_err());
    }

    #[test]
    fn accepts_matching_official_images() {
        let sources = test_sources();

        assert!(validate_official_image("redis", "redis:7", &sources).is_ok());

        assert!(validate_official_image("kafka", "apache/kafka:4.3.1", &sources).is_ok());
    }

    #[test]
    fn rejects_unapproved_images() {
        let sources = test_sources();

        assert!(validate_official_image("redis", "random/redis:7", &sources).is_err());

        assert!(validate_official_image("redis", "redis:7@sha256:unexpected", &sources).is_err());

        assert!(validate_official_image("redis", "redis:7 extra", &sources).is_err());
    }

    #[test]
    fn packaged_services_match_source_registry() {
        for service in ["redis", "postgresql", "kafka", "prometheus", "grafana"] {
            resolve_download_image(service).unwrap_or_else(|error| {
                panic!(
                    "Invalid download source for \
                         {service}: {error}"
                )
            });
        }
    }

    #[test]
    fn rejects_unknown_services() {
        let sources = test_sources();

        assert!(validate_official_image("unknown", "unknown:1", &sources).is_err());
    }
}

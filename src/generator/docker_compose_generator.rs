use crate::errors::GenerationError;
use crate::file_config::yaml_writer::write_yaml_with_options;
use crate::generation_plan::GeneratedFile;
use crate::models::compile_struct::CompileService;
use crate::models::docker_compose_struct::RecipeCompose;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Generates a Docker Compose file for the requested services.
pub fn generate_docker_compose(
    services: &[CompileService],
    output_dir: &Path,
) -> Result<Vec<PathBuf>, GenerationError> {
    generate_docker_compose_with_options(services, output_dir, false)
}

pub fn generate_docker_compose_with_options(
    services: &[CompileService],
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    let generated = plan_docker_compose(services)?;

    let destination = output_dir.join(&generated.relative_path);

    let written = write_yaml_with_options(
        &serde_yaml::from_str::<serde_yaml::Value>(&generated.contents)?,
        &destination,
        force,
    )?;

    Ok(vec![written])
}

/// Converts Compose services into YAML without filesystem writes.
pub fn render_docker_compose(services: &[CompileService]) -> Result<String, GenerationError> {
    let mut compose_services = BTreeMap::new();

    for service in services {
        let recipe = service.to_recipe();

        compose_services.insert(recipe.name.clone(), ComposeService::from_recipe(&recipe));
    }

    let compose = DockerComposeFile {
        services: compose_services,
        volumes: collect_named_volumes(services),
    };

    Ok(serde_yaml::to_string(&compose)?)
}

/// Prepares a Docker Compose file without writing it.
pub fn plan_docker_compose(services: &[CompileService]) -> Result<GeneratedFile, GenerationError> {
    let yaml = render_docker_compose(services)?;

    GeneratedFile::new("docker-compose.yaml", yaml)
}

#[derive(Debug, Serialize)]
struct DockerComposeFile {
    services: BTreeMap<String, ComposeService>,

    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    volumes: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Serialize)]
/// Represents one serialized Compose service.
struct ComposeService {
    /// Stores the container image.
    image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores published ports.
    ports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores environment entries.
    environment: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores volume mounts.
    volumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores attached networks.
    networks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores service dependencies.
    depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores the restart policy.
    restart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores the container command.
    command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores generated metadata labels.
    labels: Option<BTreeMap<String, String>>,
}

impl ComposeService {
    /// Converts an application recipe into a Compose service.
    fn from_recipe(recipe: &RecipeCompose) -> Self {
        let labels = recipe.description.as_ref().map(|description| {
            BTreeMap::from([("com.devinit.description".to_string(), description.clone())])
        });
        Self {
            image: recipe.image.clone(),
            ports: recipe.ports.clone(),
            environment: recipe.environment.clone(),
            volumes: recipe.volumes.clone(),
            networks: recipe.networks.clone(),
            depends_on: recipe.depends_on.clone(),
            restart: recipe.restart.clone(),
            command: recipe.command.clone(),
            labels,
        }
    }
}

/// Extracts a named volume from a Compose short-syntax mount.
///
/// Examples:
/// - "postgres_data:/var/lib/postgresql/data" -> Some("postgres_data")
/// - "./data:/data" -> None
/// - "/host/data:/data" -> None
/// - "/data" -> None
fn named_volume(mount: &str) -> Option<&str> {
    let mut parts = mount.split(':');

    let source = parts.next()?;
    let target = parts.next()?;

    if source.is_empty() || target.is_empty() {
        return None;
    }

    // Ignore bind mounts and filesystem paths.
    if source.starts_with('/')
        || source.starts_with('.')
        || source.starts_with('~')
        || source.contains('/')
        || source.contains('\\')
    {
        return None;
    }

    // Docker volume names may contain letters, numbers,
    // underscores, periods and hyphens.
    if !source
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
    {
        return None;
    }

    if !source.bytes().next()?.is_ascii_alphanumeric() {
        return None;
    }

    Some(source)
}

/// Collects named volumes referenced by service recipes.
fn collect_named_volumes(
    services: &[CompileService],
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut volumes = BTreeMap::new();

    for service in services {
        let recipe = service.to_recipe();

        if let Some(mounts) = &recipe.volumes {
            for mount in mounts {
                if let Some(name) = named_volume(mount) {
                    volumes
                        .entry(name.to_string())
                        .or_insert_with(BTreeMap::new);
                }
            }
        }
    }

    volumes
}

#[cfg(test)]
#[path = "../tests/docker_compose_generator_tests.rs"]
mod tests;

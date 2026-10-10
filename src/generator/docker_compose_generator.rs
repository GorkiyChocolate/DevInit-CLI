use crate::errors::GenerationError;
use crate::file_config::yaml_writer::write_yaml;
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
    let mut compose_services = BTreeMap::new();
    // Convert every input service into a named Compose service.
    for service in services {
        let recipe = service.to_recipe();
        compose_services.insert(recipe.name.clone(), ComposeService::from_recipe(&recipe));
    }

    let compose = DockerComposeFile {
        services: compose_services,
    };
    let path = write_yaml(&compose, &output_dir.join("docker-compose.yaml"))?;
    Ok(vec![path])
}

#[derive(Debug, Serialize)]
/// Represents the root Docker Compose document.
struct DockerComposeFile {
    /// Maps service names to their Compose definitions.
    services: BTreeMap<String, ComposeService>,
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

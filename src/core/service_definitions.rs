use crate::models::docker_compose_struct::RecipeCompose;

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

pub const SERVICE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinition {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub default_version: String,
    pub versions: BTreeMap<String, ServiceVersion>,
    #[serde(default)]
    pub defaults: ServiceDefaults,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceVersion {
    pub image: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefaults {
    pub ports: Option<Vec<String>>,
    pub environment: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub networks: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub restart: Option<String>,
    pub command: Option<Vec<String>>,
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

impl ServiceDefinition {
    pub fn validate(&self) -> io::Result<()> {
        if self.schema_version != SERVICE_SCHEMA_VERSION {
            return Err(invalid_data(format!(
                "Service '{}': unsupported schema version {}",
                self.name, self.schema_version
            )));
        }

        if !valid_name(&self.name) {
            return Err(invalid_data(
                "Service name must contain only lowercase letters, digits, '-' or '_'",
            ));
        }

        if self.description.trim().is_empty() {
            return Err(invalid_data(format!(
                "Service '{}': description cannot be empty",
                self.name
            )));
        }

        if self.versions.is_empty() {
            return Err(invalid_data(format!(
                "Service '{}': no versions defined",
                self.name
            )));
        }

        if !self.versions.contains_key(&self.default_version) {
            return Err(invalid_data(format!(
                "Service '{}': default version '{}' is not defined",
                self.name, self.default_version
            )));
        }

        for (version, definition) in &self.versions {
            if version.trim().is_empty() || definition.image.trim().is_empty() {
                return Err(invalid_data(format!(
                    "Service '{}': version and image must not be empty",
                    self.name
                )));
            }
        }

        Ok(())
    }

    pub fn supported_versions(&self) -> Vec<&str> {
        self.versions.keys().map(String::as_str).collect()
    }

    pub fn to_recipe(&self, version: Option<&str>) -> io::Result<RecipeCompose> {
        self.validate()?;

        let selected = version.unwrap_or(&self.default_version);

        let definition = self.versions.get(selected).ok_or_else(|| {
            invalid_data(format!(
                "Service '{}': unsupported version '{}'. Supported: {}",
                self.name,
                selected,
                self.versions.keys().cloned().collect::<Vec<_>>().join(", ")
            ))
        })?;

        Ok(RecipeCompose {
            name: self.name.clone(),
            description: Some(self.description.clone()),
            image: definition.image.clone(),
            ports: self.defaults.ports.clone(),
            environment: self.defaults.environment.clone(),
            volumes: self.defaults.volumes.clone(),
            networks: self.defaults.networks.clone(),
            depends_on: self.defaults.depends_on.clone(),
            restart: self.defaults.restart.clone(),
            command: self.defaults.command.clone(),
            files: None,
            env: None,
            notes: None,
        })
    }
}

pub fn load_service_definition(path: &Path) -> io::Result<ServiceDefinition> {
    let contents = fs::read_to_string(path)?;

    let definition: ServiceDefinition = serde_yaml::from_str(&contents)
        .map_err(|error| invalid_data(format!("{}: {error}", path.display())))?;

    definition.validate()?;

    let filename = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_data("Invalid service definition filename"))?;

    if filename != definition.name {
        return Err(invalid_data(format!(
            "{}: filename must match service name '{}'",
            path.display(),
            definition.name
        )));
    }

    Ok(definition)
}

pub fn load_service_registry(directory: &Path) -> io::Result<BTreeMap<String, ServiceDefinition>> {
    let mut registry = BTreeMap::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        // Do not follow symbolic links to service definitions.
        if entry.file_type()?.is_symlink() {
            return Err(invalid_data(format!(
                "Symbolic links are not allowed in the service registry: {}",
                path.display()
            )));
        }

        if !entry.file_type()?.is_file() {
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }

        let definition = load_service_definition(&path)?;
        let name = definition.name.clone();

        if registry.insert(name.clone(), definition).is_some() {
            return Err(invalid_data(format!(
                "Duplicate service definition: {name}"
            )));
        }
    }

    if registry.is_empty() {
        return Err(invalid_data(format!(
            "No service definitions found in {}",
            directory.display()
        )));
    }

    Ok(registry)
}

#[cfg(test)]
#[path = "../tests/service_definitions_tests.rs"]
mod tests;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Defines one Docker Compose service recipe.
pub struct RecipeCompose {
    /// Stores the service name.
    pub name: String,
    /// Optionally describes the service.
    pub description: Option<String>,

    // Docker Compose service settings.
    /// Stores the container image.
    pub image: String,
    /// Maps host ports to container ports.
    pub ports: Option<Vec<String>>,
    /// Defines container environment entries.
    pub environment: Option<Vec<String>>,
    /// Defines mounted volumes.
    pub volumes: Option<Vec<String>>,
    /// Lists networks attached to the service.
    pub networks: Option<Vec<String>>,
    /// Lists services that must start first.
    pub depends_on: Option<Vec<String>>,
    /// Defines the container restart policy.
    pub restart: Option<String>,
    /// Defines the container command and arguments.
    pub command: Option<Vec<String>>,

    /// Optionally defines a file generated with the recipe.
    pub files: Option<File>,

    // Environment file settings.
    /// Stores entries written to an environment file.
    pub env: Option<Vec<String>>,

    /// Stores optional human-readable notes.
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Contains a list of service configurations.
pub struct ConfigsList {
    /// Stores the available service recipes.
    pub configs: Vec<RecipeCompose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a generated file.
pub struct File {
    /// Stores the output file path.
    pub path: String,
    /// Stores the output file contents.
    pub content: String,
}

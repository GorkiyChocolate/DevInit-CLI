use crate::models::{
    cicd_struct::Pipeline, docker_compose_struct::RecipeCompose, k8s_struct::Deployment,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Describes all inputs used by the compiler.
pub struct CompileSpec {
    /// Lists built-in and custom services.
    #[serde(default)]
    pub services: Vec<CompileService>,
    /// Optionally defines a Kubernetes deployment.
    pub kubernetes: Option<Deployment>,
    /// Optionally defines a CI/CD pipeline.
    pub cicd: Option<Pipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
/// Represents a resolved infrastructure service recipe.
pub enum CompileService {
    /// A complete service recipe resolved from YAML or
    /// provided explicitly in compile.yaml.
    Custom(Box<RecipeCompose>),
}

impl CompileService {
    /// Returns the resolved Docker Compose service recipe.
    pub fn to_recipe(&self) -> RecipeCompose {
        match self {
            Self::Custom(recipe) => recipe.as_ref().clone(),
        }
    }
}

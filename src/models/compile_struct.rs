use crate::models::{
    cicd_struct::Pipeline, docker_compose_struct::RecipeCompose, k8s_struct::Deployment,
    services_struct::Services,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Describes all inputs used by the compiler.
pub struct CompileSpec {
    /// Lists built-in and custom services.
    pub services: Vec<CompileService>,
    /// Optionally defines a Kubernetes deployment.
    pub kubernetes: Option<Deployment>,
    /// Optionally defines a CI/CD pipeline.
    pub cicd: Option<Pipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
/// Represents either a built-in or custom service.
pub enum CompileService {
    /// Selects a predefined service recipe.
    BuiltIn(Services),
    /// Provides a user-defined service recipe.
    Custom(Box<RecipeCompose>),
}

impl CompileService {
    /// Converts the service variant into a compose recipe.
    pub fn to_recipe(&self) -> RecipeCompose {
        match self {
            Self::BuiltIn(service) => service.to_recipe(),
            Self::Custom(recipe) => recipe.as_ref().clone(),
        }
    }
}

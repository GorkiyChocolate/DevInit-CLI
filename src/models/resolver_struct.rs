use std::collections::BTreeMap;

use crate::models::{
    cicd_struct::Pipeline, docker_compose_struct::RecipeCompose, k8s_struct::Deployment,
};

/// Holds configuration after service resolution.
pub struct ResolvedSpec {
    /// Maps service names to resolved recipes.
    pub services: BTreeMap<String, RecipeCompose>,
    /// Holds the optional Kubernetes deployment.
    pub kubernetes: Option<Deployment>,
    /// Holds the optional CI/CD pipeline.
    pub cicd: Option<Pipeline>,
}

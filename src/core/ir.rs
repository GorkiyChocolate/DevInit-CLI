use crate::errors::ValidationError;
use crate::models::compile_struct::CompileSpec;
use crate::models::docker_compose_struct::RecipeCompose;

use crate::validator::{compile_validator::validate_compile, dependency_graph::topological_sort};

use std::collections::HashMap;

/// Validated intermediate representation of a DevInit project.
///
/// All service references must be resolved before construction.
#[derive(Debug, Clone)]
pub struct ResolvedCompileSpec {
    spec: CompileSpec,
    services: Vec<RecipeCompose>,
    dependency_order: Vec<String>,
}

impl ResolvedCompileSpec {
    /// Builds an IR from a fully resolved CompileSpec.
    ///
    /// The original specification is preserved.
    /// Services are materialized into concrete recipes.
    /// Dependencies are ordered deterministically.
    pub fn build(spec: CompileSpec) -> Result<Self, Vec<ValidationError>> {
        // Validate the entire specification before creating IR.
        validate_compile(&spec)?;

        // Compute dependency-first ordering.
        let dependency_order = topological_sort(&spec)?;

        // Materialize service recipes.
        let services: Vec<RecipeCompose> = spec
            .services
            .iter()
            .map(|service| service.to_recipe())
            .collect();

        Ok(Self {
            spec,
            services,
            dependency_order,
        })
    }

    /// Returns the original, validated compile specification.
    pub fn spec(&self) -> &CompileSpec {
        &self.spec
    }

    /// Returns concrete recipes after resolution.
    pub fn services(&self) -> &[RecipeCompose] {
        &self.services
    }

    /// Returns service names in dependency-first order.
    pub fn dependency_order(&self) -> &[String] {
        &self.dependency_order
    }

    /// Returns a concrete recipe by its name.
    pub fn service(&self, name: &str) -> Option<&RecipeCompose> {
        self.services.iter().find(|service| service.name == name)
    }

    /// Returns recipes in dependency-first order.
    pub fn ordered_services(&self) -> Vec<&RecipeCompose> {
        let indexes: HashMap<&str, &RecipeCompose> = self
            .services
            .iter()
            .map(|service| (service.name.as_str(), service))
            .collect();

        self.dependency_order
            .iter()
            .filter_map(|name| indexes.get(name.as_str()).copied())
            .collect()
    }

    /// Consumes the IR and returns the validated specification.
    ///
    /// This provides compatibility with existing generators.
    pub fn into_spec(self) -> CompileSpec {
        self.spec
    }
}

#[cfg(test)]
#[path = "../tests/ir_tests.rs"]
mod tests;

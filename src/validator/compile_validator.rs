use crate::errors::ValidationError;
use crate::models::compile_struct::CompileSpec;

/// Validates all compile sections and their cross-references.
pub fn validate_compile(spec: &CompileSpec) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Validate all configured services.
    for service in &spec.services {
        let recipe = service.to_recipe();

        crate::validator::services_validator::validate_service(&recipe.name, &recipe, &mut errors);
    }

    // Validate Kubernetes deployment.
    if let Some(kubernetes) = &spec.kubernetes {
        crate::validator::k8s_validator::validate_deployment(kubernetes, &mut errors);
    }

    // Validate CI/CD pipeline.
    if let Some(cicd) = &spec.cicd {
        crate::validator::cicd_validator::validate_pipeline(cicd, &mut errors);
    }

    // Validate the dependency graph:
    // - duplicate service names
    // - missing dependencies
    // - self-dependencies
    // - duplicate dependencies
    // - circular dependencies
    crate::validator::dependency_graph::validate_dependency_graph(spec, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

use crate::errors::ValidationError;
use crate::models::compile_struct::CompileSpec;
use std::collections::{HashMap, HashSet};

/// Validates all compile sections and their cross-references.
pub fn validate_compile(spec: &CompileSpec) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    // Validate each configured service recipe.
    for service in &spec.services {
        let recipe = service.to_recipe();
        crate::validator::services_validator::validate_service(&recipe.name, &recipe, &mut errors);
    }
    // Validate the optional Kubernetes section.
    if let Some(kubernetes) = &spec.kubernetes {
        crate::validator::k8s_validator::validate_deployment(kubernetes, &mut errors);
    }
    // Validate the optional CI/CD section.
    if let Some(cicd) = &spec.cicd {
        crate::validator::cicd_validator::validate_pipeline(cicd, &mut errors);
    }
    validate_cross_references(spec, &mut errors);
    // Return success only when every validation rule passed.
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates unique service names and dependency references.
fn validate_cross_references(spec: &CompileSpec, errors: &mut Vec<ValidationError>) {
    let mut services = HashMap::new();
    // Index services and detect duplicate names.
    for (index, service) in spec.services.iter().enumerate() {
        let recipe = service.to_recipe();
        // Report duplicate service names.
        if services.insert(recipe.name.clone(), index).is_some() {
            errors.push(ValidationError {
                path: format!("services[{index}].name"),
                message: format!("service '{}' is defined more than once", recipe.name),
            });
        }
    }
    // Check each service dependency list.
    for (index, service) in spec.services.iter().enumerate() {
        let recipe = service.to_recipe();
        // Validate dependencies only when they are present.
        if let Some(dependencies) = &recipe.depends_on {
            // Check every referenced service name.
            for (dependency_index, dependency) in dependencies.iter().enumerate() {
                // Report references to unknown services.
                if !services.contains_key(dependency.as_str()) {
                    errors.push(ValidationError {
                        path: format!("services[{index}].depends_on[{dependency_index}]"),
                        message: format!("service '{dependency}' does not exist"),
                    });
                }
            }
        }
    }
    detect_cycles(spec, &services, errors);
}

/// Detects circular dependencies between services.
fn detect_cycles(
    spec: &CompileSpec,
    services: &HashMap<String, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    // Visit every service as a possible cycle entry point.
    for index in 0..spec.services.len() {
        // Record a validation error when recursion finds a cycle.
        if visit_service(index, spec, services, &mut visiting, &mut visited) {
            errors.push(ValidationError {
                path: format!("services[{index}].depends_on"),
                message: "circular dependency detected".to_string(),
            });
        }
    }
}

/// Recursively visits dependencies and reports whether a cycle exists.
fn visit_service(
    index: usize,
    spec: &CompileSpec,
    services: &HashMap<String, usize>,
    visiting: &mut HashSet<usize>,
    visited: &mut HashSet<usize>,
) -> bool {
    // A currently visiting node closes a dependency cycle.
    if visiting.contains(&index) {
        return true;
    }
    // Skip nodes that were already fully explored.
    if !visited.insert(index) {
        return false;
    }
    visiting.insert(index);
    let recipe = spec.services[index].to_recipe();
    let cycle = recipe.depends_on.as_ref().is_some_and(|dependencies| {
        // Follow only dependencies that resolve to known services.
        dependencies
            .iter()
            .filter_map(|name| services.get(name))
            .any(|dependency| visit_service(*dependency, spec, services, visiting, visited))
    });
    visiting.remove(&index);
    cycle
}

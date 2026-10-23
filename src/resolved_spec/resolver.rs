use std::collections::BTreeMap;

use crate::models::{compile_struct::CompileSpec, resolver_struct::ResolvedSpec};

/// Resolves compile services into a name-indexed specification.
pub fn resolve(spec: CompileSpec) -> ResolvedSpec {
    let mut services = BTreeMap::new();

    // Convert every service variant into a concrete recipe.
    for service in spec.services {
        let recipe = service.to_recipe();

        services.insert(recipe.name.clone(), recipe);
    }

    ResolvedSpec {
        services,
        kubernetes: spec.kubernetes,
        cicd: spec.cicd,
    }
}

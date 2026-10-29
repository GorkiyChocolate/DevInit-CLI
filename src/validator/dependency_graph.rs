use crate::errors::ValidationError;
use crate::models::compile_struct::CompileSpec;
use std::collections::BTreeSet;
use std::collections::{HashMap, HashSet};

pub fn validate_dependency_graph(spec: &CompileSpec, errors: &mut Vec<ValidationError>) {
    let recipes: Vec<_> = spec
        .services
        .iter()
        .map(|service| service.to_recipe())
        .collect();

    let mut indexes = HashMap::new();

    for (index, recipe) in recipes.iter().enumerate() {
        if indexes.contains_key(&recipe.name) {
            errors.push(ValidationError {
                path: format!("services[{index}].name"),
                message: format!("service '{}' is defined more than once", recipe.name),
            });
        } else {
            indexes.insert(recipe.name.clone(), index);
        }
    }

    let mut graph = vec![Vec::<usize>::new(); recipes.len()];

    for (index, recipe) in recipes.iter().enumerate() {
        let mut seen = HashSet::new();

        if let Some(dependencies) = &recipe.depends_on {
            for (position, dependency) in dependencies.iter().enumerate() {
                let path = format!("services[{index}].depends_on[{position}]");

                if !seen.insert(dependency.as_str()) {
                    errors.push(ValidationError {
                        path,
                        message: format!("duplicate dependency '{dependency}'"),
                    });
                    continue;
                }

                if dependency == &recipe.name {
                    errors.push(ValidationError {
                        path,
                        message: format!("service '{}' cannot depend on itself", recipe.name),
                    });
                    continue;
                }

                match indexes.get(dependency) {
                    Some(&target) => graph[index].push(target),
                    None => errors.push(ValidationError {
                        path,
                        message: format!("service '{dependency}' does not exist"),
                    }),
                }
            }
        }
    }

    // 0 = unvisited, 1 = visiting, 2 = completed
    let mut states = vec![0u8; recipes.len()];
    let mut stack = Vec::new();

    for index in 0..recipes.len() {
        if states[index] == 0 {
            detect_cycles(index, &graph, &recipes, &mut states, &mut stack, errors);
        }
    }
}

fn detect_cycles(
    node: usize,
    graph: &[Vec<usize>],
    recipes: &[crate::models::docker_compose_struct::RecipeCompose],
    states: &mut [u8],
    stack: &mut Vec<usize>,
    errors: &mut Vec<ValidationError>,
) {
    states[node] = 1;
    stack.push(node);

    for &next in &graph[node] {
        match states[next] {
            0 => detect_cycles(next, graph, recipes, states, stack, errors),
            1 => {
                if let Some(start) = stack.iter().position(|&n| n == next) {
                    let mut names: Vec<&str> = stack[start..]
                        .iter()
                        .map(|&n| recipes[n].name.as_str())
                        .collect();

                    names.push(recipes[next].name.as_str());

                    errors.push(ValidationError {
                        path: format!("services[{node}].depends_on"),
                        message: format!("circular dependency detected: {}", names.join(" -> ")),
                    });
                }
            }
            _ => {}
        }
    }

    stack.pop();
    states[node] = 2;
}

/// Returns service names in dependency-first order.
///
/// Services without dependencies are ordered alphabetically,
/// ensuring deterministic results.
///
/// Returns validation errors when the dependency graph is invalid.
pub fn topological_sort(spec: &CompileSpec) -> Result<Vec<String>, Vec<ValidationError>> {
    // Reuse the existing graph validator.
    let mut errors = Vec::new();
    validate_dependency_graph(spec, &mut errors);

    if !errors.is_empty() {
        return Err(errors);
    }

    let recipes: Vec<_> = spec
        .services
        .iter()
        .map(|service| service.to_recipe())
        .collect();

    // name -> number of unresolved dependencies
    let mut indegree = HashMap::<String, usize>::new();

    // dependency -> services depending on it
    let mut dependents = HashMap::<String, Vec<String>>::new();

    for recipe in &recipes {
        let dependencies = recipe.depends_on.as_deref().unwrap_or(&[]);

        indegree.insert(recipe.name.clone(), dependencies.len());

        for dependency in dependencies {
            dependents
                .entry(dependency.clone())
                .or_default()
                .push(recipe.name.clone());
        }
    }

    // BTreeSet ensures deterministic alphabetical ordering.
    let mut ready = BTreeSet::new();

    for (name, degree) in &indegree {
        if *degree == 0 {
            ready.insert(name.clone());
        }
    }

    let mut ordered = Vec::with_capacity(recipes.len());

    while let Some(name) = ready.pop_first() {
        ordered.push(name.clone());

        if let Some(children) = dependents.get(&name) {
            for child in children {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree -= 1;

                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
    }

    // Defensive check: validation should have rejected all cycles.
    if ordered.len() != recipes.len() {
        return Err(vec![ValidationError {
            path: "services".to_string(),
            message: "dependency ordering failed: cyclic graph".to_string(),
        }]);
    }

    Ok(ordered)
}
#[cfg(test)]
#[path = "../tests/dependency_graph_tests.rs"]
mod tests;

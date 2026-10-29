use crate::errors::ValidationError;
use crate::models::compile_struct::CompileSpec;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compile_struct::CompileService;
    use crate::models::docker_compose_struct::RecipeCompose;

    fn service(name: &str, deps: &[&str]) -> CompileService {
        CompileService::Custom(Box::new(RecipeCompose {
            name: name.into(),
            description: None,
            image: format!("{name}:latest"),
            ports: None,
            environment: None,
            volumes: None,
            networks: None,
            depends_on: Some(deps.iter().map(|s| s.to_string()).collect()),
            restart: None,
            command: None,
            files: None,
            env: None,
            notes: None,
        }))
    }

    fn check(items: Vec<CompileService>) -> Vec<ValidationError> {
        let spec = CompileSpec {
            services: items,
            kubernetes: None,
            cicd: None,
        };

        let mut errors = Vec::new();
        validate_dependency_graph(&spec, &mut errors);
        errors
    }

    #[test]
    fn accepts_valid_chain() {
        let errors = check(vec![
            service("api", &["worker"]),
            service("worker", &["redis"]),
            service("redis", &[]),
        ]);
        assert!(errors.is_empty());
    }

    #[test]
    fn rejects_self_dependency() {
        let errors = check(vec![service("api", &["api"])]);
        assert!(errors.iter().any(|e| e.message.contains("itself")));
    }

    #[test]
    fn rejects_duplicate_dependency() {
        let errors = check(vec![
            service("api", &["redis", "redis"]),
            service("redis", &[]),
        ]);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("duplicate dependency"))
        );
    }

    #[test]
    fn reports_full_cycle() {
        let errors = check(vec![
            service("api", &["worker"]),
            service("worker", &["redis"]),
            service("redis", &["api"]),
        ]);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("api -> worker -> redis -> api"))
        );
    }

    #[test]
    fn rejects_missing_service() {
        let errors = check(vec![service("api", &["database"])]);
        assert!(errors.iter().any(|e| e.message.contains("does not exist")));
    }

    #[test]
    fn rejects_duplicate_service_names() {
        let errors = check(vec![service("redis", &[]), service("redis", &[])]);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("defined more than once"))
        );
    }
}

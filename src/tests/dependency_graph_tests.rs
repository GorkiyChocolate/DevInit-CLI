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

fn spec(items: Vec<CompileService>) -> CompileSpec {
    CompileSpec {
        services: items,
        kubernetes: None,
        cicd: None,
    }
}

#[test]
fn sorts_dependency_chain() {
    let config = spec(vec![
        service("gateway", &["backend"]),
        service("backend", &["redis"]),
        service("redis", &[]),
    ]);

    let ordered = topological_sort(&config).unwrap();

    assert_eq!(ordered, vec!["redis", "backend", "gateway"]);
}

#[test]
fn sorts_independent_services_deterministically() {
    let config = spec(vec![
        service("redis", &[]),
        service("api", &[]),
        service("postgresql", &[]),
    ]);

    let ordered = topological_sort(&config).unwrap();

    assert_eq!(ordered, vec!["api", "postgresql", "redis"]);
}

#[test]
fn sorts_complex_dependency_graph() {
    let config = spec(vec![
        service("gateway", &["backend"]),
        service("backend", &["postgresql", "redis"]),
        service("redis", &[]),
        service("postgresql", &[]),
    ]);

    let ordered = topological_sort(&config).unwrap();

    assert_eq!(ordered, vec!["postgresql", "redis", "backend", "gateway",]);
}

#[test]
fn rejects_circular_graph_during_sort() {
    let config = spec(vec![
        service("api", &["worker"]),
        service("worker", &["api"]),
    ]);

    let result = topological_sort(&config);

    assert!(result.is_err());

    let errors = result.unwrap_err();

    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("circular dependency"))
    );
}

#[test]
fn rejects_unknown_dependency_during_sort() {
    let config = spec(vec![service("backend", &["unknown-db"])]);

    let result = topological_sort(&config);

    assert!(result.is_err());
}

use devinit_cli_2::cli::cli_logic::compile_at;
use devinit_cli_2::cli::cli_logic::compile_at_with_options;
use devinit_cli_2::cli::cli_logic::{PlannedAction, check_at, dry_run_at};
use devinit_cli_2::errors::DevinitError;
use devinit_cli_2::file_config::file_validator::ensure_file_not_empty;
use devinit_cli_2::generator::cicd_generator::generate_cicd;
use devinit_cli_2::generator::docker_compose_generator::generate_docker_compose;
use devinit_cli_2::generator::generate;
use devinit_cli_2::generator::k8s_generator::generate_k8s;
use devinit_cli_2::generator::k8s_generator::generate_k8s_with_options;
use devinit_cli_2::models::cicd_struct::{CiProvider, Job, Pipeline, Runner, Step, StepAction};
use devinit_cli_2::models::compile_struct::{CompileService, CompileSpec};
use devinit_cli_2::models::docker_compose_struct::RecipeCompose;
use devinit_cli_2::models::k8s_struct::{
    Container, Deployment, DeploymentSpec, LabelSelector, ObjectMeta, PodSpec, PodTemplateSpec,
};
use devinit_cli_2::service_registry::cache_recipe_version;
use devinit_cli_2::service_registry::packaged_recipe_version;
use devinit_cli_2::validator::cicd_validator::validate_pipeline;
use devinit_cli_2::validator::compile_validator::validate_compile;
use devinit_cli_2::validator::k8s_validator::validate_deployment;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::PathBuf;

fn temp_paths(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("devinit-compile-{name}-{}", std::process::id()));
    (root.join("compile.yaml"), root.join("generated"))
}

fn valid_yaml() -> &'static str {
    "services: []\nkubernetes:\n  apiVersion: apps/v1\n  kind: Deployment\n  metadata:\n    name: backend\n  spec:\n    replicas: 1\n    selector:\n      matchLabels: {}\n    template:\n      metadata:\n        name: backend\n      spec:\n        containers:\n          - name: backend\n            image: backend:latest\ncicd: null\n"
}

fn deployment() -> Deployment {
    Deployment {
        api_version: "apps/v1".to_string(),
        kind: "Deployment".to_string(),
        metadata: ObjectMeta {
            name: "backend".to_string(),
            namespace: None,
            labels: None,
            annotations: None,
        },
        spec: DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_labels: HashMap::new(),
            },
            template: PodTemplateSpec {
                metadata: ObjectMeta {
                    name: "backend".to_string(),
                    namespace: None,
                    labels: None,
                    annotations: None,
                },
                spec: PodSpec {
                    containers: vec![Container {
                        name: "backend".to_string(),
                        image: "backend:latest".to_string(),
                        image_pull_policy: None,
                        command: None,
                        args: None,
                        working_dir: None,
                        ports: None,
                        env: None,
                        env_from: None,
                        resources: None,
                        volume_mounts: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                        security_context: None,
                    }],
                    init_containers: None,
                    volumes: None,
                    restart_policy: None,
                    termination_grace_period_seconds: None,
                    service_account_name: None,
                    image_pull_secrets: None,
                    node_selector: None,
                    security_context: None,
                },
            },
            strategy: None,
            min_ready_seconds: None,
            revision_history_limit: None,
            progress_deadline_seconds: None,
            paused: None,
        },
    }
}

fn service(name: &str, image: &str, depends_on: Option<Vec<String>>) -> RecipeCompose {
    RecipeCompose {
        name: name.to_string(),
        description: None,
        image: image.to_string(),
        ports: None,
        environment: None,
        volumes: None,
        networks: None,
        depends_on,
        restart: None,
        command: None,
        files: None,
        env: None,
        notes: None,
    }
}

#[test]
fn valid_config_validates_then_generates() {
    let (input, output) = temp_paths("valid");
    fs::create_dir_all(input.parent().unwrap()).unwrap();
    fs::write(&input, valid_yaml()).unwrap();

    let paths = compile_at(&input, &output).expect("valid compile should succeed");
    assert_eq!(paths, vec![output.join("k8s/backend-deployment.yaml")]);
    assert!(paths[0].exists());
    let _ = fs::remove_dir_all(input.parent().unwrap());
}

#[test]
fn invalid_config_does_not_generate_files() {
    let (input, output) = temp_paths("invalid");
    fs::create_dir_all(input.parent().unwrap()).unwrap();
    fs::write(
        &input,
        "services:\n  - name: backend\n    image: \"\"\nkubernetes: null\ncicd: null\n",
    )
    .unwrap();

    let result = compile_at(&input, &output);
    assert!(matches!(result, Err(DevinitError::ValidationErrors(_))));
    assert!(!output.exists());
    let _ = fs::remove_dir_all(input.parent().unwrap());
}

#[test]
fn malformed_yaml_fails_before_validation_or_generation() {
    let (input, output) = temp_paths("malformed");
    fs::create_dir_all(input.parent().unwrap()).unwrap();
    fs::write(&input, "services: [").unwrap();

    let result = compile_at(&input, &output);
    assert!(matches!(result, Err(DevinitError::YamlParseError(_))));
    assert!(!output.exists());
    let _ = fs::remove_dir_all(input.parent().unwrap());
}

#[test]
fn missing_compile_file_returns_an_error() {
    let (input, output) = temp_paths("missing");
    let result = compile_at(&input, &output);
    assert!(matches!(result, Err(DevinitError::ConfigurationError(_))));
    assert!(!output.exists());
}

#[test]
fn repeated_compile_requires_force_to_overwrite() {
    let (input, output) = temp_paths("repeat");

    let _ = fs::remove_dir_all(input.parent().unwrap());
    fs::create_dir_all(input.parent().unwrap()).unwrap();
    fs::write(&input, valid_yaml()).unwrap();

    // First compile should succeed.
    let first = compile_at(&input, &output).expect("first compile should succeed");

    let generated_file = output.join("k8s/backend-deployment.yaml");
    let original = fs::read(&generated_file).unwrap();

    // Second compile must refuse to overwrite.
    let result = compile_at(&input, &output);

    assert!(
        result.is_err(),
        "compile without --force must reject existing files"
    );

    // Original content must remain unchanged.
    assert_eq!(fs::read(&generated_file).unwrap(), original);

    // Explicit force allows replacement.
    let second = compile_at_with_options(&input, &output, true)
        .expect("compile with --force should succeed");

    assert_eq!(first, second);
    assert_eq!(fs::read_dir(output.join("k8s")).unwrap().count(), 1);

    let _ = fs::remove_dir_all(input.parent().unwrap());
}

#[test]
fn rejects_an_empty_file() {
    let path = std::env::temp_dir().join(format!("devinit-empty-file-{}", std::process::id()));
    fs::write(&path, "  \n").unwrap();

    let result = ensure_file_not_empty(&path);

    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    let _ = fs::remove_file(path);
}

#[test]
fn generates_all_configured_targets() {
    let output =
        std::env::temp_dir().join(format!("devinit-generator-test-{}", std::process::id()));

    let _ = fs::remove_dir_all(&output);

    let pipeline = Pipeline {
        provider: CiProvider::Github,
        triggers: Vec::new(),
        jobs: vec![Job {
            name: "build".to_string(),
            runner: Runner::UbuntuLatest,
            needs: Vec::new(),
            steps: vec![Step {
                name: "Build".to_string(),
                action: StepAction::Run {
                    command: "cargo build".to_string(),
                },
                env: BTreeMap::new(),
            }],
        }],
    };

    let spec = CompileSpec {
        services: vec![CompileService::Custom(Box::new(
            packaged_recipe_version("redis", None).unwrap().unwrap(),
        ))],
        kubernetes: Some(deployment()),
        cicd: Some(pipeline),
    };

    let paths = generate(&spec, &output).expect("generation should succeed");

    assert_eq!(paths.len(), 3);

    assert!(paths.contains(&output.join("docker-compose.yaml")));

    assert!(paths.contains(&output.join("k8s/backend-deployment.yaml")));

    assert!(paths.contains(&output.join(".github/workflows/devinit.yml")));

    let _ = fs::remove_dir_all(output);
}

#[test]
fn generates_github_workflow_path_and_yaml() {
    let output = std::env::temp_dir().join(format!("devinit-cicd-test-{}", std::process::id()));
    let pipeline = Pipeline {
        provider: CiProvider::Github,
        triggers: Vec::new(),
        jobs: vec![Job {
            name: "build".to_string(),
            runner: Runner::UbuntuLatest,
            needs: Vec::new(),
            steps: vec![Step {
                name: "Build".to_string(),
                action: StepAction::Run {
                    command: "cargo build".to_string(),
                },
                env: BTreeMap::new(),
            }],
        }],
    };
    let paths = generate_cicd(&pipeline, &output).expect("generation should succeed");
    assert_eq!(paths, vec![output.join(".github/workflows/devinit.yml")]);
    let contents = fs::read_to_string(&paths[0]).expect("generated file should exist");
    assert!(!contents.contains("provider: github"));
    assert!(contents.contains("jobs:"));
    assert!(contents.contains("runs-on: ubuntu-latest"));
    assert!(contents.contains("steps:"));
    let _ = fs::remove_dir_all(output);
}

#[test]
fn generates_gitlab_pipeline_path_and_structure() {
    let output = std::env::temp_dir().join(format!("devinit-gitlab-test-{}", std::process::id()));
    let pipeline = Pipeline {
        provider: CiProvider::Gitlab,
        triggers: Vec::new(),
        jobs: vec![Job {
            name: "build".to_string(),
            runner: Runner::UbuntuLatest,
            needs: Vec::new(),
            steps: vec![Step {
                name: "Build".to_string(),
                action: StepAction::Run {
                    command: "cargo build".to_string(),
                },
                env: BTreeMap::new(),
            }],
        }],
    };
    let paths = generate_cicd(&pipeline, &output).expect("generation should succeed");
    assert_eq!(paths, vec![output.join(".gitlab-ci.yml")]);
    let contents = fs::read_to_string(&paths[0]).expect("generated file should exist");
    assert!(contents.contains("stages:"));
    assert!(contents.contains("build:"));
    assert!(contents.contains("script:"));
    let _ = fs::remove_dir_all(output);
}

#[test]
fn generates_selected_services_with_descriptions() {
    let output = std::env::temp_dir().join(format!("devinit-compose-test-{}", std::process::id()));
    let services = vec![
        CompileService::Custom(Box::new(
            packaged_recipe_version("postgresql", None)
                .unwrap()
                .unwrap(),
        )),
        CompileService::Custom(Box::new(
            packaged_recipe_version("redis", None).unwrap().unwrap(),
        )),
    ];

    let paths = generate_docker_compose(&services, &output).expect("generation should succeed");
    assert_eq!(paths, vec![output.join("docker-compose.yaml")]);
    let contents = fs::read_to_string(&paths[0]).expect("compose file should exist");
    assert!(contents.contains("postgresql:"));
    assert!(contents.contains("redis:"));
    assert!(contents.contains("com.devinit.description"));
    let _ = fs::remove_dir_all(output);
}

#[test]
fn generates_kubernetes_yaml_and_expected_path() {
    let output = std::env::temp_dir().join(format!("devinit-k8s-test-{}", std::process::id()));

    let _ = fs::remove_dir_all(&output);

    let paths = generate_k8s(&deployment(), &output).expect("first generation should succeed");

    assert_eq!(paths, vec![output.join("k8s/backend-deployment.yaml")]);

    let original = fs::read_to_string(&paths[0]).unwrap();

    assert!(original.contains("apiVersion: apps/v1"));
    assert!(original.contains("kind: Deployment"));

    // Existing file must be protected.
    let result = generate_k8s(&deployment(), &output);

    assert!(
        result.is_err(),
        "generation without force must not overwrite existing YAML"
    );

    assert_eq!(fs::read_to_string(&paths[0]).unwrap(), original);

    // Force explicitly permits replacement.
    let replaced = generate_k8s_with_options(&deployment(), &output, true)
        .expect("generation with force should succeed");

    assert_eq!(paths, replaced);

    let _ = fs::remove_dir_all(output);
}

#[test]
fn reports_missing_job_dependency() {
    let pipeline = Pipeline {
        provider: CiProvider::Github,
        triggers: Vec::new(),
        jobs: vec![Job {
            name: "deploy".to_string(),
            runner: Runner::default(),
            needs: vec!["build".to_string()],
            steps: vec![Step {
                name: "deploy".to_string(),
                action: StepAction::Run {
                    command: "deploy".to_string(),
                },
                env: BTreeMap::new(),
            }],
        }],
    };
    let mut errors = Vec::new();
    validate_pipeline(&pipeline, &mut errors);
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("does not exist"))
    );
}

#[test]
fn reports_empty_image_and_invalid_port() {
    let mut recipe = service("backend", "", None);
    recipe.ports = Some(vec!["8080:0".to_string()]);
    let invalid = CompileService::Custom(Box::new(recipe));
    let result = validate_compile(&CompileSpec {
        services: vec![invalid],
        kubernetes: None,
        cicd: None,
    });
    let errors = result.expect_err("invalid service should fail");
    assert!(
        errors
            .iter()
            .any(|error| error.path == "services.backend.image")
    );
    assert!(
        errors
            .iter()
            .any(|error| error.path == "services.backend.ports[0]")
    );
}

#[test]
fn reports_missing_and_circular_dependencies() {
    let result = validate_compile(&CompileSpec {
        services: vec![
            CompileService::Custom(Box::new(service(
                "a",
                "a:latest",
                Some(vec!["missing".to_string(), "b".to_string()]),
            ))),
            CompileService::Custom(Box::new(service(
                "b",
                "b:latest",
                Some(vec!["a".to_string()]),
            ))),
        ],
        kubernetes: None,
        cicd: None,
    });
    let errors = result.expect_err("invalid dependencies should fail");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("does not exist"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("circular dependency"))
    );
}

#[test]
fn accepts_valid_services() {
    let result = validate_compile(&CompileSpec {
        services: vec![CompileService::Custom(Box::new(service(
            "backend",
            "backend:latest",
            None,
        )))],
        kubernetes: None,
        cicd: None,
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn reports_empty_container_image() {
    let mut deployment = deployment();
    deployment.spec.template.spec.containers[0].image = " ".to_string();
    let mut errors = Vec::new();
    validate_deployment(&deployment, &mut errors);
    assert!(errors.iter().any(|error| error.path.ends_with(".image")));
}

#[test]
fn compiles_unknown_service_from_local_cache() {
    let root = std::env::temp_dir().join(format!("devinit-custom-e2e-{}", std::process::id()));

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let input = root.join("compile.yaml");
    let output = root.join("generated");

    fs::write(
        &input,
        r#"
services:
  - name: caddy
    version: "2.10"
"#,
    )
    .unwrap();

    // Caddy is not part of the installed YAML Registry.
    let recipe = RecipeCompose {
        name: "caddy".to_string(),
        description: Some("Custom HTTP server".to_string()),
        image: "caddy:2.10".to_string(),
        ports: Some(vec!["127.0.0.1:8080:80".to_string()]),
        environment: None,
        volumes: None,
        networks: None,
        depends_on: None,
        restart: Some("unless-stopped".to_string()),
        command: None,
        files: None,
        env: None,
        notes: None,
    };

    // Save using the real cache API, including SHA-256 metadata.
    cache_recipe_version(&root, &recipe, Some("2.10")).unwrap();

    // Check must plan the Compose file without writing it.
    let planned_count = check_at(&input).unwrap();

    assert_eq!(planned_count, 1);
    assert!(!output.join("docker-compose.yaml").exists());

    // Dry-run must report CREATE without generating files.
    let preview = dry_run_at(&input, &output, false).unwrap();

    assert_eq!(preview.len(), 1);
    assert_eq!(preview[0].action, PlannedAction::Create);
    assert!(!output.join("docker-compose.yaml").exists());

    // Compile generates the actual Docker Compose file.
    let generated = compile_at_with_options(&input, &output, false).unwrap();

    let compose_path = output.join("docker-compose.yaml");

    assert_eq!(generated.len(), 1);
    assert!(generated.contains(&compose_path));

    let contents = fs::read_to_string(&compose_path).unwrap();

    assert!(contents.contains("caddy:"));
    assert!(contents.contains("caddy:2.10"));
    assert!(contents.contains("127.0.0.1:8080:80"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn legacy_shorthand_services_still_compile() {
    let (input, output) = temp_paths("legacy-shorthand");
    let root = input.parent().unwrap();

    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root).unwrap();

    fs::write(
        &input,
        r#"
services:
  - postgresql
  - redis
"#,
    )
    .unwrap();

    assert_eq!(check_at(&input).unwrap(), 1);

    let paths =
        compile_at_with_options(&input, &output, false).expect("legacy shorthand must compile");

    assert_eq!(paths.len(), 1);

    let compose = fs::read_to_string(output.join("docker-compose.yaml")).unwrap();

    assert!(compose.contains("postgresql:"));
    assert!(compose.contains("redis:"));
    assert!(compose.contains("postgres:16"));
    assert!(compose.contains("redis:7"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_custom_service_still_compiles() {
    let (input, output) = temp_paths("legacy-custom");
    let root = input.parent().unwrap();

    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root).unwrap();

    fs::write(
        &input,
        r#"
services:
  - name: custom-api
    image: example/custom-api:1.0
    ports:
      - "127.0.0.1:8080:8080"
"#,
    )
    .unwrap();

    assert_eq!(check_at(&input).unwrap(), 1);

    compile_at_with_options(&input, &output, false).unwrap();

    let compose = fs::read_to_string(output.join("docker-compose.yaml")).unwrap();

    assert!(compose.contains("custom-api:"));
    assert!(compose.contains("example/custom-api:1.0"));
    assert!(compose.contains("127.0.0.1:8080:8080"));

    fs::remove_dir_all(root).unwrap();
}

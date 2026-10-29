#[test]
fn compose_preset_overrides_ports() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - name: postgresql
      version: "17"
      ports:
        - "5433:5432"
    - name: redis
      version: "7"
      ports:
        - "6379:6379"
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - use: db
    overrides:
      postgresql:
        ports:
          - "5434:5432"
      redis:
        ports:
          - "6381:6379"
"#,
    )
    .unwrap();

    expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).unwrap();

    assert_eq!(
        document["services"][0]["ports"][0].as_str(),
        Some("5434:5432")
    );

    assert_eq!(
        document["services"][1]["ports"][0].as_str(),
        Some("6381:6379")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compose_preset_preserves_unmodified_fields() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - name: postgresql
      version: "17"
      restart: always
      ports:
        - "5433:5432"
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - use: db
    overrides:
      postgresql:
        ports:
          - "5434:5432"
"#,
    )
    .unwrap();

    expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).unwrap();

    assert_eq!(document["services"][0]["version"].as_str(), Some("17"));

    assert_eq!(document["services"][0]["restart"].as_str(), Some("always"));

    assert_eq!(
        document["services"][0]["ports"][0].as_str(),
        Some("5434:5432")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compose_preset_rejects_unknown_override_service() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - name: redis
      version: "7"
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - use: db
    overrides:
      mongodb:
        ports:
          - "27017:27017"
"#,
    )
    .unwrap();

    assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml"),).is_err());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compose_preset_rejects_invalid_overrides() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - name: redis
      version: "7"
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - use: db
    overrides:
      redis: invalid
"#,
    )
    .unwrap();

    assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml"),).is_err());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compose_preset_rejects_service_renaming() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - name: redis
      version: "7"
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - use: db
    overrides:
      redis:
        name: another-redis
"#,
    )
    .unwrap();

    assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml"),).is_err());

    fs::remove_dir_all(dir).unwrap();
}

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_ID: AtomicU64 = AtomicU64::new(0);

fn test_dir() -> std::path::PathBuf {
    let id = TEST_ID.fetch_add(1, Ordering::Relaxed);

    let dir = std::env::temp_dir().join(format!("devinit-presets-{}-{id}", std::process::id()));

    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn expands_database_preset() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - name: postgresql
      version: "17"
    - name: redis
      version: "7"
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - use: db
  - name: backend
    image: backend:latest
"#,
    )
    .unwrap();

    expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).unwrap();

    let services = document["services"].as_sequence().unwrap();

    assert_eq!(services.len(), 3);
    assert_eq!(services[0]["name"].as_str(), Some("postgresql"));
    assert_eq!(services[1]["name"].as_str(), Some("redis"));
    assert_eq!(services[2]["name"].as_str(), Some("backend"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_unknown_preset() {
    let dir = test_dir();

    fs::write(dir.join("devinit.yaml"), "docker_compose: {}").unwrap();

    let mut document: Value = serde_yaml::from_str("services:\n  - use: missing\n").unwrap();

    assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml"),).is_err());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_nested_presets() {
    let dir = test_dir();

    fs::write(
        dir.join("devinit.yaml"),
        r#"
docker_compose:
  db:
    - use: another
"#,
    )
    .unwrap();

    let mut document: Value = serde_yaml::from_str("services:\n  - use: db\n").unwrap();

    assert!(expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).is_err());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ordinary_services_work_without_preset_file() {
    let dir = test_dir();

    let mut document: Value = serde_yaml::from_str(
        r#"
services:
  - name: redis
    image: redis:7
"#,
    )
    .unwrap();

    expand_compose_presets_from(&mut document, &dir.join("devinit.yaml")).unwrap();

    assert_eq!(document["services"][0]["name"].as_str(), Some("redis"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_invalid_default_url() {
    assert!(validate_url("localhost:3003").is_err());
    assert!(validate_url("http://localhost:3003").is_ok());
    assert!(validate_url("").is_err());
}

#[test]
fn expands_kubernetes_preset() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
backend:
  deployment:
    template: deployment
    name: backend
    image: backend:latest

  service:
    template: service
    name: backend
    target: backend
"#,
    )
    .unwrap();

    let mut section: Mapping = serde_yaml::from_str(
        r#"
use:
  - backend
"#,
    )
    .unwrap();

    expand_k8s_section(&mut section, &configured).unwrap();

    assert!(section.contains_key(key("backend-deployment")));
    assert!(section.contains_key(key("backend-service")));
    assert!(!section.contains_key(key("use")));
}

#[test]
fn rejects_unknown_kubernetes_preset() {
    let configured = Mapping::new();

    let mut section: Mapping = serde_yaml::from_str("use: missing\n").unwrap();

    assert!(expand_k8s_section(&mut section, &configured).is_err());
}

#[test]
fn rejects_duplicate_kubernetes_output() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
backend:
  deployment:
    template: deployment
    name: backend
    image: backend:latest
"#,
    )
    .unwrap();

    let mut section: Mapping = serde_yaml::from_str(
        r#"
use: backend

backend-deployment:
  apiVersion: v1
  kind: ConfigMap
  metadata:
    name: existing
"#,
    )
    .unwrap();

    assert!(expand_k8s_section(&mut section, &configured).is_err());
}

#[test]
fn overrides_kubernetes_deployment_template() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
backend:
  deployment:
    template: deployment
    name: backend
    image: backend:latest
    replicas: 2
    port: 8080
"#,
    )
    .unwrap();

    let mut section: Mapping = serde_yaml::from_str(
        r#"
use:
  - name: backend
    overrides:
      deployment:
        image: backend:v2
        replicas: 5
"#,
    )
    .unwrap();

    expand_k8s_section(&mut section, &configured).unwrap();

    let deployment = section.get(key("backend-deployment")).unwrap();

    assert_eq!(deployment["image"].as_str(), Some("backend:v2"));

    assert_eq!(deployment["replicas"].as_u64(), Some(5));

    assert_eq!(deployment["port"].as_u64(), Some(8080));
}

#[test]
fn deep_merges_raw_kubernetes_manifest() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
backend:
  deployment:
    apiVersion: apps/v1
    kind: Deployment
    metadata:
      name: backend
      labels:
        app: backend
    spec:
      replicas: 2
"#,
    )
    .unwrap();

    let mut section: Mapping = serde_yaml::from_str(
        r#"
use:
  - name: backend
    overrides:
      deployment:
        metadata:
          labels:
            environment: production
        spec:
          replicas: 5
"#,
    )
    .unwrap();

    expand_k8s_section(&mut section, &configured).unwrap();

    let deployment = section.get(key("backend-deployment")).unwrap();

    assert_eq!(
        deployment["metadata"]["labels"]["app"].as_str(),
        Some("backend")
    );

    assert_eq!(
        deployment["metadata"]["labels"]["environment"].as_str(),
        Some("production")
    );

    assert_eq!(deployment["spec"]["replicas"].as_u64(), Some(5));
}

#[test]
fn rejects_unknown_kubernetes_override_manifest() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
backend:
  deployment:
    template: deployment
    name: backend
    image: backend:latest
"#,
    )
    .unwrap();

    let mut section: Mapping = serde_yaml::from_str(
        r#"
use:
  - name: backend
    overrides:
      unknown:
        replicas: 5
"#,
    )
    .unwrap();

    assert!(expand_k8s_section(&mut section, &configured).is_err());
}

#[test]
fn cicd_preset_overrides_jobs() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
rust-tests:
  provider: github
  triggers:
    - type: push
      branches:
        - main
  jobs:
    - name: test
      steps:
        - name: Run tests
          action: run
          command: cargo test
"#,
    )
    .unwrap();

    let reference: Mapping = serde_yaml::from_str(
        r#"
use: rust-tests
overrides:
  jobs:
    - name: test
      steps:
        - name: Release tests
          action: run
          command: cargo test --release
"#,
    )
    .unwrap();

    let resolved = resolve_cicd_preset(&reference, &configured).unwrap();

    assert_eq!(resolved["provider"].as_str(), Some("github"));

    assert_eq!(
        resolved["jobs"][0]["steps"][0]["command"].as_str(),
        Some("cargo test --release")
    );

    assert_eq!(
        resolved["triggers"][0]["branches"][0].as_str(),
        Some("main")
    );
}

#[test]
fn cicd_preset_rejects_unknown_name() {
    let configured = Mapping::new();

    let reference: Mapping = serde_yaml::from_str("use: missing").unwrap();

    assert!(resolve_cicd_preset(&reference, &configured).is_err());
}

#[test]
fn cicd_preset_rejects_invalid_overrides() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
rust-tests:
  provider: github
"#,
    )
    .unwrap();

    let reference: Mapping = serde_yaml::from_str(
        r#"
use: rust-tests
overrides: invalid
"#,
    )
    .unwrap();

    assert!(resolve_cicd_preset(&reference, &configured).is_err());
}

#[test]
fn cicd_preset_rejects_nested_reference() {
    let configured: Mapping = serde_yaml::from_str(
        r#"
rust-tests:
  use: another-preset
"#,
    )
    .unwrap();

    let reference: Mapping = serde_yaml::from_str("use: rust-tests").unwrap();

    assert!(resolve_cicd_preset(&reference, &configured).is_err());
}

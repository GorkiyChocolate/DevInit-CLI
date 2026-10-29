use super::*;

#[test]
fn detects_named_volume() {
    assert_eq!(
        named_volume("postgres_data:/var/lib/postgresql/data"),
        Some("postgres_data")
    );
}

#[test]
fn ignores_relative_bind_mount() {
    assert_eq!(named_volume("./data:/var/lib/postgresql/data"), None);
}

#[test]
fn ignores_absolute_bind_mount() {
    assert_eq!(named_volume("/home/user/data:/data"), None);
}

#[test]
fn ignores_anonymous_volume() {
    assert_eq!(named_volume("/var/lib/postgresql/data"), None);
}

#[test]
fn supports_read_only_named_volume() {
    assert_eq!(named_volume("shared_data:/data:ro"), Some("shared_data"));
}

#[test]
fn rejects_invalid_volume_name() {
    assert_eq!(named_volume("invalid name:/data"), None);
}

#[test]
fn renders_compose_without_writing_files() {
    let services: Vec<CompileService> = serde_yaml::from_str(
        r#"
- name: redis
  image: redis:7
  ports:
    - "6379:6379"
"#,
    )
    .unwrap();

    let yaml = render_docker_compose(&services).unwrap();

    let document: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(
        document["services"]["redis"]["image"].as_str(),
        Some("redis:7")
    );
}

#[test]
fn plans_compose_file() {
    let services: Vec<CompileService> = serde_yaml::from_str(
        r#"
- name: postgres
  image: postgres:17
"#,
    )
    .unwrap();

    let generated = plan_docker_compose(&services).unwrap();

    assert_eq!(
        generated.relative_path,
        std::path::PathBuf::from("docker-compose.yaml")
    );

    assert!(generated.contents.contains("postgres:17"));
}
#[test]
fn rendering_preserves_named_volumes() {
    let services: Vec<CompileService> = serde_yaml::from_str(
        r#"
- name: postgres
  image: postgres:17
  volumes:
    - "postgres_data:/var/lib/postgresql/data"
"#,
    )
    .unwrap();

    let yaml = render_docker_compose(&services).unwrap();

    let document: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();

    assert!(document["volumes"]["postgres_data"].as_mapping().is_some());
}

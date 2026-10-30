use super::*;

fn parse(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap()
}

fn resolve(yaml: &str) -> Result<Value, DevinitError> {
    let mut document = parse(yaml);

    resolve_document(&mut document, Path::new("."))?;

    Ok(document)
}

#[test]
fn resolves_postgresql_version() {
    let document = resolve("services:\n  - name: postgresql\n    version: '17'\n").unwrap();

    assert_eq!(
        document["services"][0]["image"].as_str(),
        Some("postgres:17")
    );
}

#[test]
fn overrides_host_port() {
    let document = resolve(
        "services:\n  - name: postgresql\n    version: '17'\n    ports:\n      - '5433:5432'\n",
    )
    .unwrap();

    assert_eq!(
        document["services"][0]["ports"][0].as_str(),
        Some("5433:5432")
    );
}

#[test]
fn overrides_environment() {
    let document = resolve(
        "services:\n  - name: postgresql\n    environment:\n      - 'POSTGRES_DB=devinit'\n",
    )
    .unwrap();

    assert_eq!(
        document["services"][0]["environment"][0].as_str(),
        Some("POSTGRES_DB=devinit")
    );
}

#[test]
fn preserves_default_image() {
    let document =
        resolve("services:\n  - name: redis\n    version: '7'\n    restart: always\n").unwrap();

    assert_eq!(document["services"][0]["image"].as_str(), Some("redis:7"));

    assert_eq!(document["services"][0]["restart"].as_str(), Some("always"));
}

#[test]
fn rejects_unknown_override() {
    let result = resolve("services:\n  - name: redis\n    unknown_setting: true\n");

    assert!(result.is_err());
}

#[test]
fn rejects_unsupported_version() {
    let result = resolve("services:\n  - name: postgresql\n    version: '999'\n");

    assert!(result.is_err());
}

#[test]
fn preserves_custom_services() {
    let document = resolve(
        "services:\n  - name: api\n    image: my-api:latest\n    ports:\n      - '8080:8080'\n",
    )
    .unwrap();

    assert_eq!(
        document["services"][0]["image"].as_str(),
        Some("my-api:latest")
    );
}

#[test]
fn resolves_shorthand_postgresql_from_yaml() {
    let mut document: serde_yaml::Value = serde_yaml::from_str(
        r#"
services:
  - postgresql
"#,
    )
    .unwrap();

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    resolve_document(&mut document, root).unwrap();

    let service = &document["services"][0];

    assert_eq!(service["name"].as_str(), Some("postgresql"));
    assert_eq!(service["image"].as_str(), Some("postgres:16"));
}

#[test]
fn resolves_shorthand_redis_from_yaml() {
    let mut document: serde_yaml::Value = serde_yaml::from_str(
        r#"
services:
  - redis
"#,
    )
    .unwrap();

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    resolve_document(&mut document, root).unwrap();

    let service = &document["services"][0];

    assert_eq!(service["name"].as_str(), Some("redis"));
    assert_eq!(service["image"].as_str(), Some("redis:7"));
}

#[test]
fn packaged_recipe_takes_priority_over_cached_recipe() {
    let root =
        std::env::temp_dir().join(format!("devinit-registry-priority-{}", std::process::id()));

    let recipe = crate::service_registry::packaged_recipe_version("postgresql", None)
        .unwrap()
        .unwrap();

    let mut cached = recipe.clone();
    cached.image = "postgres:untrusted".to_string();

    crate::service_registry::cache_recipe_version(&root, &cached, None).unwrap();

    let resolved = resolve_recipe(&root, "postgresql", None).unwrap();

    assert_eq!(resolved.image, recipe.image);
    assert_eq!(resolved.image, "postgres:16");

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn unsupported_builtin_version_is_not_loaded_from_cache() {
    let root =
        std::env::temp_dir().join(format!("devinit-registry-version-{}", std::process::id()));

    let mut recipe = crate::service_registry::packaged_recipe_version("postgresql", None)
        .unwrap()
        .unwrap();

    recipe.image = "postgres:99".to_string();

    crate::service_registry::cache_recipe_version(&root, &recipe, Some("99")).unwrap();

    let result = resolve_recipe(&root, "postgresql", Some("99"));

    assert!(result.is_err());

    std::fs::remove_dir_all(&root).unwrap();
}

use crate::service_registry::packaged_recipe_version;

use super::*;
use std::path::PathBuf;

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("services")
}

#[test]
fn loads_builtin_service_definitions() {
    let registry = load_service_registry(&definitions_dir()).unwrap();

    assert!(registry.contains_key("postgresql"));
    assert!(registry.contains_key("redis"));
}

#[test]
fn resolves_default_postgresql_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let postgresql = registry.get("postgresql").unwrap();

    let recipe = postgresql.to_recipe(None).unwrap();

    assert_eq!(recipe.image, "postgres:16");
}

#[test]
fn resolves_explicit_postgresql_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let postgresql = registry.get("postgresql").unwrap();

    let recipe = postgresql.to_recipe(Some("17")).unwrap();

    assert_eq!(recipe.image, "postgres:17");
}

#[test]
fn resolves_explicit_redis_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let redis = registry.get("redis").unwrap();

    let recipe = redis.to_recipe(Some("6")).unwrap();

    assert_eq!(recipe.image, "redis:6");
}

#[test]
fn rejects_unsupported_service_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let postgresql = registry.get("postgresql").unwrap();

    let error = postgresql.to_recipe(Some("99")).unwrap_err();

    assert!(error.to_string().contains("unsupported version"));
}

#[test]
fn packaged_postgresql_uses_yaml_default() {
    let recipe = packaged_recipe_version("postgresql", None)
        .unwrap()
        .unwrap();

    assert_eq!(recipe.name, "postgresql");
    assert_eq!(recipe.image, "postgres:16");
}

#[test]
fn packaged_postgresql_selects_explicit_version() {
    let recipe = packaged_recipe_version("postgresql", Some("17"))
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "postgres:17");
}

#[test]
fn packaged_redis_selects_explicit_version() {
    let recipe = packaged_recipe_version("redis", Some("6"))
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "redis:6");
}

#[test]
fn packaged_registry_rejects_unsupported_version() {
    let error = packaged_recipe_version("postgresql", Some("99")).unwrap_err();

    assert!(error.to_string().contains("unsupported version"));
}

#[test]
fn packaged_registry_returns_none_for_unmigrated_service() {
    let recipe = packaged_recipe_version("mysql", None).unwrap();

    assert!(recipe.is_none());
}

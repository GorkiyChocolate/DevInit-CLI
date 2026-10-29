use super::*;

#[test]
fn builtin_postgresql_works_offline() {
    let recipe = builtin_recipe("postgresql").unwrap();
    assert_eq!(recipe.image, "postgres:16");
}

#[test]
fn supports_explicit_postgresql_version() {
    let recipe = builtin_recipe_version("postgresql", Some("17")).unwrap();

    assert_eq!(recipe.image, "postgres:17");
}

#[test]
fn rejects_unsupported_builtin_version() {
    assert!(builtin_recipe_version("postgresql", Some("999")).is_none());
}

#[test]
fn supports_redis_version() {
    let recipe = builtin_recipe_version("redis", Some("7")).unwrap();

    assert_eq!(recipe.image, "redis:7");
}

#[test]
fn unknown_service_is_not_builtin() {
    assert!(builtin_recipe("unknown-service").is_none());
}

#[test]
fn rejects_unsafe_service_names() {
    assert!(cache_path(Path::new("."), "../postgresql", None).is_err());
    assert!(cache_path(Path::new("."), "postgresql/test", None).is_err());
    assert!(cache_path(Path::new("."), "postgresql", Some("../17")).is_err());
}

#[test]
fn cache_paths_are_versioned() {
    let path = cache_path(Path::new("/tmp/example"), "postgresql", Some("17")).unwrap();

    assert!(path.ends_with("recipes/postgresql/17.yaml"));
}

#[test]
fn preserves_existing_cache() {
    let root = std::env::temp_dir().join(format!("devinit-registry-test-{}", std::process::id()));

    let _ = fs::remove_dir_all(&root);

    let recipe = builtin_recipe_version("postgresql", Some("17")).unwrap();

    let first = cache_recipe_version(&root, &recipe, Some("17")).unwrap();
    let second = cache_recipe_version(&root, &recipe, Some("17")).unwrap();

    assert_eq!(first, second);

    let loaded = cached_recipe_version(&root, "postgresql", Some("17"))
        .unwrap()
        .unwrap();

    assert_eq!(loaded.image, "postgres:17");

    fs::remove_dir_all(root).unwrap();
}

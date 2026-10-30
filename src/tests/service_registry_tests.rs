use super::*;

#[test]
fn packaged_postgresql_works_offline() {
    let recipe = packaged_recipe_version("postgresql", None)
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "postgres:16");
}

#[test]
fn supports_explicit_postgresql_version() {
    let recipe = packaged_recipe_version("postgresql", Some("17"))
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "postgres:17");
}

#[test]
fn rejects_unsupported_packaged_version() {
    assert!(packaged_recipe_version("postgresql", Some("999")).is_err());
}

#[test]
fn supports_redis_version() {
    let recipe = packaged_recipe_version("redis", Some("7"))
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "redis:7");
}

#[test]
fn unknown_service_is_not_packaged() {
    assert!(
        packaged_recipe_version("unknown-service", None)
            .unwrap()
            .is_none()
    );
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

    let recipe = packaged_recipe_version("postgresql", Some("17"))
        .unwrap()
        .unwrap();

    let first = cache_recipe_version(&root, &recipe, Some("17")).unwrap();
    let second = cache_recipe_version(&root, &recipe, Some("17")).unwrap();

    assert_eq!(first, second);

    let loaded = cached_recipe_version(&root, "postgresql", Some("17"))
        .unwrap()
        .unwrap();

    assert_eq!(loaded.image, "postgres:17");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_modified_cached_recipe() {
    let root = std::env::temp_dir().join(format!("devinit-cache-integrity-{}", std::process::id()));

    let _ = fs::remove_dir_all(&root);

    let recipe = packaged_recipe_version("redis", Some("6"))
        .unwrap()
        .unwrap();

    let path = cache_recipe_version(&root, &recipe, Some("6")).unwrap();

    assert!(metadata_path(&path).exists());

    // Confirm that the original cache is valid.
    assert!(
        cached_recipe_version(&root, "redis", Some("6"))
            .unwrap()
            .is_some()
    );

    // Simulate modification after caching.
    let original = fs::read_to_string(&path).unwrap();
    let modified = original.replace("redis:6", "redis:7");

    assert_ne!(original, modified);

    fs::write(&path, modified).unwrap();

    let error = cached_recipe_version(&root, "redis", Some("6")).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("checksum mismatch"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_cache_version_mismatch() {
    let root = std::env::temp_dir().join(format!("devinit-cache-version-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let recipe = packaged_recipe_version("redis", Some("6"))
        .unwrap()
        .unwrap();

    let path = cache_recipe_version(&root, &recipe, Some("6")).unwrap();
    let meta_path = metadata_path(&path);

    let mut metadata: CacheMetadata =
        serde_json::from_slice(&fs::read(&meta_path).unwrap()).unwrap();

    metadata.version = Some("7".to_string());

    fs::write(&meta_path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();

    let error = cached_recipe_version(&root, "redis", Some("6")).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("version mismatch"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_cache_without_metadata() {
    let root =
        std::env::temp_dir().join(format!("devinit-cache-no-metadata-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let recipe = packaged_recipe_version("redis", Some("6"))
        .unwrap()
        .unwrap();

    let path = cache_recipe_version(&root, &recipe, Some("6")).unwrap();

    fs::remove_file(metadata_path(&path)).unwrap();

    let error = cached_recipe_version(&root, "redis", Some("6")).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("Missing cache metadata"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_modified_recipe_with_same_image() {
    let root = std::env::temp_dir().join(format!("devinit-cache-conflict-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let recipe = packaged_recipe_version("redis", Some("6"))
        .unwrap()
        .unwrap();

    cache_recipe_version(&root, &recipe, Some("6")).unwrap();

    let mut modified = recipe.clone();
    modified.ports = Some(vec!["6380:6379".to_string()]);

    let error = cache_recipe_version(&root, &modified, Some("6")).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);

    fs::remove_dir_all(root).unwrap();
}

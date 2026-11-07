use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn test_dir() -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    let root = env::temp_dir().join(format!(
        "devinit-docker-discovery-{}-{id}",
        std::process::id()
    ));

    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn discovers_multiple_dockerfiles_in_project() {
    let root = test_dir();

    fs::create_dir_all(root.join("backend")).unwrap();
    fs::create_dir_all(root.join("frontend")).unwrap();

    fs::write(root.join("backend/Dockerfile"), "").unwrap();
    fs::write(root.join("frontend/Dockerfile.dev"), "").unwrap();

    let found = find_dockerfiles_in(&root).unwrap();

    assert_eq!(found.len(), 2);

    let backend = root.join("backend/Dockerfile").canonicalize().unwrap();
    let frontend = root.join("frontend/Dockerfile.dev").canonicalize().unwrap();

    assert!(found.contains(&backend));
    assert!(found.contains(&frontend));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ignores_dependency_and_build_directories() {
    let root = test_dir();

    fs::create_dir_all(root.join("backend")).unwrap();
    fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    fs::create_dir_all(root.join("target/debug")).unwrap();

    fs::write(root.join("backend/Dockerfile"), "").unwrap();
    fs::write(root.join("node_modules/pkg/Dockerfile"), "").unwrap();
    fs::write(root.join("target/debug/Dockerfile"), "").unwrap();

    let found = find_dockerfiles_in(&root).unwrap();

    let expected = root.join("backend/Dockerfile").canonicalize().unwrap();

    assert_eq!(found, vec![expected]);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn discovery_is_deterministic() {
    let root = test_dir();

    fs::create_dir_all(root.join("z-service")).unwrap();
    fs::create_dir_all(root.join("a-service")).unwrap();

    fs::write(root.join("z-service/Dockerfile"), "").unwrap();
    fs::write(root.join("a-service/Dockerfile"), "").unwrap();

    let first = find_dockerfiles_in(&root).unwrap();
    let second = find_dockerfiles_in(&root).unwrap();

    assert_eq!(first, second);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_missing_project_root() {
    let root = test_dir();
    let missing = root.join("does-not-exist");

    assert!(find_dockerfiles_in(&missing).is_err());

    fs::remove_dir_all(root).unwrap();
}

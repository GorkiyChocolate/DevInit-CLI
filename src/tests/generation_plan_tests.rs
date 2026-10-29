use super::*;

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

fn test_dir() -> PathBuf {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);

    let path = std::env::temp_dir().join(format!(
        "devinit-generation-plan-{}-{id}",
        std::process::id()
    ));

    fs::create_dir_all(&path).unwrap();

    path
}

#[test]
fn adds_generated_files() {
    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("docker-compose.yaml", "services: {}").unwrap())
        .unwrap();

    assert_eq!(plan.len(), 1);
    assert!(!plan.is_empty());
}

#[test]
fn rejects_duplicate_paths() {
    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("k8s/api.yaml", "first").unwrap())
        .unwrap();

    let result = plan.add(GeneratedFile::new("k8s/api.yaml", "second").unwrap());

    assert!(result.is_err());
    assert_eq!(plan.len(), 1);
}

#[test]
fn rejects_path_traversal() {
    assert!(GeneratedFile::new("../secret.yaml", "data").is_err());

    assert!(GeneratedFile::new("/etc/example.yaml", "data").is_err());
}

#[test]
fn orders_paths_deterministically() {
    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("k8s/z.yaml", "").unwrap())
        .unwrap();

    plan.add(GeneratedFile::new("k8s/a.yaml", "").unwrap())
        .unwrap();

    let paths: Vec<_> = plan
        .files()
        .map(|file| file.relative_path.clone())
        .collect();

    assert_eq!(
        paths,
        vec![PathBuf::from("k8s/a.yaml"), PathBuf::from("k8s/z.yaml"),]
    );
}

#[test]
fn preflight_rejects_existing_file() {
    let dir = test_dir();

    fs::write(dir.join("compose.yaml"), "original").unwrap();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("compose.yaml", "generated").unwrap())
        .unwrap();

    assert!(plan.preflight(&dir, false).is_err());

    assert_eq!(
        fs::read_to_string(dir.join("compose.yaml")).unwrap(),
        "original"
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn preflight_allows_force_without_writing() {
    let dir = test_dir();

    fs::write(dir.join("compose.yaml"), "original").unwrap();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("compose.yaml", "generated").unwrap())
        .unwrap();

    assert!(plan.preflight(&dir, true).is_ok());

    // Preflight must never modify the destination.
    assert_eq!(
        fs::read_to_string(dir.join("compose.yaml")).unwrap(),
        "original"
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn writer_creates_multiple_files() {
    let dir = test_dir();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("docker-compose.yaml", "services: {}\n").unwrap())
        .unwrap();

    plan.add(GeneratedFile::new("k8s/backend.yaml", "kind: Deployment\n").unwrap())
        .unwrap();

    let written = plan.write(&dir, false).unwrap();

    assert_eq!(written.len(), 2);

    assert_eq!(
        fs::read_to_string(dir.join("docker-compose.yaml")).unwrap(),
        "services: {}\n"
    );

    assert_eq!(
        fs::read_to_string(dir.join("k8s/backend.yaml")).unwrap(),
        "kind: Deployment\n"
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn writer_refuses_existing_files_without_force() {
    let dir = test_dir();

    fs::write(dir.join("existing.yaml"), "original\n").unwrap();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("new.yaml", "new\n").unwrap())
        .unwrap();

    plan.add(GeneratedFile::new("existing.yaml", "modified\n").unwrap())
        .unwrap();

    assert!(plan.write(&dir, false).is_err());

    // Global preflight prevents partial generation on conflicts.
    assert!(!dir.join("new.yaml").exists());

    assert_eq!(
        fs::read_to_string(dir.join("existing.yaml")).unwrap(),
        "original\n"
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn writer_overwrites_only_with_force() {
    let dir = test_dir();

    fs::write(dir.join("compose.yaml"), "old\n").unwrap();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("compose.yaml", "new\n").unwrap())
        .unwrap();

    plan.write(&dir, true).unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("compose.yaml")).unwrap(),
        "new\n"
    );

    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn writer_rejects_symlink_destination() {
    use std::os::unix::fs::symlink;

    let dir = test_dir();

    let target = dir.join("original.yaml");
    let link = dir.join("linked.yaml");

    fs::write(&target, "original\n").unwrap();
    symlink(&target, &link).unwrap();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("linked.yaml", "modified\n").unwrap())
        .unwrap();

    assert!(plan.write(&dir, true).is_err());

    assert_eq!(fs::read_to_string(&target).unwrap(), "original\n");

    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn writer_rejects_symlink_parent() {
    use std::os::unix::fs::symlink;

    let dir = test_dir();

    let real_dir = dir.join("real");
    let linked_dir = dir.join("k8s");

    fs::create_dir_all(&real_dir).unwrap();
    symlink(&real_dir, &linked_dir).unwrap();

    let mut plan = GenerationPlan::new();

    plan.add(GeneratedFile::new("k8s/backend.yaml", "kind: Deployment\n").unwrap())
        .unwrap();

    assert!(plan.write(&dir, false).is_err());

    assert!(!real_dir.join("backend.yaml").exists());

    fs::remove_dir_all(dir).unwrap();
}

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

fn test_project() -> PathBuf {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);

    let dir = std::env::temp_dir().join(format!(
        "devinit-validation-modes-{}-{id}",
        std::process::id()
    ));

    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("compile.yaml"),
        r#"
services:
  - name: backend
    image: backend:latest
    ports:
      - "8080:80"
"#,
    )
    .unwrap();

    dir
}

#[test]
fn check_does_not_generate_files() {
    let dir = test_project();

    let count = check_at(&dir.join("compile.yaml")).unwrap();

    assert_eq!(count, 1);
    assert!(!dir.join("docker-compose.yaml").exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn dry_run_reports_create_without_writing() {
    let dir = test_project();

    let files = dry_run_at(&dir.join("compile.yaml"), &dir, false).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].action, PlannedAction::Create);
    assert_eq!(files[0].path, PathBuf::from("docker-compose.yaml"));

    assert!(!dir.join("docker-compose.yaml").exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn dry_run_force_preserves_existing_file() {
    let dir = test_project();
    let destination = dir.join("docker-compose.yaml");

    fs::write(&destination, "ORIGINAL CONTENT").unwrap();

    let conflicts = dry_run_at(&dir.join("compile.yaml"), &dir, false).unwrap();

    assert_eq!(conflicts[0].action, PlannedAction::Conflict);

    let overwrites = dry_run_at(&dir.join("compile.yaml"), &dir, true).unwrap();

    assert_eq!(overwrites[0].action, PlannedAction::Overwrite);

    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "ORIGINAL CONTENT"
    );

    fs::remove_dir_all(dir).unwrap();
}

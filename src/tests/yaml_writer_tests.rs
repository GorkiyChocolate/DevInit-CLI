use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("devinit-writer-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn creates_new_yaml() {
    let dir = test_dir();
    let path = dir.join("docker-compose.yaml");
    write_yaml(&vec!["redis"], &path).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("redis"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn refuses_to_replace_user_file() {
    let dir = test_dir();
    let path = dir.join("docker-compose.yaml");
    fs::write(&path, "# hand edited\n").unwrap();
    let error = write_yaml(&vec!["redis"], &path).unwrap_err();
    assert!(matches!(error, GenerationError::Io(e) if e.kind() == io::ErrorKind::AlreadyExists));
    assert_eq!(fs::read_to_string(&path).unwrap(), "# hand edited\n");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn explicit_force_replaces_file() {
    let dir = test_dir();
    let path = dir.join("docker-compose.yaml");
    fs::write(&path, "old\n").unwrap();
    write_yaml_with_options(&vec!["new"], &path, true).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("new"));
    fs::remove_dir_all(dir).unwrap();
}

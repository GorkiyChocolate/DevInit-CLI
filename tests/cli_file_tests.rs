
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "devinit-cli-file-{name}-{}",
        std::process::id()
    ));

    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_config(path: &Path) {
    fs::write(
        path,
        "services:\n  - name: redis\n    image: redis:7\n",
    )
    .unwrap();
}

fn run_cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_devinit"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("DevInit executable should start")
}

#[test]
fn check_uses_default_compile_yaml() {
    let dir = test_dir("default");
    write_config(&dir.join("compile.yaml"));

    let output = run_cli(&dir, &["check"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.join("docker-compose.yaml").exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn custom_file_with_spaces_supports_check_and_dry_run() {
    let dir = test_dir("custom");
    write_config(&dir.join("custom config.yaml"));

    let check = run_cli(&dir, &["check", "-f", "custom config.yaml"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let preview = run_cli(
        &dir,
        &[
            "compile",
            "--file",
            "custom config.yaml",
            "--dry-run",
        ],
    );

    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );

    assert!(
        String::from_utf8_lossy(&preview.stdout)
            .contains("docker-compose.yaml")
    );

    assert!(!dir.join("docker-compose.yaml").exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_custom_file_returns_error() {
    let dir = test_dir("missing");

    let output = run_cli(&dir, &["check", "-f", "missing.yaml"]);

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing.yaml"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn custom_file_compile_preserves_existing_output() {
    let dir = test_dir("overwrite");
    write_config(&dir.join("custom.yaml"));

    let first = run_cli(&dir, &["compile", "-f", "custom.yaml"]);
    assert!(first.status.success());

    let generated = dir.join("docker-compose.yaml");
    let original = fs::read(&generated).unwrap();

    let second = run_cli(&dir, &["compile", "-f", "custom.yaml"]);

    assert!(!second.status.success());
    assert_eq!(fs::read(&generated).unwrap(), original);

    fs::remove_dir_all(dir).unwrap();
}

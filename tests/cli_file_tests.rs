use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("devinit-cli-file-{name}-{}", std::process::id()));

    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_config(path: &Path) {
    fs::write(path, "services:\n  - name: redis\n    image: redis:7\n").unwrap();
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
        &["compile", "--file", "custom config.yaml", "--dry-run"],
    );

    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );

    assert!(String::from_utf8_lossy(&preview.stdout).contains("docker-compose.yaml"));

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

#[test]
fn check_json_returns_valid_document() {
    let dir = test_dir("check-json");
    write_config(&dir.join("compile.yaml"));

    let output = run_cli(&dir, &["check", "--json"]);
    assert!(output.status.success());

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(value["status"], "success");
    assert_eq!(value["command"], "check");
    assert_eq!(value["valid"], true);
    assert_eq!(value["files_written"], 0);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn check_json_reports_missing_file() {
    let dir = test_dir("check-json-error");

    let output = run_cli(&dir, &["check", "--file", "missing.yaml", "--json"]);

    assert!(!output.status.success());

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(value["status"], "error");
    assert_eq!(value["valid"], false);
    assert!(
        value["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("missing.yaml")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn dry_run_json_reports_planned_files() {
    let dir = test_dir("dry-run-json");
    write_config(&dir.join("compile.yaml"));

    let output = run_cli(&dir, &["compile", "--dry-run", "--json"]);

    assert!(output.status.success());

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(value["status"], "success");
    assert_eq!(value["mode"], "dry-run");
    assert_eq!(value["summary"]["create"], 1);
    assert_eq!(value["files"][0]["action"], "create");

    assert!(!dir.join("docker-compose.yaml").exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn list_json_returns_services() {
    let dir = test_dir("list-json");

    let output = run_cli(&dir, &["list", "--json"]);
    assert!(output.status.success());

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(value["status"], "success");
    assert_eq!(value["command"], "list");
    assert_eq!(value["total"], 12);
    assert_eq!(value["services"].as_array().unwrap().len(), 12);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn check_accepts_k8s_section_with_json_output() {
    let dir = test_dir("k8s-json-check");

    let config = r#"
services:
  - redis

k8s:
  redis-deployment:
    template: deployment
    name: redis-test
    image: redis:7
    port: 6379
"#;

    fs::write(dir.join("compile.yaml"), config).unwrap();

    let output = run_cli(&dir, &["check", "--json"]);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(result["status"], "success");
    assert_eq!(result["valid"], true);

    // Docker Compose + Kubernetes Deployment.
    assert_eq!(result["planned_files"], 2);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn check_rejects_invalid_k8s_section() {
    let dir = test_dir("k8s-invalid");

    let config = r#"
services:
  - redis

k8s:
  invalid-deployment:
    template: deployment
    name: redis-test
    image: redis:7
    port: 70000
"#;

    fs::write(dir.join("compile.yaml"), config).unwrap();

    let output = run_cli(&dir, &["check", "--json"]);

    assert!(
        !output.status.success(),
        "Invalid Kubernetes configuration was accepted"
    );

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(result["status"], "error");
    assert_eq!(result["valid"], false);
    assert!(!result["errors"].as_array().unwrap().is_empty());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compile_json_reports_created_file() {
    let dir = test_dir("compile-json-create");
    write_config(&dir.join("compile.yaml"));

    let output = run_cli(&dir, &["compile", "--json"]);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(result["status"], "success");
    assert_eq!(result["command"], "compile");
    assert_eq!(result["mode"], "write");
    assert_eq!(result["valid"], true);

    assert_eq!(result["created"][0], "docker-compose.yaml");
    assert_eq!(result["overwritten"].as_array().unwrap().len(), 0);

    assert_eq!(result["files"][0]["path"], "docker-compose.yaml");
    assert_eq!(result["files"][0]["action"], "create");

    assert_eq!(result["summary"]["create"], 1);
    assert_eq!(result["summary"]["overwrite"], 0);
    assert_eq!(result["summary"]["files_written"], 1);

    assert!(dir.join("docker-compose.yaml").is_file());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compile_json_reports_overwritten_file() {
    let dir = test_dir("compile-json-force");
    write_config(&dir.join("compile.yaml"));

    let generated = dir.join("docker-compose.yaml");
    fs::write(&generated, "old configuration").unwrap();

    let output = run_cli(&dir, &["compile", "--json", "--force"]);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(result["status"], "success");
    assert_eq!(result["mode"], "write");

    assert_eq!(result["created"].as_array().unwrap().len(), 0);

    assert_eq!(result["overwritten"][0], "docker-compose.yaml");

    assert_eq!(result["files"][0]["action"], "overwrite");

    assert_eq!(result["summary"]["create"], 0);
    assert_eq!(result["summary"]["overwrite"], 1);
    assert_eq!(result["summary"]["files_written"], 1);

    let contents = fs::read_to_string(&generated).unwrap();

    assert_ne!(contents, "old configuration");
    assert!(contents.contains("redis"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn compile_json_refuses_overwrite_without_force() {
    let dir = test_dir("compile-json-conflict");
    write_config(&dir.join("compile.yaml"));

    let generated = dir.join("docker-compose.yaml");
    let original = "user-owned configuration";

    fs::write(&generated, original).unwrap();

    let output = run_cli(&dir, &["compile", "--json"]);

    assert!(!output.status.success());

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(result["status"], "error");
    assert_eq!(result["command"], "compile");
    assert_eq!(result["valid"], false);

    assert!(!result["errors"].as_array().unwrap().is_empty());

    assert_eq!(fs::read_to_string(&generated).unwrap(), original);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn list_json_contains_supported_versions() {
    let dir = test_dir("list-json-versions");

    let output = run_cli(&dir, &["list", "--json"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let services = result["services"]
        .as_array()
        .expect("services must be an array");

    assert!(!services.is_empty());

    for service in services {
        let name = service["name"].as_str().unwrap();

        let default_version = service["default_version"].as_str().unwrap();

        let versions = service["versions"]
            .as_array()
            .expect("versions must be an array");

        assert!(!versions.is_empty(), "{name} has no supported versions");

        assert!(
            versions
                .iter()
                .any(|version| { version.as_str() == Some(default_version) }),
            "{name}: default version {default_version} \
             is not in supported versions"
        );
    }

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn list_json_reports_known_service_versions() {
    let dir = test_dir("list-json-known-versions");

    let output = run_cli(&dir, &["list", "--json"]);

    assert!(output.status.success());

    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let services = result["services"].as_array().unwrap();

    let find_service = |name: &str| {
        services
            .iter()
            .find(|service| service["name"] == name)
            .unwrap_or_else(|| panic!("Missing service: {name}"))
    };

    let postgres = find_service("postgresql");
    let redis = find_service("redis");

    assert!(
        postgres["versions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("17"))
    );

    let redis_versions = redis["versions"].as_array().unwrap();

    assert!(redis_versions.contains(&serde_json::json!("6")));
    assert!(redis_versions.contains(&serde_json::json!("7")));

    fs::remove_dir_all(dir).unwrap();
}

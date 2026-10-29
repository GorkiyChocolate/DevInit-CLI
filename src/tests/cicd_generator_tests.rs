use super::*;

fn pipeline(yaml: &str) -> Pipeline {
    serde_yaml::from_str(yaml).unwrap()
}

#[test]
fn plans_github_actions() {
    let config = pipeline(
        r#"
provider: github
triggers:
  - type: push
    branches:
      - main

jobs:
  - name: build
    runner: ubuntu-latest
    steps:
      - name: Checkout
        action: checkout
      - name: Build
        action: run
        command: cargo build
"#,
    );

    let file = plan_cicd(&config).unwrap();

    assert_eq!(
        file.relative_path,
        PathBuf::from(".github/workflows/devinit.yml")
    );

    let document: serde_yaml::Value = serde_yaml::from_str(&file.contents).unwrap();

    assert_eq!(document["name"].as_str(), Some("DevInit"));

    assert_eq!(
        document["jobs"]["build"]["runs-on"].as_str(),
        Some("ubuntu-latest")
    );

    assert_eq!(
        document["jobs"]["build"]["steps"][1]["run"].as_str(),
        Some("cargo build")
    );
}

#[test]
fn plans_gitlab_pipeline() {
    let config = pipeline(
        r#"
provider: gitlab
jobs:
  - name: build
    steps:
      - name: Build application
        action: run
        command: cargo build
"#,
    );

    let file = plan_cicd(&config).unwrap();

    assert_eq!(file.relative_path, PathBuf::from(".gitlab-ci.yml"));

    let document: serde_yaml::Value = serde_yaml::from_str(&file.contents).unwrap();

    assert_eq!(document["stages"][0].as_str(), Some("build"));

    assert_eq!(document["build"]["script"][0].as_str(), Some("cargo build"));
}

#[test]
fn rendering_matches_existing_github_serializer() {
    let config = pipeline(
        r#"
provider: github
triggers:
  - type: manual
jobs:
  - name: test
    steps:
      - name: Run tests
        action: run
        command: cargo test
"#,
    );

    let rendered = render_cicd(&config).unwrap();

    let expected = serde_yaml::to_string(&GithubWorkflow::from_pipeline(&config)).unwrap();

    assert_eq!(rendered, expected);
}

#[test]
fn rendering_matches_existing_gitlab_serializer() {
    let config = pipeline(
        r#"
provider: gitlab
jobs:
  - name: test
    steps:
      - name: Run tests
        action: run
        command: cargo test
"#,
    );

    let rendered = render_cicd(&config).unwrap();

    let expected = serde_yaml::to_string(&GitlabPipeline::from_pipeline(&config)).unwrap();

    assert_eq!(rendered, expected);
}

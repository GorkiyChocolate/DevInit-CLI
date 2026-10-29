use super::*;

fn validate(yaml: &str) -> Vec<ValidationError> {
    let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
    let mut errors = Vec::new();

    validate_pipeline(&pipeline, &mut errors);

    errors
}

#[test]
fn rejects_circular_job_dependencies() {
    let errors = validate(
        r#"
provider: github
jobs:
  - name: build
    needs: [test]
  - name: test
    needs: [build]
"#,
    );

    assert!(
        errors
            .iter()
            .any(|error| { error.message.contains("circular job dependency") })
    );
}

#[test]
fn rejects_self_referencing_job() {
    let errors = validate(
        r#"
provider: github
jobs:
  - name: build
    needs: [build]
"#,
    );

    assert!(
        errors
            .iter()
            .any(|error| { error.message.contains("build -> build") })
    );
}

#[test]
fn accepts_acyclic_job_dependencies() {
    let errors = validate(
        r#"
provider: github
jobs:
  - name: build
  - name: test
    needs: [build]
  - name: publish
    needs: [test]
"#,
    );

    assert!(
        !errors
            .iter()
            .any(|error| { error.message.contains("circular job dependency") })
    );
}

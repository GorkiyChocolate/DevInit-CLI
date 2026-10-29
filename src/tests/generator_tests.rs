use super::*;

fn spec(yaml: &str) -> CompileSpec {
    serde_yaml::from_str(yaml).unwrap()
}

#[test]
fn builds_unified_plan_for_compose_and_kubernetes() {
    let config = spec(
        r#"
services:
  - name: redis
    image: redis:7
"#,
    );

    let document: serde_yaml::Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
data:
  ENV: production
"#,
    )
    .unwrap();

    let manifests = vec![KubernetesManifest {
        file_name: "app-config".into(),
        document,
    }];

    let plan = build_generation_plan(&config, &manifests).unwrap();

    assert_eq!(plan.len(), 2);

    let paths: Vec<_> = plan
        .files()
        .map(|file| file.relative_path.clone())
        .collect();

    assert!(paths.contains(&PathBuf::from("docker-compose.yaml")));

    assert!(paths.contains(&PathBuf::from("k8s/app-config.yaml")));
}

#[test]
fn unified_plan_rejects_duplicate_paths() {
    let config = spec("services: []");

    let document: serde_yaml::Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: example
"#,
    )
    .unwrap();

    let manifests = vec![
        KubernetesManifest {
            file_name: "example".into(),
            document: document.clone(),
        },
        KubernetesManifest {
            file_name: "example".into(),
            document,
        },
    ];

    assert!(build_generation_plan(&config, &manifests).is_err());
}

#[test]
fn unified_plan_preserves_compose_content() {
    let config = spec(
        r#"
services:
  - name: redis
    image: redis:7
"#,
    );

    let plan = build_generation_plan(&config, &[]).unwrap();

    let file = plan.files().next().unwrap();

    let yaml: serde_yaml::Value = serde_yaml::from_str(&file.contents).unwrap();

    assert_eq!(yaml["services"]["redis"]["image"].as_str(), Some("redis:7"));
}

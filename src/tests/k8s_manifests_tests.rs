use super::*;

fn temp_file(yaml: &str, label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "devinit-manifest-{}-{label}.yaml",
        std::process::id()
    ));

    fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn accepts_multiple_manifests() {
    let path = temp_file(
        r#"
k8s:
  backend:
    apiVersion: apps/v1
    kind: Deployment
    metadata:
      name: backend
    spec:
      replicas: 2
      selector:
        matchLabels:
          app: backend
      template:
        metadata:
          labels:
            app: backend
        spec:
          containers:
            - name: backend
              image: backend:latest
              ports:
                - containerPort: 8080

  networking:
    apiVersion: v1
    kind: Service
    metadata:
      name: backend
    spec:
      selector:
        app: backend
      ports:
        - port: 80
          targetPort: 8080
"#,
        "multiple",
    );

    let manifests = load_manifests(&path).unwrap();

    assert_eq!(manifests.len(), 2);
    assert_eq!(manifests[0].file_name, "backend");
    assert_eq!(manifests[1].file_name, "networking");

    fs::remove_file(path).unwrap();
}

#[test]
fn rejects_path_traversal() {
    let path = temp_file(
        r#"
k8s:
  ../escape:
    apiVersion: v1
    kind: ConfigMap
    metadata:
      name: example
"#,
        "traversal",
    );

    assert!(load_manifests(&path).is_err());

    fs::remove_file(path).unwrap();
}

#[test]
fn rejects_missing_resource_kind() {
    let path = temp_file(
        r#"
k8s:
  broken:
    apiVersion: v1
    metadata:
      name: broken
"#,
        "missing-kind",
    );

    assert!(load_manifests(&path).is_err());

    fs::remove_file(path).unwrap();
}

#[test]
fn rejects_duplicate_resource_identity() {
    let path = temp_file(
        r#"
k8s:
  first:
    apiVersion: v1
    kind: ConfigMap
    metadata:
      name: settings

  second:
    apiVersion: v1
    kind: ConfigMap
    metadata:
      name: settings
"#,
        "duplicate",
    );

    assert!(load_manifests(&path).is_err());

    fs::remove_file(path).unwrap();
}

#[test]
fn expands_deployment_template() {
    let resource: Value = serde_yaml::from_str(
        r#"
template: deployment
name: backend
image: backend:1.0
replicas: 3
port: 8080
"#,
    )
    .unwrap();

    let result = expand_template(&resource).unwrap();

    assert_eq!(result["kind"].as_str(), Some("Deployment"));
    assert_eq!(result["metadata"]["name"].as_str(), Some("backend"));
    assert_eq!(result["spec"]["replicas"].as_u64(), Some(3));
    assert_eq!(
        result["spec"]["template"]["spec"]["containers"][0]["image"].as_str(),
        Some("backend:1.0")
    );
}

#[test]
fn expands_service_template() {
    let resource: Value = serde_yaml::from_str(
        r#"
template: service
name: backend
target: backend
port: 80
targetPort: 8080
"#,
    )
    .unwrap();

    let result = expand_template(&resource).unwrap();

    assert_eq!(result["kind"].as_str(), Some("Service"));
    assert_eq!(result["spec"]["selector"]["app"].as_str(), Some("backend"));
    assert_eq!(
        result["spec"]["ports"][0]["targetPort"].as_u64(),
        Some(8080)
    );
}

#[test]
fn preserves_raw_manifest() {
    let resource: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
data:
  ENV: production
"#,
    )
    .unwrap();

    let result = expand_template(&resource).unwrap();

    assert_eq!(result, resource);
}

#[test]
fn rejects_unknown_template() {
    let resource: Value = serde_yaml::from_str(
        r#"
template: unknown
name: example
"#,
    )
    .unwrap();

    assert!(expand_template(&resource).is_err());
}

#[test]
fn rejects_invalid_port() {
    let resource: Value = serde_yaml::from_str(
        r#"
template: deployment
name: backend
image: backend:latest
port: 99999
"#,
    )
    .unwrap();

    assert!(expand_template(&resource).is_err());
}

#[test]
fn rejects_unknown_template_field() {
    let resource: Value = serde_yaml::from_str(
        r#"
template: deployment
name: backend
image: backend:latest
unknownField: true
"#,
    )
    .unwrap();

    assert!(expand_template(&resource).is_err());
}

#[test]
fn plans_custom_manifest_without_writing() {
    let document: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
data:
  APP_ENV: production
"#,
    )
    .unwrap();

    let manifests = vec![KubernetesManifest {
        file_name: "app-config".into(),
        document,
    }];

    let files = plan_custom_manifests(&manifests).unwrap();

    assert_eq!(files.len(), 1);

    assert_eq!(files[0].relative_path, PathBuf::from("k8s/app-config.yaml"));

    let parsed: Value = serde_yaml::from_str(&files[0].contents).unwrap();

    assert_eq!(parsed["kind"].as_str(), Some("ConfigMap"));
}

#[test]
fn plans_multiple_custom_manifests_in_sorted_order() {
    let first: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: z-config
"#,
    )
    .unwrap();

    let second: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: a-config
"#,
    )
    .unwrap();

    let manifests = vec![
        KubernetesManifest {
            file_name: "z-config".into(),
            document: first,
        },
        KubernetesManifest {
            file_name: "a-config".into(),
            document: second,
        },
    ];

    let files = plan_custom_manifests(&manifests).unwrap();

    assert_eq!(
        files
            .iter()
            .map(|file| file.relative_path.as_path())
            .collect::<Vec<_>>(),
        vec![
            Path::new("k8s/a-config.yaml"),
            Path::new("k8s/z-config.yaml"),
        ]
    );
}

#[test]
fn combined_plan_rejects_duplicate_custom_output_paths() {
    let document: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
"#,
    )
    .unwrap();

    let manifests = vec![
        KubernetesManifest {
            file_name: "settings".into(),
            document: document.clone(),
        },
        KubernetesManifest {
            file_name: "settings".into(),
            document,
        },
    ];

    let result = crate::generator::k8s_generator::plan_all_kubernetes(None, &manifests);

    assert!(result.is_err());
}

#[test]
fn rejects_deployment_without_containers() {
    let resource: Value = serde_yaml::from_str(
        r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: backend
spec:
  selector:
    matchLabels:
      app: backend
  template:
    metadata:
      labels:
        app: backend
    spec:
      containers: []
"#,
    )
    .unwrap();

    assert!(validate_supported_manifest(&resource, "k8s.backend").is_err());
}

#[test]
fn rejects_deployment_selector_mismatch() {
    let resource: Value = serde_yaml::from_str(
        r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: backend
spec:
  selector:
    matchLabels:
      app: backend
  template:
    metadata:
      labels:
        app: frontend
    spec:
      containers:
        - name: backend
          image: backend:latest
"#,
    )
    .unwrap();

    assert!(validate_supported_manifest(&resource, "k8s.backend").is_err());
}

#[test]
fn rejects_invalid_raw_service_port() {
    let resource: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: Service
metadata:
  name: backend
spec:
  ports:
    - port: 99999
      targetPort: 8080
"#,
    )
    .unwrap();

    assert!(validate_supported_manifest(&resource, "k8s.backend").is_err());
}

#[test]
fn rejects_raw_deployment_without_spec() {
    let resource: Value = serde_yaml::from_str(
        r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: backend
"#,
    )
    .unwrap();

    let result = validate_supported_manifest(&resource, "k8s.backend");

    assert!(result.is_err());
}

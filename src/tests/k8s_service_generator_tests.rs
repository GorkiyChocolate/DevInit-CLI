use super::*;

#[test]
fn service_uses_deployment_selector() {
    let deployment: Deployment = serde_yaml::from_str(
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
      name: backend
      labels:
        app: backend
    spec:
      containers:
        - name: backend
          image: nginx:latest
          ports:
            - containerPort: 8080
"#,
    )
    .unwrap();

    let service = build_service(&deployment).unwrap();
    let yaml = serde_yaml::to_value(&service).unwrap();

    assert_eq!(yaml["kind"].as_str(), Some("Service"));
    assert_eq!(yaml["spec"]["selector"]["app"].as_str(), Some("backend"));
    assert_eq!(yaml["spec"]["ports"][0]["targetPort"].as_u64(), Some(8080));
}

#[test]
fn no_container_ports_means_no_service() {
    let deployment: Deployment = serde_yaml::from_str(
        r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: worker
spec:
  selector:
    matchLabels:
      app: worker
  template:
    metadata:
      name: worker
      labels:
        app: worker
    spec:
      containers:
        - name: worker
          image: worker:latest
"#,
    )
    .unwrap();

    assert!(build_service(&deployment).is_none());
}

use crate::models::k8s_struct::{Deployment, ObjectMeta};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesService {
    api_version: String,
    kind: String,
    metadata: ObjectMeta,
    spec: ServiceSpec,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceSpec {
    #[serde(rename = "type")]
    service_type: String,
    selector: HashMap<String, String>,
    ports: Vec<ServicePort>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServicePort {
    name: String,
    port: u16,
    target_port: u16,
    protocol: String,
}

/// Builds a ClusterIP Service for a Deployment with exposed ports.
/// Returns None when the Deployment declares no container ports.
pub fn build_service(deployment: &Deployment) -> Option<KubernetesService> {
    let mut ports = Vec::new();
    let mut seen = BTreeSet::new();

    for container in &deployment.spec.template.spec.containers {
        if let Some(container_ports) = &container.ports {
            for port in container_ports {
                let protocol = port
                    .protocol
                    .as_deref()
                    .unwrap_or("TCP")
                    .to_ascii_uppercase();

                let key = (port.container_port, protocol.clone());

                if !seen.insert(key) {
                    continue;
                }

                ports.push(ServicePort {
                    name: format!(
                        "port-{}-{}",
                        port.container_port,
                        protocol.to_ascii_lowercase()
                    ),
                    port: port.container_port,
                    target_port: port.container_port,
                    protocol,
                });
            }
        }
    }

    if ports.is_empty() {
        return None;
    }

    let metadata = ObjectMeta {
        name: deployment.metadata.name.clone(),
        namespace: deployment.metadata.namespace.clone(),
        labels: deployment.metadata.labels.clone(),
        annotations: None,
    };

    Some(KubernetesService {
        api_version: "v1".to_string(),
        kind: "Service".to_string(),
        metadata,
        spec: ServiceSpec {
            service_type: "ClusterIP".to_string(),
            selector: deployment.spec.selector.match_labels.clone(),
            ports,
        },
    })
}

#[cfg(test)]
mod tests {
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
}

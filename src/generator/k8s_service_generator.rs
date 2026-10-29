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
#[path = "../tests/k8s_service_generator_tests.rs"]
mod tests;

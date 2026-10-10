use crate::errors::ValidationError;
use crate::models::k8s_struct::Deployment;

/// Validates the required fields of a Kubernetes deployment.
pub fn validate_deployment(deployment: &Deployment, errors: &mut Vec<ValidationError>) {
    let base = "kubernetes";
    validate_name(
        &deployment.metadata.name,
        &format!("{base}.metadata.name"),
        errors,
    );
    // Require the deployment API version.
    if deployment.api_version.trim().is_empty() {
        errors.push(ValidationError {
            path: format!("{base}.apiVersion"),
            message: "cannot be empty".to_string(),
        });
    }
    // Require the deployment kind.
    if deployment.kind.trim().is_empty() {
        errors.push(ValidationError {
            path: format!("{base}.kind"),
            message: "cannot be empty".to_string(),
        });
    }
    // Reject an explicitly zero replica count.
    if deployment.spec.replicas == Some(0) {
        errors.push(ValidationError {
            path: format!("{base}.spec.replicas"),
            message: "must be greater than 0".to_string(),
        });
    }
    let pod = &deployment.spec.template.spec;
    // Require at least one pod container.
    if pod.containers.is_empty() {
        errors.push(ValidationError {
            path: format!("{base}.spec.template.spec.containers"),
            message: "must contain at least one container".to_string(),
        });
    }
    // Validate every pod container.
    for (index, container) in pod.containers.iter().enumerate() {
        let path = format!("{base}.spec.template.spec.containers[{index}]");
        validate_name(&container.name, &format!("{path}.name"), errors);
        // Require a container image.
        if container.image.trim().is_empty() {
            errors.push(ValidationError {
                path: format!("{path}.image"),
                message: "cannot be empty".to_string(),
            });
        }
        // Validate every declared container port.
        if let Some(ports) = &container.ports {
            for (port_index, port) in ports.iter().enumerate() {
                // Reject port number zero.
                if port.container_port == 0 {
                    errors.push(ValidationError {
                        path: format!("{path}.ports[{port_index}].containerPort"),
                        message: "must be greater than 0".to_string(),
                    });
                }
            }
        }
        // Validate every environment variable.
        if let Some(env) = &container.env {
            for (env_index, variable) in env.iter().enumerate() {
                // Require a name and exactly one value source.
                if variable.name.trim().is_empty()
                    || (variable.value.is_none() && variable.value_from.is_none())
                {
                    errors.push(ValidationError {
                        path: format!("{path}.env[{env_index}]"),
                        message: "must have a name and a value or valueFrom".to_string(),
                    });
                }
                // Reject simultaneous literal and referenced values.
                if variable.value.is_some() && variable.value_from.is_some() {
                    errors.push(ValidationError {
                        path: format!("{path}.env[{env_index}]"),
                        message: "value and valueFrom are mutually exclusive".to_string(),
                    });
                }
            }
        }
        // Validate every mounted volume.
        if let Some(mounts) = &container.volume_mounts {
            for (mount_index, mount) in mounts.iter().enumerate() {
                // Require both a volume name and mount path.
                if mount.name.trim().is_empty() || mount.mount_path.trim().is_empty() {
                    errors.push(ValidationError {
                        path: format!("{path}.volumeMounts[{mount_index}]"),
                        message: "name and mountPath cannot be empty".to_string(),
                    });
                }
            }
        }
    }
    // Validate pod volumes when they are configured.
    if let Some(volumes) = &pod.volumes {
        // Validate every volume declaration.
        for (index, volume) in volumes.iter().enumerate() {
            let path = format!("{base}.spec.template.spec.volumes[{index}]");
            validate_name(&volume.name, &format!("{path}.name"), errors);
            let source_count = [
                volume.empty_dir.is_some(),
                volume.config_map.is_some(),
                volume.secret.is_some(),
                volume.persistent_volume_claim.is_some(),
                volume.host_path.is_some(),
            ]
            .into_iter()
            .filter(|present| *present)
            .count();
            // Require exactly one volume source.
            if source_count != 1 {
                errors.push(ValidationError {
                    path,
                    message: "must define exactly one volume source".to_string(),
                });
            }
        }
    }
}

/// Validates that a Kubernetes resource name is present.
fn validate_name(name: &str, path: &str, errors: &mut Vec<ValidationError>) {
    // Reject blank names.
    if name.trim().is_empty() {
        errors.push(ValidationError {
            path: path.to_string(),
            message: "cannot be empty".to_string(),
        });
    }
}

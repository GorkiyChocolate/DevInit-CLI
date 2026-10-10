use crate::errors::ValidationError;
use crate::models::docker_compose_struct::RecipeCompose;

/// Validates a Docker Compose service recipe.
pub fn validate_service(name: &str, service: &RecipeCompose, errors: &mut Vec<ValidationError>) {
    let base = format!("services.{name}");
    // Require a non-empty service name.
    if name.trim().is_empty() {
        errors.push(ValidationError {
            path: "services".to_string(),
            message: "service name cannot be empty".to_string(),
        });
    }
    // Require a container image.
    if service.image.trim().is_empty() {
        errors.push(ValidationError {
            path: format!("{base}.image"),
            message: "cannot be empty".to_string(),
        });
    }
    // Validate every published port.
    if let Some(ports) = &service.ports {
        for (index, port) in ports.iter().enumerate() {
            validate_port(port, &format!("{base}.ports[{index}]"), errors);
        }
    }
    // Validate every environment entry.
    if let Some(values) = &service.environment {
        for (index, value) in values.iter().enumerate() {
            // Require KEY=VALUE syntax with content.
            if value.trim().is_empty() || value.split_once('=').is_none() {
                errors.push(ValidationError {
                    path: format!("{base}.environment[{index}]"),
                    message: "must contain a non-empty KEY=VALUE pair".to_string(),
                });
            }
        }
    }
    // Validate every volume entry.
    if let Some(values) = &service.volumes {
        for (index, value) in values.iter().enumerate() {
            // Reject blank volume declarations.
            if value.trim().is_empty() {
                errors.push(ValidationError {
                    path: format!("{base}.volumes[{index}]"),
                    message: "cannot be empty".to_string(),
                });
            }
        }
    }
    // Validate every network entry.
    if let Some(values) = &service.networks {
        for (index, value) in values.iter().enumerate() {
            // Reject blank network names.
            if value.trim().is_empty() {
                errors.push(ValidationError {
                    path: format!("{base}.networks[{index}]"),
                    message: "cannot be empty".to_string(),
                });
            }
        }
    }
    // Validate every dependency name.
    if let Some(values) = &service.depends_on {
        for (index, value) in values.iter().enumerate() {
            // Reject blank dependency names.
            if value.trim().is_empty() {
                errors.push(ValidationError {
                    path: format!("{base}.depends_on[{index}]"),
                    message: "service name cannot be empty".to_string(),
                });
            }
        }
    }
    // Validate the generated file path when a file is configured.
    if let Some(file) = &service.files
        && file.path.trim().is_empty()
    {
        errors.push(ValidationError {
            path: format!("{base}.files.path"),
            message: "cannot be empty".to_string(),
        });
    }
}

/// Validates the container port portion of a port mapping.
fn validate_port(value: &str, path: &str, errors: &mut Vec<ValidationError>) {
    let port = value.rsplit(':').next().unwrap_or_default();
    // Accept only numeric, non-zero ports.
    match port.parse::<u16>() {
        Ok(0) | Err(_) => errors.push(ValidationError {
            path: path.to_string(),
            message: "port must be a number greater than 0".to_string(),
        }),
        Ok(_) => {}
    }
}

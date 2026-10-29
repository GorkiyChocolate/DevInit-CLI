use crate::errors::{DevinitError, GenerationError};
use crate::file_config::yaml_writer::write_yaml_with_options;
use crate::models::k8s_struct::Deployment;

use serde_yaml::Value;

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct KubernetesManifest {
    pub file_name: String,
    pub document: Value,
}

fn invalid(message: impl Into<String>) -> DevinitError {
    DevinitError::ConfigurationError(message.into())
}

fn valid_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, DevinitError> {
    let text = value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| invalid(format!("{field} must be a non-empty string")))?;

    Ok(text)
}

fn template_field<'a>(resource: &'a Value, name: &str) -> Option<&'a Value> {
    resource.as_mapping()?.get(Value::String(name.into()))
}

fn template_string<'a>(resource: &'a Value, name: &str) -> Result<&'a str, DevinitError> {
    template_field(resource, name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            invalid(format!(
                "Template field '{name}' must be a non-empty string"
            ))
        })
}

fn template_integer(
    resource: &Value,
    name: &str,
    default: u64,
    minimum: u64,
    maximum: u64,
) -> Result<u64, DevinitError> {
    let value = match template_field(resource, name) {
        Some(value) => value.as_u64().ok_or_else(|| {
            invalid(format!(
                "Template field '{name}' must be a positive integer"
            ))
        })?,
        None => default,
    };

    if !(minimum..=maximum).contains(&value) {
        return Err(invalid(format!(
            "Template field '{name}' must be between {minimum} and {maximum}"
        )));
    }

    Ok(value)
}

fn validate_template_fields(resource: &Value, allowed: &[&str]) -> Result<(), DevinitError> {
    let mapping = resource
        .as_mapping()
        .ok_or_else(|| invalid("Kubernetes template must be a mapping"))?;

    for key in mapping.keys() {
        let name = key
            .as_str()
            .ok_or_else(|| invalid("Template field names must be strings"))?;

        if !allowed.contains(&name) {
            return Err(invalid(format!(
                "Unsupported Kubernetes template field: '{name}'"
            )));
        }
    }

    Ok(())
}

fn validate_resource_name(name: &str) -> Result<(), DevinitError> {
    if name.is_empty()
        || name.len() > 63
        || !name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        || !name
            .bytes()
            .last()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(invalid(format!(
            "Invalid Kubernetes resource name: '{name}'"
        )));
    }

    Ok(())
}

/// Converts a short template declaration into a full
/// Kubernetes resource.
///
/// Raw manifests are returned without modification.
pub fn expand_template(resource: &Value) -> Result<Value, DevinitError> {
    let Some(template) = template_field(resource, "template") else {
        return Ok(resource.clone());
    };

    let template = template
        .as_str()
        .ok_or_else(|| invalid("'template' must be a string"))?;

    match template {
        "deployment" => expand_deployment(resource),
        "service" => expand_service(resource),
        other => Err(invalid(format!(
            "Unsupported Kubernetes template: '{other}'"
        ))),
    }
}

fn expand_deployment(resource: &Value) -> Result<Value, DevinitError> {
    validate_template_fields(resource, &["template", "name", "image", "replicas", "port"])?;

    let name = template_string(resource, "name")?;
    let image = template_string(resource, "image")?;

    validate_resource_name(name)?;

    let replicas = template_integer(resource, "replicas", 1, 0, 10000)?;

    let port = template_integer(resource, "port", 8080, 1, 65535)?;

    let mut deployment: Value = serde_yaml::from_str(
        r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: placeholder
spec:
  replicas: 1
  selector:
    matchLabels:
      app: placeholder
  template:
    metadata:
      labels:
        app: placeholder
    spec:
      containers:
        - name: placeholder
          image: placeholder
          ports:
            - containerPort: 8080
"#,
    )?;

    deployment["metadata"]["name"] = Value::String(name.into());
    deployment["spec"]["replicas"] = Value::Number(replicas.into());

    deployment["spec"]["selector"]["matchLabels"]["app"] = Value::String(name.into());

    deployment["spec"]["template"]["metadata"]["labels"]["app"] = Value::String(name.into());

    let container = &mut deployment["spec"]["template"]["spec"]["containers"][0];

    container["name"] = Value::String(name.into());
    container["image"] = Value::String(image.into());
    container["ports"][0]["containerPort"] = Value::Number(port.into());

    Ok(deployment)
}

fn expand_service(resource: &Value) -> Result<Value, DevinitError> {
    validate_template_fields(
        resource,
        &["template", "name", "target", "port", "targetPort", "type"],
    )?;

    let name = template_string(resource, "name")?;
    let target = template_string(resource, "target")?;

    validate_resource_name(name)?;
    validate_resource_name(target)?;

    let port = template_integer(resource, "port", 80, 1, 65535)?;

    let target_port = template_integer(resource, "targetPort", port, 1, 65535)?;

    let service_type = template_field(resource, "type")
        .map(|_| template_string(resource, "type"))
        .transpose()?
        .unwrap_or("ClusterIP");

    if !matches!(service_type, "ClusterIP" | "NodePort" | "LoadBalancer") {
        return Err(invalid(format!(
            "Unsupported Kubernetes Service type: '{service_type}'"
        )));
    }

    let mut service: Value = serde_yaml::from_str(
        r#"
apiVersion: v1
kind: Service
metadata:
  name: placeholder
spec:
  type: ClusterIP
  selector:
    app: placeholder
  ports:
    - port: 80
      targetPort: 80
      protocol: TCP
"#,
    )?;

    service["metadata"]["name"] = Value::String(name.into());
    service["spec"]["type"] = Value::String(service_type.into());

    service["spec"]["selector"]["app"] = Value::String(target.into());

    service["spec"]["ports"][0]["port"] = Value::Number(port.into());

    service["spec"]["ports"][0]["targetPort"] = Value::Number(target_port.into());

    Ok(service)
}
/// Loads custom Kubernetes manifests from the `k8s` section.
pub fn load_manifests(path: &Path) -> Result<Vec<KubernetesManifest>, DevinitError> {
    let contents = fs::read_to_string(path).map_err(DevinitError::FileIOError)?;

    let document: Value = serde_yaml::from_str(&contents)?;

    let Some(k8s) = document.get("k8s") else {
        return Ok(Vec::new());
    };

    let manifests = k8s
        .as_mapping()
        .ok_or_else(|| invalid("k8s must be a mapping of manifest names to resources"))?;

    let mut result = Vec::new();
    let mut identities = HashSet::new();

    for (file_key, resource) in manifests {
        let name = required_string(file_key, "k8s manifest name")?;

        if !valid_file_name(name) {
            return Err(invalid(format!(
                "Invalid Kubernetes manifest filename: {name}"
            )));
        }

        let expanded = expand_template(resource)?;
        let resource = &expanded;

        let mapping = resource.as_mapping().ok_or_else(|| {
            invalid(format!(
                "k8s.{name} must contain a Kubernetes resource object"
            ))
        })?;

        let api_version = required_string(
            mapping
                .get(Value::String("apiVersion".into()))
                .unwrap_or(&Value::Null),
            &format!("k8s.{name}.apiVersion"),
        )?;

        let kind = required_string(
            mapping
                .get(Value::String("kind".into()))
                .unwrap_or(&Value::Null),
            &format!("k8s.{name}.kind"),
        )?;

        let metadata = mapping
            .get(Value::String("metadata".into()))
            .ok_or_else(|| invalid(format!("k8s.{name}.metadata is required")))?;

        let metadata_name = required_string(
            metadata.get("name").unwrap_or(&Value::Null),
            &format!("k8s.{name}.metadata.name"),
        )?;

        let namespace = metadata
            .get("namespace")
            .and_then(Value::as_str)
            .unwrap_or("");

        // Different files must not declare the same Kubernetes
        // resource identity.
        let identity = (
            api_version.to_owned(),
            kind.to_owned(),
            namespace.to_owned(),
            metadata_name.to_owned(),
        );

        if !identities.insert(identity) {
            return Err(invalid(format!(
                "Duplicate Kubernetes resource in k8s.{name}: \
                 {kind}/{metadata_name}"
            )));
        }

        result.push(KubernetesManifest {
            file_name: name.to_owned(),
            document: resource.clone(),
        });
    }

    // Stable output order regardless of declaration order.
    result.sort_by(|a, b| a.file_name.cmp(&b.file_name));

    Ok(result)
}

/// Checks path collisions and existing destination files.
///
/// The legacy Deployment generator is also considered to avoid
/// generating the same output path twice.
pub fn preflight(
    manifests: &[KubernetesManifest],
    output_dir: &Path,
    force: bool,
    legacy: Option<&Deployment>,
) -> Result<(), DevinitError> {
    let mut reserved = HashSet::new();

    if let Some(deployment) = legacy {
        let name = &deployment.metadata.name;

        reserved.insert(format!("{name}-deployment.yaml"));

        if crate::generator::k8s_service_generator::build_service(deployment).is_some() {
            reserved.insert(format!("{name}-service.yaml"));
        }
    }

    for manifest in manifests {
        let filename = format!("{}.yaml", manifest.file_name);

        if reserved.contains(&filename) {
            return Err(invalid(format!(
                "Kubernetes manifest '{}' conflicts with \
                 a legacy generated resource",
                manifest.file_name
            )));
        }

        let destination = output_dir.join("k8s").join(filename);

        if !force {
            match destination.symlink_metadata() {
                Ok(_) => {
                    return Err(DevinitError::FileIOError(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!(
                            "Refusing to overwrite {} (use --force)",
                            destination.display()
                        ),
                    )));
                }

                Err(error) if error.kind() == io::ErrorKind::NotFound => {}

                Err(error) => {
                    return Err(DevinitError::FileIOError(error));
                }
            }
        }
    }

    Ok(())
}

/// Writes already validated Kubernetes manifests.
///
/// Preflight must be called before generating any output.
pub fn write_manifests(
    manifests: &[KubernetesManifest],
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    let mut generated = Vec::new();

    for manifest in manifests {
        let path = output_dir
            .join("k8s")
            .join(format!("{}.yaml", manifest.file_name));

        let written = write_yaml_with_options(&manifest.document, &path, force)?;

        generated.push(written);
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
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
services: []
k8s:
  backend:
    apiVersion: apps/v1
    kind: Deployment
    metadata:
      name: backend
  networking:
    apiVersion: v1
    kind: Service
    metadata:
      name: backend
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
}

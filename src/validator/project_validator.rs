use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};

fn field<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

fn nonempty_string<'a>(value: &'a Value, context: &str) -> Result<&'a str, String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("{context} must be a string"))?;

    if text.trim().is_empty() {
        return Err(format!("{context} must not be empty"));
    }

    Ok(text)
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.len() > 80
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || !name.as_bytes()[0].is_ascii_alphanumeric()
    {
        return Err(format!(
            "Invalid service name '{name}': \
             use letters, numbers, hyphens or underscores, \
             starting with a letter or number"
        ));
    }

    Ok(())
}

/// Validate source declarations before registry resolution.
pub fn validate_declarations(document: &Value) -> Result<(), String> {
    let Some(services_value) = document.get("services") else {
        return Ok(());
    };

    let services = services_value
        .as_sequence()
        .ok_or("services must be a YAML sequence")?;

    let mut names = HashSet::new();

    for (index, service) in services.iter().enumerate() {
        let context = format!("services[{index}]");

        // Preserve compatibility with legacy scalar entries.
        let Some(map) = service.as_mapping() else {
            continue;
        };

        let Some(name_value) = field(map, "name") else {
            // Existing model validation handles missing names.
            continue;
        };

        let name = nonempty_string(name_value, &format!("{context}.name"))?;

        validate_name(name)?;

        if !names.insert(name.to_string()) {
            return Err(format!("Duplicate service name '{name}' at {context}"));
        }

        if let Some(version) = field(map, "version") {
            nonempty_string(version, &format!("{context}.version"))?;

            if field(map, "image").is_some() {
                return Err(format!(
                    "{context} ('{name}') cannot contain both \
                     'image' and 'version'; use either a custom \
                     image or a versioned registry recipe"
                ));
            }
        }

        if let Some(image) = field(map, "image") {
            nonempty_string(image, &format!("{context}.image"))?;
        }
    }

    Ok(())
}

/// Parse a simple Compose published-port mapping:
/// 8080:80, 127.0.0.1:8080:80 or 8080:80/udp.
///
/// Complex mappings are left to the existing Compose validation.
fn published_port(value: &Value) -> Option<(u16, String, String)> {
    let text = value.as_str()?;
    let (without_protocol, protocol) = text.rsplit_once('/').unwrap_or((text, "tcp"));

    let parts: Vec<&str> = without_protocol.split(':').collect();

    if parts.len() < 2 {
        // Container-only port: no published host port.
        return None;
    }

    let (host, host_port) = if parts.len() == 2 {
        ("0.0.0.0", parts[0])
    } else if parts.len() == 3 {
        (parts[0], parts[1])
    } else {
        // IPv6 and complex formats are not checked here.
        return None;
    };

    let port = host_port.parse::<u16>().ok()?;

    if port == 0 {
        return None;
    }

    Some((port, protocol.to_ascii_lowercase(), host.to_string()))
}

/// Validate resolved recipes before generators write any files.
pub fn validate_resolved_services(document: &Value) -> Result<(), String> {
    let Some(services) = document.get("services").and_then(Value::as_sequence) else {
        return Ok(());
    };

    let mut ports: HashMap<(u16, String), Vec<(String, String)>> = HashMap::new();

    for (index, service) in services.iter().enumerate() {
        let Some(map) = service.as_mapping() else {
            continue;
        };

        let name = field(map, "name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        let Some(port_values) = field(map, "ports") else {
            continue;
        };

        if port_values.is_null() {
            continue;
        }

        let entries = port_values
            .as_sequence()
            .ok_or_else(|| format!("services[{index}] ('{name}').ports must be a list"))?;

        for entry in entries {
            let Some((port, protocol, host)) = published_port(entry) else {
                // Unsupported syntax is left to existing validators.
                continue;
            };

            let key = (port, protocol.clone());
            let bindings = ports.entry(key).or_default();

            for (previous_name, previous_host) in bindings.iter() {
                let overlaps = host == previous_host.as_str()
                    || host == "0.0.0.0"
                    || previous_host == "0.0.0.0";

                if overlaps {
                    return Err(format!(
                        "Host port {port}/{protocol} conflict: \
                         '{previous_name}' ({previous_host}) \
                         and '{name}' ({host})"
                    ));
                }
            }

            bindings.push((name.to_string(), host));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Value {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn accepts_valid_registry_services() {
        let doc = parse(
            "services:\n  - name: postgresql\n    version: '17'\n  - name: redis\n    version: '7'\n",
        );

        assert!(validate_declarations(&doc).is_ok());
    }

    #[test]
    fn rejects_duplicate_names() {
        let doc = parse("services:\n  - name: redis\n  - name: redis\n");

        let error = validate_declarations(&doc).unwrap_err();
        assert!(error.contains("Duplicate service"));
    }

    #[test]
    fn rejects_invalid_name() {
        let doc = parse("services:\n  - name: ../postgresql\n");

        assert!(validate_declarations(&doc).is_err());
    }

    #[test]
    fn rejects_image_and_version_combination() {
        let doc = parse("services:\n  - name: redis\n    image: redis:7\n    version: '7'\n");

        assert!(validate_declarations(&doc).is_err());
    }

    #[test]
    fn detects_conflicting_host_ports() {
        let doc = parse(
            "services:\n  - name: api\n    ports: ['8080:80']\n  - name: gateway\n    ports: ['8080:3000']\n",
        );

        let error = validate_resolved_services(&doc).unwrap_err();
        assert!(error.contains("8080/tcp"));
        assert!(error.contains("gateway"));
    }

    #[test]
    fn different_protocols_do_not_conflict() {
        let doc = parse(
            "services:\n  - name: tcp-service\n    ports: ['8080:80/tcp']\n  - name: udp-service\n    ports: ['8080:80/udp']\n",
        );

        assert!(validate_resolved_services(&doc).is_ok());
    }

    #[test]
    fn detects_duplicate_bindings_in_same_service() {
        let doc = parse("services:\n  - name: api\n    ports: ['8080:80', '8080:81']\n");

        assert!(validate_resolved_services(&doc).is_err());
    }
}

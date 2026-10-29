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

/// Parses a supported Docker Compose short port mapping.
///
/// Supported:
/// - "80"
/// - "8080:80"
/// - "127.0.0.1:8080:80"
/// - "8080:80/tcp"
/// - "8080:80/udp"
///
/// IPv6 mappings and port ranges are left to a dedicated
/// Compose-compatible validator.
fn published_port(value: &Value) -> Result<Option<(u16, String, String)>, String> {
    let text = value
        .as_str()
        .ok_or("Port mapping must be a string")?
        .trim();

    if text.is_empty() {
        return Err("Port mapping must not be empty".into());
    }

    let (mapping, protocol) = match text.rsplit_once('/') {
        Some((mapping, protocol)) => {
            if !matches!(protocol, "tcp" | "udp" | "sctp") {
                return Err(format!(
                    "Unsupported port protocol '{protocol}' in '{text}'"
                ));
            }

            (mapping, protocol)
        }

        None => (text, "tcp"),
    };

    // Complex Compose syntax requires a separate parser.
    // Do not incorrectly interpret bracketed IPv6 addresses
    // or port ranges as simple bindings.
    if mapping.contains('[') || mapping.contains(']') || mapping.contains('-') {
        return Ok(None);
    }

    let parts: Vec<&str> = mapping.split(':').collect();

    let (host, published, target) = match parts.as_slice() {
        [target] => ("0.0.0.0", None, *target),

        [published, target] => ("0.0.0.0", Some(*published), *target),

        [host, published, target] => {
            if host.is_empty() {
                return Err(format!("Host address must not be empty in '{text}'"));
            }

            (*host, Some(*published), *target)
        }

        _ => {
            return Err(format!("Invalid port mapping format: '{text}'"));
        }
    };

    fn parse_port(value: &str, mapping: &str) -> Result<u16, String> {
        let port = value
            .parse::<u16>()
            .map_err(|_| format!("Invalid port '{value}' in '{mapping}'"))?;

        if port == 0 {
            return Err(format!("Port must be between 1 and 65535 in '{mapping}'"));
        }

        Ok(port)
    }

    // Target port is always required.
    parse_port(target, text)?;

    let Some(published) = published else {
        return Ok(None);
    };

    let published = parse_port(published, text)?;

    Ok(Some((published, protocol.to_string(), host.to_string())))
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
            let Some((port, protocol, host)) = published_port(entry)
                .map_err(|error| format!("services[{index}] ('{name}').ports: {error}"))?
            else {
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
#[path = "../tests/project_validator_tests.rs"]
mod tests;

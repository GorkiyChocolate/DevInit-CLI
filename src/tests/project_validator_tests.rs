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

#[test]
fn accepts_valid_port_mappings() {
    let doc = parse(
        r#"
services:
  - name: api
    ports:
      - "8080:80"
      - "127.0.0.1:8081:81"
      - "8082:82/udp"
      - "83"
"#,
    );

    assert!(validate_resolved_services(&doc).is_ok());
}

#[test]
fn rejects_out_of_range_published_port() {
    let doc = parse(
        r#"
services:
  - name: api
    ports:
      - "99999:80"
"#,
    );

    assert!(validate_resolved_services(&doc).is_err());
}

#[test]
fn rejects_out_of_range_target_port() {
    let doc = parse(
        r#"
services:
  - name: api
    ports:
      - "8080:99999"
"#,
    );

    assert!(validate_resolved_services(&doc).is_err());
}

#[test]
fn rejects_zero_ports() {
    for port in ["0:80", "8080:0", "0"] {
        let yaml = format!("services:\n  - name: api\n    ports: ['{port}']\n");

        let doc = parse(&yaml);

        assert!(
            validate_resolved_services(&doc).is_err(),
            "Expected rejection for {port}"
        );
    }
}

#[test]
fn rejects_unknown_protocol() {
    let doc = parse(
        r#"
services:
  - name: api
    ports:
      - "8080:80/invalid"
"#,
    );

    assert!(validate_resolved_services(&doc).is_err());
}

#[test]
fn accepts_same_port_with_different_protocols() {
    let doc = parse(
        r#"
services:
  - name: api
    ports:
      - "8080:80/tcp"

  - name: dns
    ports:
      - "8080:53/udp"
"#,
    );

    assert!(validate_resolved_services(&doc).is_ok());
}

#[test]
fn rejects_conflicting_published_ports() {
    let doc = parse(
        r#"
services:
  - name: backend
    ports:
      - "8080:3000"

  - name: frontend
    ports:
      - "8080:80"
"#,
    );

    let error = validate_resolved_services(&doc).unwrap_err();

    assert!(error.contains("8080/tcp"));
}

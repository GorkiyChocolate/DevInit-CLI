use super::*;

fn parse(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap()
}

fn resolve(yaml: &str) -> Result<Value, DevinitError> {
    let mut document = parse(yaml);

    resolve_document(&mut document, Path::new("."))?;

    Ok(document)
}

#[test]
fn resolves_postgresql_version() {
    let document = resolve("services:\n  - name: postgresql\n    version: '17'\n").unwrap();

    assert_eq!(
        document["services"][0]["image"].as_str(),
        Some("postgres:17")
    );
}

#[test]
fn overrides_host_port() {
    let document = resolve(
        "services:\n  - name: postgresql\n    version: '17'\n    ports:\n      - '5433:5432'\n",
    )
    .unwrap();

    assert_eq!(
        document["services"][0]["ports"][0].as_str(),
        Some("5433:5432")
    );
}

#[test]
fn overrides_environment() {
    let document = resolve(
        "services:\n  - name: postgresql\n    environment:\n      - 'POSTGRES_DB=devinit'\n",
    )
    .unwrap();

    assert_eq!(
        document["services"][0]["environment"][0].as_str(),
        Some("POSTGRES_DB=devinit")
    );
}

#[test]
fn preserves_default_image() {
    let document =
        resolve("services:\n  - name: redis\n    version: '7'\n    restart: always\n").unwrap();

    assert_eq!(document["services"][0]["image"].as_str(), Some("redis:7"));

    assert_eq!(document["services"][0]["restart"].as_str(), Some("always"));
}

#[test]
fn rejects_unknown_override() {
    let result = resolve("services:\n  - name: redis\n    unknown_setting: true\n");

    assert!(result.is_err());
}

#[test]
fn rejects_unsupported_version() {
    let result = resolve("services:\n  - name: postgresql\n    version: '999'\n");

    assert!(result.is_err());
}

#[test]
fn preserves_custom_services() {
    let document = resolve(
        "services:\n  - name: api\n    image: my-api:latest\n    ports:\n      - '8080:8080'\n",
    )
    .unwrap();

    assert_eq!(
        document["services"][0]["image"].as_str(),
        Some("my-api:latest")
    );
}

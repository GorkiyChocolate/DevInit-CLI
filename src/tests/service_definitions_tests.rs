use crate::service_registry::packaged_recipe_version;

use super::*;
use std::path::PathBuf;

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("services")
}

#[test]
fn loads_builtin_service_definitions() {
    let registry = load_service_registry(&definitions_dir()).unwrap();

    assert!(registry.contains_key("postgresql"));
    assert!(registry.contains_key("redis"));
}

#[test]
fn resolves_default_postgresql_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let postgresql = registry.get("postgresql").unwrap();

    let recipe = postgresql.to_recipe(None).unwrap();

    assert_eq!(recipe.image, "postgres:16");
}

#[test]
fn resolves_explicit_postgresql_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let postgresql = registry.get("postgresql").unwrap();

    let recipe = postgresql.to_recipe(Some("17")).unwrap();

    assert_eq!(recipe.image, "postgres:17");
}

#[test]
fn resolves_explicit_redis_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let redis = registry.get("redis").unwrap();

    let recipe = redis.to_recipe(Some("6")).unwrap();

    assert_eq!(recipe.image, "redis:6");
}

#[test]
fn rejects_unsupported_service_version() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let postgresql = registry.get("postgresql").unwrap();

    let error = postgresql.to_recipe(Some("99")).unwrap_err();

    assert!(error.to_string().contains("unsupported version"));
}

#[test]
fn packaged_postgresql_uses_yaml_default() {
    let recipe = packaged_recipe_version("postgresql", None)
        .unwrap()
        .unwrap();

    assert_eq!(recipe.name, "postgresql");
    assert_eq!(recipe.image, "postgres:16");
}

#[test]
fn packaged_postgresql_selects_explicit_version() {
    let recipe = packaged_recipe_version("postgresql", Some("17"))
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "postgres:17");
}

#[test]
fn packaged_redis_selects_explicit_version() {
    let recipe = packaged_recipe_version("redis", Some("6"))
        .unwrap()
        .unwrap();

    assert_eq!(recipe.image, "redis:6");
}

#[test]
fn packaged_registry_rejects_unsupported_version() {
    let error = packaged_recipe_version("postgresql", Some("99")).unwrap_err();

    assert!(error.to_string().contains("unsupported version"));
}

#[test]
fn packaged_registry_returns_none_for_unknown_service() {
    let recipe = packaged_recipe_version("devinit-nonexistent-service", None).unwrap();

    assert!(recipe.is_none());
}

#[test]
fn loads_all_database_definitions() {
    let registry = load_service_registry(&definitions_dir()).unwrap();

    for name in ["postgresql", "redis", "mysql", "mariadb", "mongodb"] {
        assert!(
            registry.contains_key(name),
            "Missing service definition: {name}"
        );
    }

    assert_eq!(
        registry["mysql"].to_recipe(None).unwrap().image,
        "mysql:8.4"
    );
    assert_eq!(
        registry["mariadb"].to_recipe(None).unwrap().image,
        "mariadb:11.4"
    );
    assert_eq!(
        registry["mongodb"].to_recipe(None).unwrap().image,
        "mongo:7"
    );
}

#[test]
fn resolves_explicit_database_versions() {
    let registry = load_service_registry(&definitions_dir()).unwrap();

    assert_eq!(
        registry["mysql"].to_recipe(Some("9")).unwrap().image,
        "mysql:9"
    );
    assert_eq!(
        registry["mariadb"].to_recipe(Some("11.8")).unwrap().image,
        "mariadb:11.8"
    );
    assert_eq!(
        registry["mongodb"].to_recipe(Some("8")).unwrap().image,
        "mongo:8"
    );
}

#[test]
fn database_recipes_require_external_credentials() {
    let registry = load_service_registry(&definitions_dir()).unwrap();

    for name in ["mysql", "mariadb", "mongodb"] {
        let recipe = registry[name].to_recipe(None).unwrap();
        let environment = recipe.environment.unwrap();

        assert!(
            environment.iter().any(|value| value.contains(":?Set ")),
            "{name} must require externally supplied credentials"
        );
    }
}

#[test]
fn resolves_rabbitmq_versions_from_yaml() {
    let registry = load_service_registry(&definitions_dir()).unwrap();

    let rabbitmq = &registry["rabbitmq"];

    assert_eq!(
        rabbitmq.to_recipe(None).unwrap().image,
        "rabbitmq:4.1-management"
    );

    assert_eq!(
        rabbitmq.to_recipe(Some("3.13")).unwrap().image,
        "rabbitmq:3.13-management"
    );

    assert_eq!(
        rabbitmq.to_recipe(Some("4.0")).unwrap().image,
        "rabbitmq:4.0-management"
    );

    assert!(rabbitmq.to_recipe(Some("99")).is_err());
}

#[test]
fn resolves_kafka_kraft_configuration() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let kafka = &registry["kafka"];

    let recipe = kafka.to_recipe(None).unwrap();

    assert_eq!(recipe.image, "apache/kafka:4.3.1");

    let environment = recipe.environment.unwrap();

    assert!(
        environment
            .iter()
            .any(|value| { value == "KAFKA_PROCESS_ROLES=broker,controller" })
    );

    assert!(
        environment
            .iter()
            .any(|value| { value.contains("PLAINTEXT://kafka:19092") })
    );

    assert!(
        environment
            .iter()
            .any(|value| { value.contains("KAFKA_CLUSTER_ID") })
    );

    assert_eq!(
        kafka.to_recipe(Some("4.4.0")).unwrap().image,
        "apache/kafka:4.4.0"
    );

    assert!(kafka.to_recipe(Some("99")).is_err());
}

#[test]
fn resolves_grafana_from_yaml() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let grafana = &registry["grafana"];

    let recipe = grafana.to_recipe(None).unwrap();

    assert_eq!(recipe.name, "grafana");
    assert_eq!(recipe.image, "grafana/grafana:12.4.1");

    assert_eq!(
        grafana.to_recipe(Some("12.3.2")).unwrap().image,
        "grafana/grafana:12.3.2"
    );

    let environment = recipe.environment.unwrap();

    assert!(
        environment
            .iter()
            .any(|value| { value.contains("GF_SECURITY_ADMIN_PASSWORD=") })
    );

    assert!(grafana.to_recipe(Some("99")).is_err());
}

#[test]
fn loads_additional_file_from_yaml_definition() {
    let yaml = r#"
schema_version: 1
name: test-monitor
description: Test monitoring service
default_version: "1"
versions:
  "1":
    image: test-monitor:1
defaults:
  files:
    path: monitoring/config.yml
    content: |
      global:
        scrape_interval: 15s
"#;

    let definition: ServiceDefinition = serde_yaml::from_str(yaml).unwrap();

    let recipe = definition.to_recipe(None).unwrap();
    let files = recipe.files.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "monitoring/config.yml");
    assert!(files[0].content.contains("scrape_interval: 15s"));
}

#[test]
fn loads_multiple_files_from_yaml_definition() {
    let yaml = r#"
schema_version: 1
name: test-monitor
description: Test monitoring service
default_version: "1"
versions:
  "1":
    image: test-monitor:1
defaults:
  files:
    - path: monitoring/prometheus.yml
      content: |
        global:
          scrape_interval: 15s
    - path: monitoring/alerts.yml
      content: |
        groups: []
"#;

    let definition: ServiceDefinition = serde_yaml::from_str(yaml).unwrap();

    let recipe = definition.to_recipe(None).unwrap();
    let files = recipe.files.unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "monitoring/prometheus.yml");
    assert_eq!(files[1].path, "monitoring/alerts.yml");
}

#[test]
fn resolves_prometheus_with_multiple_config_files() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let prometheus = &registry["prometheus"];

    let recipe = prometheus.to_recipe(None).unwrap();

    assert_eq!(recipe.name, "prometheus");
    assert_eq!(recipe.image, "prom/prometheus:v3.13.3");

    let files = recipe.files.unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "prometheus/prometheus.yml");
    assert_eq!(files[1].path, "prometheus/alerts.yml");

    assert!(files[0].content.contains("scrape_configs:"));
    assert!(files[1].content.contains("groups: []"));

    assert!(prometheus.to_recipe(Some("99")).is_err());
}

#[test]
fn resolves_elasticsearch_from_yaml() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let elasticsearch = &registry["elasticsearch"];

    let recipe = elasticsearch.to_recipe(None).unwrap();

    assert_eq!(recipe.name, "elasticsearch");
    assert_eq!(
        recipe.image,
        "docker.elastic.co/elasticsearch/elasticsearch:9.5.5"
    );

    let environment = recipe.environment.unwrap();

    assert!(
        environment
            .iter()
            .any(|value| { value == "discovery.type=single-node" })
    );

    assert!(
        environment
            .iter()
            .any(|value| { value == "xpack.security.enabled=false" })
    );

    assert!(
        environment
            .iter()
            .any(|value| { value == "ES_JAVA_OPTS=-Xms512m -Xmx512m" })
    );

    assert_eq!(
        elasticsearch.to_recipe(Some("9.5.3")).unwrap().image,
        "docker.elastic.co/elasticsearch/elasticsearch:9.5.3"
    );

    assert!(elasticsearch.to_recipe(Some("99")).is_err());
}

#[test]
fn resolves_s3_minio_from_yaml() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let definition = &registry["s3"];

    let recipe = definition.to_recipe(None).unwrap();

    assert_eq!(recipe.name, "s3");
    assert_eq!(
        recipe.image,
        "quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z"
    );

    let ports = recipe.ports.unwrap();

    assert!(ports.contains(&"127.0.0.1:9000:9000".to_string()));
    assert!(ports.contains(&"127.0.0.1:9001:9001".to_string()));

    let environment = recipe.environment.unwrap();

    assert!(
        environment
            .iter()
            .any(|value| { value.contains("MINIO_ROOT_PASSWORD=${MINIO_ROOT_PASSWORD:") })
    );

    assert!(definition.to_recipe(Some("unknown")).is_err());
}

#[test]
fn resolves_ec2_localstack_from_yaml() {
    let registry = load_service_registry(&definitions_dir()).unwrap();
    let definition = &registry["ec2"];

    let recipe = definition.to_recipe(None).unwrap();

    assert_eq!(recipe.name, "ec2");

    assert_eq!(recipe.image, "localstack/localstack:2026.10.4");

    let ports = recipe.ports.unwrap();

    assert!(ports.contains(&"127.0.0.1:4566:4566".to_string()));

    let environment = recipe.environment.unwrap();

    assert!(
        environment
            .iter()
            .any(|value| { value.contains("LOCALSTACK_AUTH_TOKEN=") })
    );

    assert_eq!(
        definition.to_recipe(Some("2026.10.2")).unwrap().image,
        "localstack/localstack:2026.10.2"
    );

    assert!(definition.to_recipe(Some("unknown")).is_err());
}

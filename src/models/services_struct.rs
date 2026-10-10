use crate::models::docker_compose_struct::RecipeCompose;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Lists predefined service recipes.
pub enum Services {
    #[serde(rename = "postgresql")]
    PostgreSQL,
    #[serde(rename = "mysql")]
    MySQL,
    #[serde(rename = "mongodb")]
    MongoDB,
    #[serde(rename = "mariadb")]
    MariaDB,
    #[serde(rename = "elasticsearch")]
    Elasticsearch,
    #[serde(rename = "prometheus")]
    Prometheus,
    #[serde(rename = "grafana")]
    Grafana,
    #[serde(rename = "redis")]
    Redis,
    #[serde(rename = "rabbitmq")]
    RabbitMQ,
    #[serde(rename = "kafka")]
    Kafka,
    #[serde(rename = "s3")]
    S3,
    #[serde(rename = "ec2")]
    EC2,
}

impl Services {
    /// Converts a predefined service into a Compose recipe.
    pub fn to_recipe(&self) -> RecipeCompose {
        let (name, description, image, ports, command) = match self {
            Self::PostgreSQL => (
                "postgresql",
                "PostgreSQL database",
                "postgres:16",
                Some(vec!["5432:5432"]),
                None,
            ),
            Self::MySQL => (
                "mysql",
                "MySQL database",
                "mysql:8.4",
                Some(vec!["3306:3306"]),
                None,
            ),
            Self::MongoDB => (
                "mongodb",
                "MongoDB document database",
                "mongo:7",
                Some(vec!["27017:27017"]),
                None,
            ),
            Self::MariaDB => (
                "mariadb",
                "MariaDB relational database",
                "mariadb:11",
                Some(vec!["3306:3306"]),
                None,
            ),
            Self::Elasticsearch => (
                "elasticsearch",
                "Elasticsearch search engine",
                "docker.elastic.co/elasticsearch/elasticsearch:8.15.0",
                Some(vec!["9200:9200"]),
                None,
            ),
            Self::Prometheus => (
                "prometheus",
                "Prometheus metrics collector",
                "prom/prometheus:latest",
                Some(vec!["9090:9090"]),
                None,
            ),
            Self::Grafana => (
                "grafana",
                "Grafana dashboards",
                "grafana/grafana:latest",
                Some(vec!["3000:3000"]),
                None,
            ),
            Self::Redis => (
                "redis",
                "Redis in-memory data store",
                "redis:7",
                Some(vec!["6379:6379"]),
                None,
            ),
            Self::RabbitMQ => (
                "rabbitmq",
                "RabbitMQ message broker",
                "rabbitmq:3-management",
                Some(vec!["5672:5672", "15672:15672"]),
                None,
            ),
            Self::Kafka => (
                "kafka",
                "Apache Kafka event streaming platform",
                "bitnami/kafka:3.8",
                Some(vec!["9092:9092"]),
                None,
            ),
            Self::S3 => (
                "s3",
                "S3-compatible object storage",
                "minio/minio:latest",
                Some(vec!["9000:9000", "9001:9001"]),
                Some(vec!["server".to_string(), "/data".to_string()]),
            ),
            Self::EC2 => (
                "ec2",
                "LocalStack AWS service emulator",
                "localstack/localstack:latest",
                Some(vec!["4566:4566"]),
                None,
            ),
        };

        RecipeCompose {
            name: name.to_string(),
            description: Some(description.to_string()),
            image: image.to_string(),
            ports: ports.map(|values| values.into_iter().map(String::from).collect()),
            environment: None,
            volumes: None,
            networks: None,
            depends_on: None,
            restart: Some("unless-stopped".to_string()),
            command,
            files: None,
            env: None,
            notes: None,
        }
    }
}
pub fn services_selector(services: Services) -> Result<(), std::io::Error> {
    // Report the selected service category.
    match services {
        Services::Elasticsearch => {
            println!("Elasticsearch service selected");
        }
        Services::Grafana => {
            println!("Grafana service selected");
        }
        Services::Redis => {
            println!("Redis service selected");
        }
        Services::RabbitMQ => {
            println!("RabbitMQ service selected");
        }
        Services::Kafka => {
            println!("Kafka service selected");
        }
        Services::S3 => {
            println!("S3 service selected");
        }
        Services::EC2 => {
            println!("EC2 service selected");
        }
        _ => {
            println!("Service not supported for validation");
        }
    }
    Ok(())
}

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Describes one configuration validation failure.
pub struct ValidationError {
    /// Identifies the invalid configuration location.
    pub path: String,
    /// Explains why the value is invalid.
    pub message: String,
}

#[derive(Debug, Error)]
/// Represents failures while generating output files.
pub enum GenerationError {
    #[error("generation I/O failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("generation serialization failed: {0}")]
    Serialization(#[from] serde_yaml::Error),
}

#[derive(Error, Debug)]
/// Represents errors exposed by the CLI workflow.
pub enum DevinitError {
    #[error("HTTP request failed: {0}")]
    HttpRequestError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("File I/O error: {0}")]
    FileIOError(#[from] std::io::Error),

    #[error("YAML parsing failed: {0}")]
    YamlParseError(#[from] serde_yaml::Error),

    #[error("configuration validation failed")]
    ValidationErrors(Vec<ValidationError>),

    #[error("file generation failed: {0}")]
    GenerationError(#[from] GenerationError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Kubernetes validation error: {0}")]
    K8sValidationError(String),

    #[error("CI/CD generation error: {0}")]
    CiCdGenerationError(String),

    #[error("Environment configuration error: {0}")]
    EnvConfigError(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Service generation error: {0}")]
    ServiceGenerationError(String),
}

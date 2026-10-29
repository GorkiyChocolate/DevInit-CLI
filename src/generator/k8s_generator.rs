use crate::errors::GenerationError;
use crate::file_config::yaml_writer::write_yaml_with_options;
use crate::models::k8s_struct::Deployment;
use std::path::{Path, PathBuf};

/// Generates a Kubernetes deployment YAML file.
pub fn generate_k8s(
    deployment: &Deployment,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, GenerationError> {
    generate_k8s_with_options(deployment, output_dir, false)
}

pub fn generate_k8s_with_options(
    deployment: &Deployment,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    let path = output_dir
        .join("k8s")
        .join(format!("{}-deployment.yaml", deployment.metadata.name));
    let path = write_yaml_with_options(deployment, &path, force)?;
    Ok(vec![path])
}

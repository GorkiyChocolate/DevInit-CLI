pub mod cicd_generator;
pub mod docker_compose_generator;
pub mod k8s_generator;

use crate::errors::GenerationError;
use crate::models::compile_struct::CompileSpec;
use std::path::{Path, PathBuf};

/// Generates all requested output files for a compile specification.
pub fn generate(spec: &CompileSpec, output_dir: &Path) -> Result<Vec<PathBuf>, GenerationError> {
    let mut generated = Vec::new();

    // Generate Compose output when services are configured.
    if !spec.services.is_empty() {
        generated.extend(docker_compose_generator::generate_docker_compose(
            &spec.services,
            output_dir,
        )?);
    }

    // Generate Kubernetes output when a deployment is configured.
    if let Some(kubernetes) = &spec.kubernetes {
        generated.extend(k8s_generator::generate_k8s(kubernetes, output_dir)?);
    }
    // Generate CI/CD output when a pipeline is configured.
    if let Some(cicd) = &spec.cicd {
        generated.extend(cicd_generator::generate_cicd(cicd, output_dir)?);
    }

    Ok(generated)
}

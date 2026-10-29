pub mod cicd_generator;
pub mod docker_compose_generator;
pub mod k8s_generator;

use crate::errors::GenerationError;
use crate::models::compile_struct::CompileSpec;
use std::path::{Path, PathBuf};

/// Generates all requested output files for a compile specification.
pub fn generate(spec: &CompileSpec, output_dir: &Path) -> Result<Vec<PathBuf>, GenerationError> {
    generate_with_options(spec, output_dir, false)
}

pub fn generate_with_options(
    spec: &CompileSpec,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    if !force {
        let mut targets = Vec::new();

        if !spec.services.is_empty() {
            targets.push(output_dir.join("docker-compose.yaml"));
        }

        if let Some(kubernetes) = &spec.kubernetes {
            targets.push(
                output_dir
                    .join("k8s")
                    .join(format!("{}-deployment.yaml", kubernetes.metadata.name)),
            );
        }

        if let Some(cicd) = &spec.cicd {
            let filename = match cicd.provider {
                crate::models::cicd_struct::CiProvider::Github => ".github/workflows/devinit.yml",
                crate::models::cicd_struct::CiProvider::Gitlab => ".gitlab-ci.yml",
            };

            targets.push(output_dir.join(filename));
        }

        for target in targets {
            match target.symlink_metadata() {
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!("Refusing to overwrite {} (use --force)", target.display()),
                    )
                    .into());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
    }

    let mut generated = Vec::new();

    // Generate Compose output when services are configured.
    if !spec.services.is_empty() {
        generated.extend(
            docker_compose_generator::generate_docker_compose_with_options(
                &spec.services,
                output_dir,
                force,
            )?,
        );
    }

    // Generate Kubernetes output when a deployment is configured.
    if let Some(kubernetes) = &spec.kubernetes {
        generated.extend(k8s_generator::generate_k8s_with_options(
            kubernetes, output_dir, force,
        )?);
    }
    // Generate CI/CD output when a pipeline is configured.
    if let Some(cicd) = &spec.cicd {
        generated.extend(cicd_generator::generate_cicd_with_options(
            cicd, output_dir, force,
        )?);
    }

    Ok(generated)
}

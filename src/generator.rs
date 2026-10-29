pub mod cicd_generator;
pub mod docker_compose_generator;
pub mod k8s_generator;
pub mod k8s_service_generator;
use crate::errors::GenerationError;
use crate::generation_plan::GenerationPlan;
use crate::k8s_manifests::{KubernetesManifest, plan_custom_manifests};
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
            let k8s_dir = output_dir.join("k8s");
            let name = &kubernetes.metadata.name;

            targets.push(k8s_dir.join(format!("{name}-deployment.yaml")));

            if crate::generator::k8s_service_generator::build_service(kubernetes).is_some() {
                targets.push(k8s_dir.join(format!("{name}-service.yaml")));
            }
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

/// Builds one generation plan for all supported output formats.
///
/// All YAML documents are rendered in memory.
/// No filesystem writes occur during this operation.
pub fn build_generation_plan(
    spec: &CompileSpec,
    custom_manifests: &[KubernetesManifest],
) -> Result<GenerationPlan, GenerationError> {
    let mut plan = GenerationPlan::new();

    // Docker Compose
    if !spec.services.is_empty() {
        plan.add(docker_compose_generator::plan_docker_compose(
            &spec.services,
        )?)?;
    }

    // Legacy Kubernetes Deployment and optional Service
    if let Some(deployment) = &spec.kubernetes {
        for file in k8s_generator::plan_kubernetes(deployment)? {
            plan.add(file)?;
        }
    }

    // User-defined Kubernetes manifests:
    // both Raw Mode and Template Mode.
    for file in plan_custom_manifests(custom_manifests)? {
        plan.add(file)?;
    }

    // GitHub Actions or GitLab CI
    if let Some(pipeline) = &spec.cicd {
        plan.add(cicd_generator::plan_cicd(pipeline)?)?;
    }

    Ok(plan)
}

#[cfg(test)]
#[path = "tests/generator_tests.rs"]
mod tests;

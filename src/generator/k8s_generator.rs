use crate::errors::GenerationError;
use crate::file_config::yaml_writer::write_yaml_with_options;
use crate::generation_plan::GeneratedFile;
use crate::generation_plan::GenerationPlan;
use crate::generator::k8s_service_generator::build_service;
use crate::k8s_manifests::{KubernetesManifest, plan_custom_manifests};
use crate::models::k8s_struct::Deployment;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Prepares legacy Kubernetes resources without writing files.
///
/// The Deployment is always generated.
/// A Service is generated when the Deployment exposes container ports.
pub fn plan_kubernetes(deployment: &Deployment) -> Result<Vec<GeneratedFile>, GenerationError> {
    let name = &deployment.metadata.name;

    let mut files = Vec::new();

    // Deployment
    let deployment_yaml = serde_yaml::to_string(deployment)?;

    files.push(GeneratedFile::new(
        format!("k8s/{name}-deployment.yaml"),
        deployment_yaml,
    )?);

    // Optional Service
    if let Some(service) = build_service(deployment) {
        let service_yaml = serde_yaml::to_string(&service)?;

        files.push(GeneratedFile::new(
            format!("k8s/{name}-service.yaml"),
            service_yaml,
        )?);
    }

    Ok(files)
}

/// Combines legacy and user-defined Kubernetes manifests.
///
/// Detects duplicate output paths before any filesystem writes.
pub fn plan_all_kubernetes(
    legacy: Option<&Deployment>,
    custom: &[KubernetesManifest],
) -> Result<GenerationPlan, GenerationError> {
    let mut plan = GenerationPlan::new();

    // Legacy-generated Deployment and Service.
    if let Some(deployment) = legacy {
        for file in plan_kubernetes(deployment)? {
            plan.add(file)?;
        }
    }

    // User-defined raw and template resources.
    for file in plan_custom_manifests(custom)? {
        plan.add(file)?;
    }

    Ok(plan)
}

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
    let k8s_dir = output_dir.join("k8s");
    let name = &deployment.metadata.name;

    let deployment_path = k8s_dir.join(format!("{name}-deployment.yaml"));

    let service = build_service(deployment);

    let service_path = service
        .as_ref()
        .map(|_| k8s_dir.join(format!("{name}-service.yaml")));

    // Validate all destination paths before writing any file.
    if !force {
        let targets = std::iter::once(&deployment_path).chain(service_path.iter());

        for path in targets {
            match fs::symlink_metadata(path) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("Refusing to overwrite {} (use --force)", path.display()),
                    )
                    .into());
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
    }

    let mut generated = Vec::new();

    let path = write_yaml_with_options(deployment, &deployment_path, force)?;
    generated.push(path);

    if let (Some(service), Some(service_path)) = (service, service_path) {
        let path = write_yaml_with_options(&service, &service_path, force)?;
        generated.push(path);
    }

    Ok(generated)
}

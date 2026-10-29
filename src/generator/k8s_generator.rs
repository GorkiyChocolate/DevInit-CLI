use crate::errors::GenerationError;
use crate::file_config::yaml_writer::write_yaml_with_options;
use crate::generator::k8s_service_generator::build_service;
use crate::models::k8s_struct::Deployment;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

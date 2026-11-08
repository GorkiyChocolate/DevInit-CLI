use crate::errors::{DevinitError, ValidationError};
use crate::generation_plan::GenerationPlan;
use crate::{api, cli, file_config, generator};
use api::add_recipe::add_recipe;
use api::get_config::get_config;
use cli::commands;
use file_config::yaml_config::load_compile_with_registry;
use serde_json::{Value, json};
use std::fs;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Loads the default compile file and generates output in the current directory.
pub fn compile() -> Result<Vec<PathBuf>, DevinitError> {
    compile_at(Path::new("compile.yaml"), Path::new("."))
}

/// Loads, validates, and generates a compile specification.
pub fn compile_at(input_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DevinitError> {
    compile_at_with_options(input_path, output_dir, false)
}

fn discover_docker_builds(
    project_root: &Path,
) -> Result<Vec<generator::docker_compose_generator::DiscoveredBuild>, DevinitError> {
    use generator::docker_compose_generator::DiscoveredBuild;
    use std::collections::BTreeSet;

    let root = fs::canonicalize(project_root).map_err(DevinitError::FileIOError)?;

    let dockerfiles = crate::docker::docker_finder::find_dockerfiles_in(&root)
        .map_err(DevinitError::FileIOError)?;

    if dockerfiles.is_empty() {
        return Err(DevinitError::ConfigurationError(
            "Docker discovery was requested, but no Dockerfile \
             or Containerfile was found in the project"
                .to_string(),
        ));
    }

    let mut names = BTreeSet::new();
    let mut builds = Vec::new();

    for dockerfile_path in dockerfiles {
        let parent = dockerfile_path.parent().ok_or_else(|| {
            DevinitError::ConfigurationError(format!(
                "Cannot determine directory for {}",
                dockerfile_path.display()
            ))
        })?;

        let relative_dir = parent.strip_prefix(&root).map_err(|_| {
            DevinitError::ConfigurationError(
                "Discovered Dockerfile is outside the project".to_string(),
            )
        })?;

        let dockerfile_name = dockerfile_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                DevinitError::ConfigurationError("Dockerfile name is not valid UTF-8".to_string())
            })?;

        let name_source = if relative_dir.as_os_str().is_empty() {
            root.file_name().ok_or_else(|| {
                DevinitError::ConfigurationError("Cannot determine root service name".to_string())
            })?
        } else {
            relative_dir.file_name().ok_or_else(|| {
                DevinitError::ConfigurationError("Cannot determine service name".to_string())
            })?
        };

        let name = name_source
            .to_str()
            .ok_or_else(|| {
                DevinitError::ConfigurationError(
                    "Service directory name is not valid UTF-8".to_string(),
                )
            })?
            .to_ascii_lowercase();

        let valid_name = !name.is_empty()
            && name.len() <= 80
            && name
                .bytes()
                .next()
                .is_some_and(|value| value.is_ascii_alphanumeric())
            && name
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || value == b'-' || value == b'_');

        if !valid_name {
            return Err(DevinitError::ConfigurationError(format!(
                "Cannot use '{name}' as a Docker service name. \
                     Rename the directory using letters, numbers, \
                     hyphens or underscores."
            )));
        }

        if !names.insert(name.clone()) {
            return Err(DevinitError::ConfigurationError(format!(
                "Multiple Dockerfiles resolve to service \
                     '{name}'. Use a unique service directory \
                     or remove the ambiguous Dockerfile."
            )));
        }

        let context = if relative_dir.as_os_str().is_empty() {
            ".".to_string()
        } else {
            format!("./{}", relative_dir.to_string_lossy().replace('\\', "/"))
        };

        builds.push(DiscoveredBuild {
            name,
            context,
            dockerfile: dockerfile_name.to_string(),
        });
    }

    Ok(builds)
}

/// Loads, validates and renders all infrastructure files in memory.
///
/// This is the shared pipeline for compile, check and dry-run.
fn build_plan(input_path: &Path, output_dir: &Path) -> Result<GenerationPlan, DevinitError> {
    // Resolve services, global presets and CI/CD configuration.
    let spec = load_compile_with_registry(input_path)?;

    let mut builds = if spec.docker_discovery {
        discover_docker_builds(compile_project_root(input_path))?
    } else {
        Vec::new()
    };

    if !builds.is_empty() {
        rebase_docker_builds(&mut builds, compile_project_root(input_path), output_dir)?;
    }

    // Resolve custom Kubernetes manifests and their presets.
    let manifests = crate::k8s_manifests::load_manifests(input_path)?;

    // Validate the specification and dependency graph.
    let ir = crate::ir::ResolvedCompileSpec::build(spec).map_err(DevinitError::ValidationErrors)?;

    // Render generated files without writing them.

    generator::build_generation_plan_with_builds(ir.spec(), &manifests, &builds)
        .map_err(DevinitError::from)
}

/// Validates a project without writing infrastructure files.
pub fn check_at(input_path: &Path) -> Result<usize, DevinitError> {
    let plan = build_plan(input_path, compile_project_root(input_path))?;

    Ok(plan.len())
}

/// Preview status of each generated file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedAction {
    Create,
    Conflict,
    Overwrite,
}

impl PlannedAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "CREATE",
            Self::Conflict => "CONFLICT",
            Self::Overwrite => "OVERWRITE",
        }
    }
}

/// A single file in a dry-run report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub path: PathBuf,
    pub action: PlannedAction,
}

/// Builds a preview without modifying the filesystem.
pub fn dry_run_at(
    input_path: &Path,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PlannedFile>, DevinitError> {
    let plan = build_plan(input_path, output_dir)?;

    // Validate output paths, including symlink protection.
    // force=true here only permits inspection of existing regular
    // files. No writes occur.
    plan.preflight(output_dir, true)?;

    let mut files = Vec::with_capacity(plan.len());

    for file in plan.files() {
        let destination = output_dir.join(&file.relative_path);

        let action = match fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_file() => {
                if force {
                    PlannedAction::Overwrite
                } else {
                    PlannedAction::Conflict
                }
            }

            Ok(_) => {
                return Err(DevinitError::ConfigurationError(format!(
                    "Unsupported output destination: {}",
                    destination.display()
                )));
            }

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => PlannedAction::Create,

            Err(error) => return Err(DevinitError::FileIOError(error)),
        };

        files.push(PlannedFile {
            path: file.relative_path.clone(),
            action,
        });
    }

    Ok(files)
}

/// Loads, validates and generates infrastructure configurations.
pub fn compile_at_with_options(
    input_path: &Path,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, DevinitError> {
    let plan = build_plan(input_path, output_dir)?;

    // Preserve existing overwrite protection.
    plan.preflight(output_dir, force)?;

    // FileWriter still performs its own preflight.
    plan.write(output_dir, force).map_err(DevinitError::from)
}

/// Prints validation errors in a readable command-line format.
fn print_validation_errors(errors: &[ValidationError]) {
    eprintln!("Configuration validation failed:");
    // Print every validation failure with its configuration path.
    for error in errors {
        eprintln!("\n✗ {}\n  {}", error.path, error.message);
    }
}

fn print_json(value: Value) {
    println!("{value}");
}

fn print_json_error(command: &str, configuration: Option<&Path>, error: &DevinitError) {
    let errors: Vec<Value> = match error {
        DevinitError::ValidationErrors(errors) => errors
            .iter()
            .map(|error| {
                json!({
                    "path": error.path,
                    "message": error.message
                })
            })
            .collect(),
        other => vec![json!({
            "path": null,
            "message": other.to_string()
        })],
    };

    print_json(json!({
        "status": "error",
        "command": command,
        "configuration": configuration
            .map(|path| path.to_string_lossy().into_owned()),
        "valid": false,
        "errors": errors
    }));
}

/// Dispatches command-line subcommands to their handlers.
pub fn cli_logic() -> Result<(), DevinitError> {
    let matches = commands::build_cli().get_matches();

    // Execute the handler matching the selected subcommand.
    match matches.subcommand() {
        Some(("download", sub_matches)) => {
            let name = sub_matches
                .get_one::<String>("service")
                .expect("Required service argument")
                .trim()
                .to_ascii_lowercase();

            if crate::downloader::binary::is_supported_tool(&name) {
                let (path, version) = crate::downloader::binary::download(&name)?;

                println!("✓ Download complete");
                println!("✓ Tool: {name}");
                println!("✓ Version: {version}");
                println!("✓ SHA-256 verified");
                println!("✓ Saved to: {}", path.display());
            } else {
                let image = crate::downloader::download(&name)?;

                println!("✓ Download complete");
                println!("✓ Image: {image}");
                println!("✓ Stored in Docker Engine");
            }
        }

        Some(("add", sub_matches)) => {
            let name = sub_matches
                .get_one::<String>("service")
                .expect("Required argument")
                .trim()
                .to_ascii_lowercase();

            let version = sub_matches.get_one::<String>("version");

            let cwd = env::current_dir().map_err(DevinitError::FileIOError)?;

            // 1. Built-in registry works without the backend.
            // 2. Previously downloaded recipes work offline.
            // 3. Unknown recipes are fetched from the backend.
            let requested_version = version.map(String::as_str);

            // Installed YAML recipes have priority over legacy
            // built-in definitions and downloaded recipes.
            let packaged =
                crate::service_registry::packaged_recipe_version(&name, requested_version)
                    .map_err(DevinitError::FileIOError)?;

            let recipe = if let Some(recipe) = packaged {
                recipe
            } else if let Some(recipe) =
                crate::service_registry::cached_recipe_version(&cwd, &name, requested_version)
                    .map_err(DevinitError::FileIOError)?
            {
                recipe
            } else {
                // Keep the existing remote backend implementation below.
                // The existing backend API does not yet support
                // selecting a recipe version.
                if requested_version.is_some() {
                    return Err(DevinitError::ConfigurationError(format!(
                        "Versioned remote recipes are not supported yet: {name}"
                    )));
                }

                let base_url = crate::presets::default_url(&cwd)?;

                let url = format!("{}/services/", base_url.trim_end_matches('/'));

                let recipe = add_recipe(&name, &url).map_err(|e| {
                    DevinitError::ConfigurationError(format!("Unable to fetch {name}: {e}"))
                })?;

                if recipe.name != name {
                    return Err(DevinitError::ConfigurationError(
                        "Backend returned an unexpected service".into(),
                    ));
                }

                recipe
            };

            let cache_path =
                crate::service_registry::cache_recipe_version(&cwd, &recipe, requested_version)
                    .map_err(DevinitError::FileIOError)?;

            println!("✓ Added service: {}", recipe.name);

            if let Some(version) = requested_version {
                println!("✓ Version: {version}");
            }

            println!("✓ Image: {}", recipe.image);
            println!("✓ Cache: {}", cache_path.display());
        }

        Some(("list", sub_matches)) => {
            let json_output = sub_matches.get_flag("json");

            let registry = match crate::service_registry::packaged_definitions() {
                Ok(registry) => registry,
                Err(error) => {
                    let error = DevinitError::FileIOError(error);

                    if json_output {
                        print_json_error("list", None, &error);
                    }

                    return Err(error);
                }
            };

            if json_output {
                let services: Vec<Value> = registry
                    .iter()
                    .map(|(name, service)| {
                        json!({
                            "name": name,
                            "default_version": service.default_version,
                            "description": service.description
                        })
                    })
                    .collect();

                print_json(json!({
                    "status": "success",
                    "command": "list",
                    "services": services,
                    "total": services.len(),
                    "errors": []
                }));
            } else if registry.is_empty() {
                println!("No built-in services found.");
            } else {
                println!("Available DevInit services:\n");

                for (name, service) in &registry {
                    println!(
                        "  {:<16} {:<10} {}",
                        name, service.default_version, service.description
                    );
                }

                println!("\nTotal: {} services", registry.len());
            }
        }

        Some(("get", sub_matches)) => {
            let config_name = sub_matches
                .get_one::<String>("service_url")
                .expect("Required argument")
                .trim()
                .to_ascii_lowercase();

            let cwd = env::current_dir().map_err(DevinitError::FileIOError)?;

            // Validate the requested identifier before accessing the cache.
            let valid_name = !config_name.is_empty()
                && config_name != "."
                && config_name != ".."
                && config_name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.');

            if !valid_name {
                return Err(DevinitError::InvalidInput(format!(
                    "Invalid configuration name: {config_name}"
                )));
            }

            // Check the project-local cache first.
            if let Some(recipe) = crate::service_registry::cached_recipe(&cwd, &config_name)
                .map_err(DevinitError::FileIOError)?
            {
                println!("✓ Configuration found in local cache");
                println!("✓ Service: {}", recipe.name);
                println!("✓ Image: {}", recipe.image);
            } else {
                // Resolve the Backend address.
                let base_url = crate::presets::default_url(&cwd)?;

                let configs_url = format!("{}/configs/", base_url.trim_end_matches('/'));

                println!("Fetching configuration: {config_name}");

                let configs = get_config(&configs_url, &config_name).map_err(|error| {
                    DevinitError::ConfigurationError(format!(
                        "Failed to download '{config_name}': {error}"
                    ))
                })?;

                // Validate the returned recipes before writing.
                for recipe in &configs.configs {
                    if recipe.image.trim().is_empty() {
                        return Err(DevinitError::ConfigurationError(format!(
                            "Backend returned an empty image for '{}'",
                            recipe.name
                        )));
                    }

                    if recipe.name.trim().is_empty() {
                        return Err(DevinitError::ConfigurationError(
                            "Backend returned a recipe without a name".into(),
                        ));
                    }
                }

                // Store downloaded recipes in the local registry.
                for recipe in &configs.configs {
                    let cache_path = crate::service_registry::cache_recipe(&cwd, recipe)
                        .map_err(DevinitError::FileIOError)?;

                    println!("✓ Downloaded: {}", recipe.name);
                    println!("✓ Image: {}", recipe.image);
                    println!("✓ Cache: {}", cache_path.display());
                }

                println!("✓ {} configuration(s) cached", configs.configs.len());
            }
        }

        Some(("login", _)) => {
            return Err(DevinitError::ConfigurationError(
                "Authentication is not available yet. \
                The login command will be enabled after \
                DevInit web service integration."
                    .to_string(),
            ));
        }

        Some(("compile", sub_matches)) => {
            let input = sub_matches
                .get_one::<PathBuf>("file")
                .expect("Input file has a default value")
                .as_path();

            let output = Path::new(".");
            let json_output = sub_matches.get_flag("json");

            let force = sub_matches.get_flag("force");
            let dry_run = sub_matches.get_flag("dry_run");

            if dry_run {
                let files = match dry_run_at(input, output, force) {
                    Ok(files) => files,
                    Err(error) => {
                        if json_output {
                            print_json_error("compile", Some(input), &error);
                        }

                        return Err(error);
                    }
                };

                let mut creates = 0usize;
                let mut conflicts = 0usize;
                let mut overwrites = 0usize;

                for file in &files {
                    match file.action {
                        PlannedAction::Create => creates += 1,
                        PlannedAction::Conflict => conflicts += 1,
                        PlannedAction::Overwrite => overwrites += 1,
                    }
                }

                if json_output {
                    let planned_files: Vec<Value> = files
                        .iter()
                        .map(|file| {
                            json!({
                                "path": file.path.to_string_lossy(),
                                "action": file.action
                                    .as_str()
                                    .to_ascii_lowercase()
                            })
                        })
                        .collect();

                    print_json(json!({
                        "status": "success",
                        "command": "compile",
                        "mode": "dry-run",
                        "configuration": input.to_string_lossy(),
                        "valid": true,
                        "files": planned_files,
                        "summary": {
                            "create": creates,
                            "overwrite": overwrites,
                            "conflict": conflicts,
                            "files_written": 0
                        },
                        "errors": []
                    }));
                } else {
                    println!("DevInit Dry Run");
                    println!("\n✓ Configuration validated");
                    println!("✓ Generation plan created");
                    println!("\nPlanned files:");

                    for file in &files {
                        println!("  {:<10} {}", file.action.as_str(), file.path.display());
                    }

                    println!("\nSummary:");
                    println!("  {} files to create", creates);
                    println!("  {} existing files to overwrite", overwrites);
                    println!("  {} conflicts", conflicts);
                    println!("  0 files written");

                    if conflicts > 0 {
                        println!("\nUse --force to allow overwriting existing files.");
                    }
                }
            } else {
                match compile_at_with_options(input, output, force) {
                    Ok(paths) => {
                        println!("✓ {} loaded", input.display());
                        println!("✓ configuration validated");
                        println!("\nGenerated:");

                        for path in &paths {
                            println!("  {}", path.display());
                        }

                        println!("\n✓ {} files generated", paths.len());
                    }

                    Err(DevinitError::ValidationErrors(errors)) => {
                        print_validation_errors(&errors);
                        return Err(DevinitError::ValidationErrors(errors));
                    }

                    Err(error) => {
                        eprintln!("Compile failed: {error}");
                        return Err(error);
                    }
                }
            }
        }

        Some(("check", sub_matches)) => {
            let input = sub_matches
                .get_one::<PathBuf>("file")
                .expect("Input file has a default value")
                .as_path();

            let json_output = sub_matches.get_flag("json");

            let count = match check_at(input) {
                Ok(count) => count,

                Err(error) => {
                    if json_output {
                        print_json_error("check", Some(input), &error);
                    } else {
                        match &error {
                            DevinitError::ValidationErrors(errors) => {
                                print_validation_errors(errors);
                            }
                            _ => {
                                eprintln!("Check failed: {error}");
                            }
                        }
                    }

                    return Err(error);
                }
            };

            if json_output {
                print_json(json!({
                    "status": "success",
                    "command": "check",
                    "configuration": input.to_string_lossy(),
                    "valid": true,
                    "planned_files": count,
                    "files_written": 0,
                    "errors": []
                }));
            } else {
                println!("DevInit Configuration Check");
                println!("✓ {} loaded", input.display());
                println!("✓ Configuration validated");
                println!("✓ Dependencies validated");
                println!("✓ Generation plan created");

                println!("\nConfiguration is valid.");
                println!("Planned files: {count}");
                println!("Files written: 0");
            }
        }

        Some(("test", _)) => {
            return Err(DevinitError::ConfigurationError(
                "The test command is not implemented yet.".to_string(),
            ));
        }

        Some(("license", _)) => {
            return Err(DevinitError::ConfigurationError(
                "The license command is reserved for a future release.".to_string(),
            ));
        }

        None => {
            return Err(DevinitError::ConfigurationError(
                "No subcommand provided. Use --help for usage instructions.".to_string(),
            ));
        }

        Some((_, _)) => {
            return Err(DevinitError::ConfigurationError(
                "Unknown subcommand. Use --help for usage instructions.".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/cli_logic_tests.rs"]
mod tests;

fn compile_project_root(input_path: &Path) -> &Path {
    input_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn normalized_absolute_path(path: &Path) -> Result<PathBuf, DevinitError> {
    use std::path::Component;

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(DevinitError::FileIOError)?
            .join(path)
    };

    let mut result = PathBuf::new();

    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => {
                result.push(prefix.as_os_str());
            }
            Component::RootDir => {
                result.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(name) => {
                result.push(name);
            }
        }
    }

    Ok(result)
}

/// Returns a relative path from `base_dir` to `target`.
fn relative_path_between(base_dir: &Path, target: &Path) -> Result<PathBuf, DevinitError> {
    use std::path::Component;

    let base = normalized_absolute_path(base_dir)?;
    let target = normalized_absolute_path(target)?;

    let base_parts: Vec<_> = base.components().collect();
    let target_parts: Vec<_> = target.components().collect();

    let common = base_parts
        .iter()
        .zip(target_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut relative = PathBuf::new();

    for component in &base_parts[common..] {
        if matches!(component, Component::Normal(_)) {
            relative.push("..");
        }
    }

    for component in &target_parts[common..] {
        relative.push(component.as_os_str());
    }

    if relative.as_os_str().is_empty() {
        relative.push(".");
    }

    Ok(relative)
}

fn rebase_docker_builds(
    builds: &mut [generator::docker_compose_generator::DiscoveredBuild],
    project_root: &Path,
    output_dir: &Path,
) -> Result<(), DevinitError> {
    for build in builds {
        let source_context = project_root.join(&build.context);

        let relative = relative_path_between(output_dir, &source_context)?;

        let context = relative.to_string_lossy().replace('\\', "/");

        build.context = if context == "." || context.starts_with("../") {
            context
        } else {
            format!("./{context}")
        };
    }

    Ok(())
}
#[cfg(test)]
mod docker_discovery_path_tests {
    use super::*;

    #[test]
    fn relative_compile_file_uses_current_directory() {
        assert_eq!(
            compile_project_root(Path::new("compile.yaml")),
            Path::new(".")
        );
    }

    #[test]
    fn nested_compile_file_uses_parent_directory() {
        assert_eq!(
            compile_project_root(Path::new("project/compile.yaml")),
            Path::new("project")
        );
    }

    #[test]
    fn absolute_compile_file_uses_its_directory() {
        assert_eq!(
            compile_project_root(Path::new("/tmp/project/compile.yaml")),
            Path::new("/tmp/project")
        );
    }
}

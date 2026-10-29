use crate::errors::{DevinitError, ValidationError};
use crate::generation_plan::GenerationPlan;
use crate::{api, cli, file_config, generator};
use std::{
    env,
    path::{Path, PathBuf},
};

use api::add_recipe::add_recipe;
use api::get_config::get_config;
use cli::commands;
use file_config::yaml_config::load_compile_with_registry;
use std::fs;

/// Loads the default compile file and generates output in the current directory.
pub fn compile() -> Result<Vec<PathBuf>, DevinitError> {
    compile_at(Path::new("compile.yaml"), Path::new("."))
}

/// Loads, validates, and generates a compile specification.
pub fn compile_at(input_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DevinitError> {
    compile_at_with_options(input_path, output_dir, false)
}

/// Loads, validates and renders all infrastructure files in memory.
///
/// This is the shared pipeline for compile, check and dry-run.
fn build_plan(input_path: &Path) -> Result<GenerationPlan, DevinitError> {
    // Resolve services, global presets and CI/CD configuration.
    let spec = load_compile_with_registry(input_path)?;

    // Resolve custom Kubernetes manifests and their presets.
    let manifests = crate::k8s_manifests::load_manifests(input_path)?;

    // Validate the specification and dependency graph.
    let ir = crate::ir::ResolvedCompileSpec::build(spec).map_err(DevinitError::ValidationErrors)?;

    // Render generated files without writing them.
    generator::build_generation_plan(ir.spec(), &manifests).map_err(DevinitError::from)
}

/// Validates a project without writing infrastructure files.
pub fn check_at(input_path: &Path) -> Result<usize, DevinitError> {
    let plan = build_plan(input_path)?;

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
    let plan = build_plan(input_path)?;

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
    let plan = build_plan(input_path)?;

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

/// Dispatches command-line subcommands to their handlers.
pub fn cli_logic() -> Result<(), DevinitError> {
    let configs_url = "http://localhost:5167/configs/";
    let matches = commands::build_cli().get_matches();

    // Execute the handler matching the selected subcommand.
    match matches.subcommand() {
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

            let is_builtin = crate::service_registry::builtin_recipe(&name).is_some();

            // An unsupported version of a known built-in service
            // must not silently fall back to the backend.
            if is_builtin
                && crate::service_registry::builtin_recipe_version(
                    &name,
                    version.map(String::as_str),
                )
                .is_none()
            {
                let available = crate::service_registry::supported_versions(&name)
                    .map(|versions| versions.join(", "))
                    .unwrap_or_else(|| "default version only".into());

                return Err(DevinitError::ConfigurationError(format!(
                    "Unsupported version for {name}. Available: {available}"
                )));
            }

            let requested_version = version.map(String::as_str);

            let recipe = if let Some(recipe) =
                crate::service_registry::builtin_recipe_version(&name, requested_version)
            {
                recipe
            } else if let Some(recipe) =
                crate::service_registry::cached_recipe_version(&cwd, &name, requested_version)
                    .map_err(DevinitError::FileIOError)?
            {
                recipe
            } else {
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
            let service_type = sub_matches.get_one::<String>("type");
            let page = sub_matches.get_one::<String>("page");

            println!(
                "Fetching list... Type: {:?}, Page: {:?}",
                service_type, page
            );
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

        Some(("login", sub_matches)) => {
            let config_name = sub_matches
                .get_one::<String>("login_url")
                .expect("Required argument");
            println!("logging in: {}, {:?}", configs_url, config_name);
        }

        Some(("compile", sub_matches)) => {
            let input = Path::new("compile.yaml");
            let output = Path::new(".");

            let force = sub_matches.get_flag("force");
            let dry_run = sub_matches.get_flag("dry_run");

            if dry_run {
                let files = dry_run_at(input, output, force)?;

                println!("DevInit Dry Run");
                println!("\n✓ Configuration validated");
                println!("✓ Generation plan created");
                println!("\nPlanned files:");

                let mut creates = 0usize;
                let mut conflicts = 0usize;
                let mut overwrites = 0usize;

                for file in &files {
                    println!("  {:<10} {}", file.action.as_str(), file.path.display());

                    match file.action {
                        PlannedAction::Create => creates += 1,
                        PlannedAction::Conflict => conflicts += 1,
                        PlannedAction::Overwrite => overwrites += 1,
                    }
                }

                println!("\nSummary:");
                println!("  {} files to create", creates);
                println!("  {} existing files to overwrite", overwrites);
                println!("  {} conflicts", conflicts);
                println!("  0 files written");

                if conflicts > 0 {
                    println!("\nUse --force to allow overwriting existing files.");
                }
            } else {
                match compile_at_with_options(input, output, force) {
                    Ok(paths) => {
                        println!("✓ compile.yaml loaded");
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

        Some(("check", _)) => {
            let count = match check_at(Path::new("compile.yaml")) {
                Ok(count) => count,

                Err(DevinitError::ValidationErrors(errors)) => {
                    print_validation_errors(&errors);
                    return Err(DevinitError::ValidationErrors(errors));
                }

                Err(error) => {
                    eprintln!("Check failed: {error}");
                    return Err(error);
                }
            };

            println!("DevInit Configuration Check");
            println!("\n✓ compile.yaml loaded");
            println!("✓ Configuration validated");
            println!("✓ Dependencies validated");
            println!("✓ Generation plan created");

            println!("\nConfiguration is valid.");
            println!("Planned files: {count}");
            println!("Files written: 0");
        }

        Some(("test", _)) => {}
        _ => {
            println!("No subcommand provided. Use --help for usage instructions.");
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "../tests/cli_logic_tests.rs"]
mod tests;

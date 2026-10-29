use crate::errors::{DevinitError, ValidationError};
use crate::{api, cli, file_config, generator, validator};
use std::{
    env,
    path::{Path, PathBuf},
};

use api::add_recipe::add_recipe;
use api::get_config::get_config;
use cli::commands;
use file_config::env_config::env_file_config;
use file_config::yaml_config::{load_compile_with_registry, yaml_configs_data, yaml_data};

/// Loads the default compile file and generates output in the current directory.
pub fn compile() -> Result<Vec<PathBuf>, DevinitError> {
    compile_at(Path::new("compile.yaml"), Path::new("."))
}

/// Loads, validates, and generates a compile specification.
pub fn compile_at(input_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DevinitError> {
    compile_at_with_options(input_path, output_dir, false)
}

pub fn compile_at_with_options(
    input_path: &Path,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, DevinitError> {
    let spec = load_compile_with_registry(input_path)?;
    // Stop generation when configuration validation fails.
    if let Err(errors) = validator::compile_validator::validate_compile(&spec) {
        return Err(DevinitError::ValidationErrors(errors));
    }
    Ok(generator::generate_with_options(&spec, output_dir, force)?)
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

                let base_url = std::env::var("DEVINIT_API_URL")
                    .unwrap_or_else(|_| "http://localhost:5167".to_string());

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

            let compose_path = cwd.join("compose.yaml.example");
            let env_path = cwd.join("env.example");

            yaml_data(&recipe, &compose_path).map_err(DevinitError::FileIOError)?;

            env_file_config(&recipe, &env_path).map_err(DevinitError::FileIOError)?;

            println!("✓ Added service: {}", recipe.name);
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
                .expect("Required argument");
            println!("Getting repository from: {}", configs_url);

            match get_config(configs_url, config_name) {
                Ok(configs_list) => {
                    println!("Succesfully retrieved configs: {:?}", configs_list);
                    let target_path = env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("compose.yaml.example");

                    // Append fetched configurations to the Compose example file.
                    if let Err(e) = yaml_configs_data(&configs_list, &target_path) {
                        eprintln!("Error updating {}: {}", target_path.display(), e);
                    }
                }
                Err(e) => eprintln!("Error fetching config: {}", e),
            }
        }

        Some(("login", sub_matches)) => {
            let config_name = sub_matches
                .get_one::<String>("login_url")
                .expect("Required argument");
            println!("logging in: {}, {:?}", configs_url, config_name);
        }

        Some(("compile", sub_matches)) => match compile_at_with_options(
            Path::new("compile.yaml"),
            Path::new("."),
            sub_matches.get_flag("force"),
        ) {
            Ok(paths) => {
                println!("✓ compile.yaml loaded");
                println!("✓ configuration validated");
                println!("\nGenerated:");
                // Print every generated path.
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
        },

        Some(("test", _)) => {}
        _ => {
            println!("No subcommand provided. Use --help for usage instructions.");
        }
    }

    Ok(())
}

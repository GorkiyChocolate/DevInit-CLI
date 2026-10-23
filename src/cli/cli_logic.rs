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
use file_config::yaml_config::{load_compile, yaml_configs_data, yaml_data};

/// Loads the default compile file and generates output in the current directory.
pub fn compile() -> Result<Vec<PathBuf>, DevinitError> {
    compile_at(Path::new("compile.yaml"), Path::new("."))
}

/// Loads, validates, and generates a compile specification.
pub fn compile_at(input_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DevinitError> {
    let spec = load_compile(input_path)?;
    // Stop generation when configuration validation fails.
    if let Err(errors) = validator::compile_validator::validate_compile(&spec) {
        return Err(DevinitError::ValidationErrors(errors));
    }
    Ok(generator::generate(&spec, output_dir)?)
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
    let base_url = "http://127.0.0.1:3000/services/";
    let configs_url = "http://127.0.0.1:3000/configs/";
    let matches = commands::build_cli().get_matches();

    // Execute the handler matching the selected subcommand.
    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let recipe_name = sub_matches
                .get_one::<String>("service")
                .expect("Required argument");

            println!("Sending request for recipe: {}", recipe_name);
            match add_recipe(recipe_name, base_url) {
                Ok(recipe) => {
                    println!("Successfully retrieved recipe: {:?}", recipe);
                    let target_path = env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("compose.yaml.example");
                    let env_path = env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("env.example");

                    // Append the recipe to the Compose example file.
                    if let Err(e) = yaml_data(&recipe, &target_path) {
                        eprintln!("Error updating {}: {}", target_path.display(), e);
                    }
                    // Append the recipe environment values to the example file.
                    if let Err(e) = env_file_config(&recipe, &env_path) {
                        eprintln!("Error updating {}: {}", env_path.display(), e);
                    }
                }
                Err(e) => eprintln!("Error fetching recipe: {}", e),
            }
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

        Some(("compile", _)) => match compile() {
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

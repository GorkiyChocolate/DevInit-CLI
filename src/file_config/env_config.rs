use crate::file_config::file_validator::ensure_name_is_new;
use crate::models::docker_compose_struct::RecipeCompose;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Appends a recipe's environment values to an environment file.
pub fn env_file_config(recipe: &RecipeCompose, path: &Path) -> std::io::Result<()> {
    // Skip recipes without environment values.
    let Some(env_values) = recipe.env.as_ref() else {
        return Ok(());
    };

    // Avoid creating empty configuration sections.
    if env_values.is_empty() {
        return Ok(());
    }

    ensure_name_is_new(path, &recipe.name)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    // Separate this section from existing environment entries.
    if path.metadata()?.len() > 0 {
        file.write_all(b"\n")?;
    }

    writeln!(file, "# devinit config: {}", recipe.name)?;
    // Write each environment entry under the recipe marker.
    for value in env_values {
        writeln!(file, "{}", value)?;
    }

    Ok(())
}

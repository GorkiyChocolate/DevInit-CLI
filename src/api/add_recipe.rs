use crate::models::docker_compose_struct::RecipeCompose;
use reqwest::blocking::Client;

/// Fetches one service recipe from the configured API.
pub fn add_recipe(
    recipe_name: &str,
    base_url: &str,
) -> Result<RecipeCompose, Box<dyn std::error::Error>> {
    let url = format!("{}{}", base_url, recipe_name);

    let recipe = Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .send()?
        .error_for_status()?
        .json::<RecipeCompose>()?;

    Ok(recipe)
}

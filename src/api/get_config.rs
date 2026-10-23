use crate::models::docker_compose_struct::ConfigsList;
use reqwest::blocking::Client;

/// Fetches a list of service configurations from the API.
pub fn get_config(
    config_url: &str,
    config_name: &str,
) -> Result<ConfigsList, Box<dyn std::error::Error>> {
    let url = format!("{}{}", config_url, config_name);

    let configs_list = Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .send()?
        .error_for_status()?
        .json::<ConfigsList>()?;

    Ok(configs_list)
}

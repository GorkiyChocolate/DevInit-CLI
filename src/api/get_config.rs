use crate::models::docker_compose_struct::ConfigsList;
use reqwest::Url;
use reqwest::blocking::Client;
use std::time::Duration;

/// Fetches configuration recipes from DevInit Backend.
///
/// Uses the existing endpoint:
/// GET /configs/{name}
pub fn get_config(
    base_url: &str,
    config_name: &str,
) -> Result<ConfigsList, Box<dyn std::error::Error>> {
    if config_name.is_empty()
        || config_name == "."
        || config_name == ".."
        || !config_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    {
        return Err("Invalid configuration name".into());
    }

    let mut url = Url::parse(base_url)?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err("Backend URL must use HTTP or HTTPS".into());
    }

    {
        let mut segments = url.path_segments_mut().map_err(|_| "Invalid Backend URL")?;

        segments.pop_if_empty();
        segments.push(config_name);
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()?;

    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()?
        .error_for_status()?;

    let configs = response.json::<ConfigsList>()?;

    if configs.configs.is_empty() {
        return Err("Backend returned an empty configuration list".into());
    }

    Ok(configs)
}

use reqwest::blocking::Client;

/// Sends a login request and returns the server result.
pub fn login(company_url: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let logining = Client::new()
        .post(company_url)
        .header("Accept", "application/json")
        .send()?
        .error_for_status()?
        .json::<bool>()?;

    Ok(logining)
}

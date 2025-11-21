use super::models::BazaarResponse;
use std::{error::Error, fmt};

const ENDPOINT: &str = "https://api.hypixel.net/v2/skyblock/bazaar";

#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Status(u16),
    ApiUnavailable(String),
    Parse(serde_json::Error),
    Io(std::io::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Http(e) => write!(f, "HTTP error: {e}"),
            ApiError::Status(s) => write!(f, "Unexpected HTTP status: {s}"),
            ApiError::ApiUnavailable(cause) => write!(f, "API unavailable: {cause}"),
            ApiError::Parse(e) => write!(f, "Parse error: {e}"),
            ApiError::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}
impl Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Http(e)
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::Parse(e)
    }
}
impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::Io(e)
    }
}

/// Fetches the full bazaar data from Hypixel API.
pub async fn fetch_bazaar() -> Result<BazaarResponse, ApiError> {
    let resp = reqwest::Client::new().get(ENDPOINT).send().await?;
    let status = resp.status();

    if !status.is_success() {
        return Err(ApiError::Status(status.as_u16()));
    }

    let body = resp.text().await?;
    let response: BazaarResponse = serde_json::from_str(&body)?;

    if !response.success {
        return Err(ApiError::ApiUnavailable(
            response.cause.unwrap_or_else(|| "Unknown error".to_string()),
        ));
    }

    Ok(response)
}

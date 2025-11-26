//! Simple API key authentication middleware for MVP
//!
//! This provides basic security through a shared API key.
//! For production, replace with JWT/OAuth2.

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Configuration for API key authentication
#[derive(Clone)]
pub struct ApiKeyConfig;

impl ApiKeyConfig {
    /// Create a new API key configuration from environment variable
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let api_key = std::env::var("BLIXARD_API_KEY")
            .map_err(|_| anyhow::anyhow!(
                "BLIXARD_API_KEY environment variable not set. \
                Please set it to a secure value (minimum 32 characters)"
            ))?;

        // Basic validation
        if api_key.len() < 32 {
            return Err(anyhow::anyhow!(
                "API key must be at least 32 characters for security"
            ));
        }

        if api_key == "CHANGE_ME_YOUR_SECRET_API_KEY_HERE_MIN_32_CHARS" {
            return Err(anyhow::anyhow!(
                "Please change the default API key in .env"
            ));
        }

        Ok(Self)
    }
}

/// Middleware function to check API key
pub async fn api_key_auth(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get API key from environment on each request for MVP simplicity
    // In production, this should be passed via State
    let expected_key = match std::env::var("BLIXARD_API_KEY") {
        Ok(key) if key.len() >= 32 => key,
        Ok(_) => {
            tracing::error!("API key is too short (< 32 characters)");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(_) => {
            tracing::error!("BLIXARD_API_KEY not set");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Check X-API-Key header
    let provided_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match provided_key {
        Some(key) if key == expected_key => {
            // Valid key - proceed with request
            Ok(next.run(request).await)
        }
        Some(_) => {
            // Invalid key
            tracing::warn!("Invalid API key provided");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            // No key provided
            tracing::debug!("No API key provided in X-API-Key header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Response type for unauthorized requests (with body)
pub struct UnauthorizedResponse;

impl IntoResponse for UnauthorizedResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: Valid X-API-Key header required",
        )
            .into_response()
    }
}
//! Error view model

use askama::Template;

/// View model for error messages
#[derive(Template)]
#[template(path = "partials/error.html")]
pub struct ErrorView {
    pub message: String,
}

impl ErrorView {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

//! View rendering utilities for dashboard handlers
//!
//! Provides helper functions to reduce boilerplate in handlers by centralizing
//! template rendering and error handling logic.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::views::ErrorView;

/// Render a template with automatic error handling
///
/// This helper function:
/// 1. Attempts to render the template
/// 2. Returns HTML on success
/// 3. Logs errors and returns error view on failure
pub fn render_template<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering failed: {}", e);
            render_error("Template rendering error")
        }
    }
}

/// Render a service result with automatic view transformation and error handling
///
/// This helper function:
/// 1. Maps service Result to view model
/// 2. Renders the view
/// 3. Handles errors with error view
pub fn render_service_result<T, V, F>(
    result: Result<T, anyhow::Error>,
    view_mapper: F,
    error_context: &str,
) -> Response
where
    V: Template,
    F: FnOnce(T) -> V,
{
    match result {
        Ok(data) => {
            let view = view_mapper(data);
            render_template(view)
        }
        Err(e) => {
            tracing::error!("{}: {}", error_context, e);
            render_error(error_context)
        }
    }
}

/// Render an error view with fallback to plain text
///
/// Attempts to render the ErrorView template. If that fails,
/// falls back to a plain text error response.
pub fn render_error(message: &str) -> Response {
    let error_view = ErrorView::new(message);
    match error_view.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Failed to render error view: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()).into_response()
        }
    }
}

/// Render an error view with a custom wrapper (e.g., header)
///
/// Useful for HTMX partials that need context around the error
pub fn render_error_with_wrapper(message: &str, wrapper: impl Fn(String) -> String) -> Response {
    let error_view = ErrorView::new(message);
    match error_view.render() {
        Ok(error_html) => {
            let wrapped = wrapper(error_html);
            Html(wrapped).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to render error view: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::Template;

    #[derive(Template)]
    #[template(source = "<p>Test: {{ value }}</p>", ext = "html")]
    struct TestTemplate {
        value: String,
    }

    #[test]
    fn test_render_template_success() {
        let template = TestTemplate {
            value: "hello".to_string(),
        };
        let response = render_template(template);
        // Response should be OK (we can't easily test body without more setup)
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_render_service_result_success() {
        let result: Result<String, anyhow::Error> = Ok("test".to_string());
        let response = render_service_result(
            result,
            |value| TestTemplate { value },
            "Test context",
        );
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_render_service_result_error() {
        let result: Result<String, anyhow::Error> = Err(anyhow::anyhow!("test error"));
        let response = render_service_result(
            result,
            |value| TestTemplate { value },
            "Test error context",
        );
        // Should still return OK with error view
        assert_eq!(response.status(), StatusCode::OK);
    }
}

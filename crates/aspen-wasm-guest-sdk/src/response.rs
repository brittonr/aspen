//! Helper constructors for common `ClientRpcResponse` variants.

use aspen_client_api::ErrorResponse;

/// Create an `ErrorResponse` with the given code and message.
pub fn error_response(code: &str, message: &str) -> ErrorResponse {
    ErrorResponse {
        code: code.to_string(),
        message: message.to_string(),
    }
}

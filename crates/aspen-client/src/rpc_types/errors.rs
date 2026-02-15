// Error response types.

use serde::Deserialize;
use serde::Serialize;

/// Error response for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

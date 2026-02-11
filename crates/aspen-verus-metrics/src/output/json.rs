//! JSON output formatting.

use crate::VerificationReport;

/// Render a report as JSON.
pub fn render(report: &VerificationReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

//! Output formatting for verification reports.

pub mod json;
pub mod terminal;

use crate::VerificationReport;

/// Output format for reports.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output
    #[default]
    Terminal,
    /// JSON output for CI integration
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" | "text" => Ok(Self::Terminal),
            "json" => Ok(Self::Json),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Render a report in the specified format.
pub fn render(report: &VerificationReport, format: OutputFormat, verbose: bool) -> String {
    match format {
        OutputFormat::Terminal => terminal::render(report, verbose),
        OutputFormat::Json => json::render(report),
    }
}

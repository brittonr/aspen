//! Output formatting for verification reports.

pub mod github;
pub mod json;
pub mod markdown;
pub mod terminal;

use crate::Severity;
use crate::VerificationReport;

/// Output format for reports.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output
    #[default]
    Terminal,
    /// JSON output for CI integration
    Json,
    /// GitHub Actions workflow commands
    GithubActions,
    /// Markdown for PR comments
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" | "text" => Ok(Self::Terminal),
            "json" => Ok(Self::Json),
            "github" | "github-actions" | "gha" => Ok(Self::GithubActions),
            "markdown" | "md" => Ok(Self::Markdown),
            _ => Err(format!("Unknown format: '{}'. Valid formats: terminal, json, github, markdown", s)),
        }
    }
}

/// Render a report in the specified format.
pub fn render(report: &VerificationReport, format: OutputFormat, is_verbose: bool, min_severity: Severity) -> String {
    match format {
        OutputFormat::Terminal => terminal::render(report, is_verbose, min_severity),
        OutputFormat::Json => json::render(report, min_severity),
        OutputFormat::GithubActions => github::render(report, min_severity),
        OutputFormat::Markdown => markdown::render(report, min_severity),
    }
}

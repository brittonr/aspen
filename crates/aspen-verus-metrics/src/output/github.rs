//! GitHub Actions workflow command output formatting.
//!
//! Outputs workflow commands that GitHub Actions will parse:
//! - `::error file=...::message` for errors
//! - `::warning file=...::message` for warnings
//! - `::notice file=...::message` for notices
//!
//! See: https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions

use crate::ComparisonResult;
use crate::Severity;
use crate::VerificationReport;

/// Render a report as GitHub Actions workflow commands.
pub fn render(report: &VerificationReport, min_severity: Severity) -> String {
    let mut output = String::new();

    for crate_report in &report.crates {
        for comparison in &crate_report.comparisons {
            let severity = comparison.result.severity();

            if severity < min_severity {
                continue;
            }

            let (level, message) = format_comparison(&comparison.result);

            // Get file path if available
            let file = comparison
                .production_file
                .as_ref()
                .or(comparison.verus_file.as_ref())
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            // Format as GitHub workflow command
            if !file.is_empty() {
                output.push_str(&format!(
                    "::{} file={},title=Verus Sync::{}: {}\n",
                    level, file, comparison.function_name, message
                ));
            } else {
                output.push_str(&format!("::{} title=Verus Sync::{}: {}\n", level, comparison.function_name, message));
            }
        }
    }

    // Add summary as a group
    output.push_str("::group::Verus Sync Summary\n");
    output.push_str(&format!(
        "Crates: {} | Functions: {} | Matches: {} | Drifts: {} | Missing: {} | Coverage: {:.1}%\n",
        report.summary.crates_checked,
        report.summary.functions_compared,
        report.summary.matches,
        report.summary.drifts,
        report.summary.missing_production,
        report.summary.coverage_percent
    ));
    output.push_str("::endgroup::\n");

    // Set output variables for downstream steps
    output.push_str(&format!("::set-output name=coverage::{:.1}\n", report.summary.coverage_percent));
    output.push_str(&format!("::set-output name=drifts::{}\n", report.summary.drifts));
    output.push_str(&format!("::set-output name=has_drift::{}\n", report.has_drift()));

    output
}

/// Format a comparison result for GitHub Actions.
fn format_comparison(result: &ComparisonResult) -> (&'static str, String) {
    match result {
        ComparisonResult::Match => ("notice", "Verified function in sync".to_string()),
        ComparisonResult::SkippedExternalBody => ("notice", "Skipped - marked with external_body".to_string()),
        ComparisonResult::SignatureDrift { production, verus } => (
            "error",
            format!("Signature mismatch. Production params: {:?}, Verus params: {:?}", production.params, verus.params),
        ),
        ComparisonResult::BodyDrift { diff, .. } => {
            // Truncate diff for readability in GitHub UI
            let short_diff = if diff.len() > 200 {
                format!("{}... (truncated)", &diff[..200])
            } else {
                diff.clone()
            };
            ("error", format!("Body mismatch: {}", short_diff.replace('\n', " ")))
        }
        ComparisonResult::MissingProduction {
            verus_function,
            verus_file,
        } => (
            "error",
            format!(
                "Verus function '{}' missing from production (defined in {})",
                verus_function,
                verus_file.display()
            ),
        ),
        ComparisonResult::MissingVerus {
            production_function,
            production_file,
        } => (
            "notice",
            format!(
                "Production function '{}' has no Verus spec (file: {})",
                production_function,
                production_file.display()
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::CrateReport;
    use crate::FunctionComparison;
    use crate::ReportSummary;

    #[test]
    fn test_github_output_format() {
        let report = VerificationReport {
            crates: vec![CrateReport {
                name: "test-crate".to_string(),
                path: PathBuf::from("/test"),
                comparisons: vec![FunctionComparison {
                    function_name: "test_fn".to_string(),
                    production_file: Some(PathBuf::from("src/verified/test.rs")),
                    verus_file: Some(PathBuf::from("verus/test_spec.rs")),
                    result: ComparisonResult::Match,
                }],
                matches: 1,
                drifts: 0,
                missing_production: 0,
                missing_verus: 0,
                skipped: 0,
            }],
            summary: ReportSummary {
                crates_checked: 1,
                functions_compared: 1,
                matches: 1,
                drifts: 0,
                missing_production: 0,
                missing_verus: 0,
                skipped: 0,
                coverage_percent: 100.0,
            },
        };

        let output = render(&report, Severity::Info);
        assert!(output.contains("::notice"));
        assert!(output.contains("::group::"));
        assert!(output.contains("::set-output"));
    }
}

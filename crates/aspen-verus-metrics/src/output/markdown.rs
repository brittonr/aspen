//! Markdown output formatting for PR comments.
//!
//! Generates a nicely formatted Markdown report suitable for
//! posting as a PR comment or saving to a file.

use crate::ComparisonResult;
use crate::Severity;
use crate::VerificationReport;

/// Render a report as Markdown.
pub fn render(report: &VerificationReport, min_severity: Severity) -> String {
    let mut output = String::new();

    // Header with status indicator
    let status_indicator = if report.has_drift() { "DRIFT" } else { "OK" };
    output.push_str(&format!("## [{}] Verus Sync Validation Report\n\n", status_indicator));

    // Summary table
    output.push_str("### Summary\n\n");
    output.push_str("| Metric | Value |\n");
    output.push_str("|--------|-------|\n");
    output.push_str(&format!("| Crates Checked | {} |\n", report.summary.crates_checked));
    output.push_str(&format!("| Functions Compared | {} |\n", report.summary.functions_compared));
    output.push_str(&format!("| Matches | {} |\n", report.summary.matches));
    output.push_str(&format!("| Drifts | {} |\n", report.summary.drifts));
    output.push_str(&format!("| Missing from Production | {} |\n", report.summary.missing_production));
    output.push_str(&format!("| Skipped | {} |\n", report.summary.skipped));
    output.push_str(&format!("| Coverage | **{:.1}%** |\n", report.summary.coverage_percent));
    output.push('\n');

    // Per-crate details
    let has_issues = report.has_issues_at_or_above(Severity::Warning);

    if has_issues {
        output.push_str("### Issues Found\n\n");

        for crate_report in &report.crates {
            let crate_issues: Vec<_> = crate_report
                .comparisons
                .iter()
                .filter(|c| c.result.severity() >= min_severity && c.result.severity() >= Severity::Warning)
                .collect();

            if crate_issues.is_empty() {
                continue;
            }

            output.push_str(&format!("#### `{}`\n\n", crate_report.name));

            for comparison in crate_issues {
                let (icon, description) = format_issue(&comparison.result);
                output.push_str(&format!("- {} **`{}`**: {}\n", icon, comparison.function_name, description));

                // Add file location if available
                if let Some(file) = &comparison.verus_file {
                    output.push_str(&format!("  - Verus: `{}`\n", file.display()));
                }
                if let Some(file) = &comparison.production_file {
                    output.push_str(&format!("  - Production: `{}`\n", file.display()));
                }
            }
            output.push('\n');
        }
    }

    // Detailed diffs in collapsible sections
    let drifts: Vec<_> = report
        .comparisons_at_or_above(Severity::Error)
        .filter(|(_, c)| matches!(c.result, ComparisonResult::BodyDrift { .. }))
        .collect();

    if !drifts.is_empty() {
        output.push_str("### Detailed Diffs\n\n");

        for (crate_report, comparison) in drifts {
            if let ComparisonResult::BodyDrift { diff, .. } = &comparison.result {
                output.push_str(&format!(
                    "<details>\n<summary><code>{}::{}</code></summary>\n\n",
                    crate_report.name, comparison.function_name
                ));
                output.push_str("```diff\n");
                output.push_str(diff);
                if !diff.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("```\n\n");
                output.push_str("</details>\n\n");
            }
        }
    }

    // How to fix section
    if report.has_drift() {
        output.push_str("### How to Fix\n\n");
        output.push_str("1. Update `src/verified/*.rs` to match the logic in `verus/*.rs`\n");
        output.push_str("2. Or update `verus/*.rs` specs to match production code\n");
        output.push_str("3. Run `nix run .#verify-verus` to verify specs are correct\n\n");
    } else {
        output.push_str("---\n\n");
        output.push_str("All verified functions are in sync with their Verus specifications.\n");
    }

    output
}

/// Format an issue for Markdown display.
fn format_issue(result: &ComparisonResult) -> (&'static str, String) {
    match result {
        ComparisonResult::Match => ("", "In sync".to_string()),
        ComparisonResult::SkippedExternalBody => ("", "Skipped (external_body)".to_string()),
        ComparisonResult::SignatureDrift { production, verus } => (
            "",
            format!(
                "Signature mismatch: production has {} params, Verus has {}",
                production.params.len(),
                verus.params.len()
            ),
        ),
        ComparisonResult::BodyDrift { .. } => ("", "Body implementation differs".to_string()),
        ComparisonResult::MissingProduction { .. } => {
            ("", "Verus spec exists but production function missing".to_string())
        }
        ComparisonResult::MissingVerus { .. } => ("", "Production function has no Verus spec".to_string()),
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
    fn test_markdown_output_format() {
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
        assert!(output.contains("## "));
        assert!(output.contains("### Summary"));
        assert!(output.contains("| Coverage |"));
    }
}

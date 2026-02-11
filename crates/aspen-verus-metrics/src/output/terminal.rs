//! Terminal output formatting.

use colored::Colorize;

use crate::ComparisonResult;
use crate::CrateReport;
use crate::Severity;
use crate::VerificationReport;

/// Render a report for terminal display.
pub fn render(report: &VerificationReport, verbose: bool, min_severity: Severity) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n\n", "=== Verus Sync Validation ===".bold()));

    for crate_report in &report.crates {
        output.push_str(&render_crate(crate_report, verbose, min_severity));
    }

    output.push_str(&render_summary(report, min_severity));

    output
}

fn render_crate(report: &CrateReport, verbose: bool, min_severity: Severity) -> String {
    let mut output = String::new();

    output.push_str(&format!("Checking {}...\n", report.name.cyan()));

    let mut has_issues = false;

    for comparison in &report.comparisons {
        let severity = comparison.result.severity();

        // Skip items below minimum severity unless verbose
        if severity < min_severity && !verbose {
            continue;
        }

        match &comparison.result {
            ComparisonResult::Match => {
                if verbose {
                    output.push_str(&format!("  {} {}\n", "OK:".green(), comparison.function_name));
                }
            }
            ComparisonResult::SkippedExternalBody => {
                if verbose {
                    output.push_str(&format!("  {} {} (external_body)\n", "SKIP:".yellow(), comparison.function_name));
                }
            }
            ComparisonResult::SignatureDrift { production, verus } => {
                has_issues = true;
                output.push_str(&format!("  {} {} - signature mismatch\n", "DRIFT:".red(), comparison.function_name));
                if verbose {
                    output.push_str(&format!("    Production: {:?}\n", production.params));
                    output.push_str(&format!("    Verus:      {:?}\n", verus.params));
                }
            }
            ComparisonResult::BodyDrift {
                production_body: _,
                verus_body: _,
                diff,
            } => {
                has_issues = true;
                output.push_str(&format!("  {} {} - body mismatch\n", "DRIFT:".red(), comparison.function_name));
                if verbose {
                    output.push_str("    Diff:\n");
                    for line in diff.lines() {
                        let colored_line = if line.starts_with('-') {
                            line.red().to_string()
                        } else if line.starts_with('+') {
                            line.green().to_string()
                        } else {
                            line.to_string()
                        };
                        output.push_str(&format!("      {}\n", colored_line));
                    }
                }
            }
            ComparisonResult::MissingProduction {
                verus_function,
                verus_file,
            } => {
                has_issues = true;
                output.push_str(&format!("  {} {} - missing from src/verified/\n", "MISSING:".red(), verus_function));
                if verbose {
                    output.push_str(&format!("    Defined in: {}\n", verus_file.display()));
                }
            }
            ComparisonResult::MissingVerus {
                production_function,
                production_file: _,
            } => {
                // This is informational, not an error
                if verbose || min_severity == Severity::Info {
                    output.push_str(&format!(
                        "  {} {} - no Verus spec (may be intentional)\n",
                        "INFO:".blue(),
                        production_function
                    ));
                }
            }
        }
    }

    if !has_issues {
        output.push_str(&format!("  {}\n", "OK".green()));
    }

    output.push('\n');
    output
}

fn render_summary(report: &VerificationReport, min_severity: Severity) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n", "=== Summary ===".bold()));

    let s = &report.summary;

    output.push_str(&format!("Crates checked: {}\n", s.crates_checked.to_string().cyan()));
    output.push_str(&format!("Functions compared: {}\n", s.functions_compared.to_string().cyan()));
    output.push_str(&format!("Matches: {}\n", s.matches.to_string().green()));

    if s.drifts > 0 {
        output.push_str(&format!("Drifts: {}\n", s.drifts.to_string().red()));
    } else {
        output.push_str(&format!("Drifts: {}\n", "0".green()));
    }

    if s.missing_production > 0 {
        output.push_str(&format!("Missing from production: {}\n", s.missing_production.to_string().red()));
    }

    output.push_str(&format!("Skipped: {}\n", s.skipped.to_string().yellow()));
    output.push_str(&format!("Coverage: {:.1}%\n", s.coverage_percent));

    if min_severity < Severity::Error {
        output.push_str(&format!("Minimum severity shown: {}\n", min_severity.to_string().cyan()));
    }

    output.push('\n');

    if report.has_drift() {
        output.push_str(&format!(
            "{}\n\n",
            "DRIFT DETECTED - Production and Verus specs may be out of sync".red().bold()
        ));
        output.push_str("To fix drift:\n");
        output.push_str("  1. Update src/verified/*.rs to match verus/*.rs logic\n");
        output.push_str("  2. Or update verus/*.rs specs to match production code\n");
        output.push_str("  3. Run 'nix run .#verify-verus' to verify specs are correct\n\n");
        output.push_str("Use --verbose to see detailed differences\n");
    } else {
        output.push_str(&format!("{}\n", "All verified functions appear to be in sync with Verus specs".green()));
    }

    output
}

//! JSON output formatting.

use serde_json::json;

use crate::Severity;
use crate::VerificationReport;

/// Render a report as JSON.
pub fn render(report: &VerificationReport, min_severity: Severity) -> String {
    // Filter comparisons by severity
    let filtered_crates: Vec<_> = report
        .crates
        .iter()
        .map(|c| {
            let filtered_comparisons: Vec<_> = c
                .comparisons
                .iter()
                .filter(|comp| comp.result.severity() >= min_severity)
                .map(|comp| {
                    json!({
                        "function_name": comp.function_name,
                        "production_file": comp.production_file,
                        "verus_file": comp.verus_file,
                        "result": comp.result.kind_name(),
                        "severity": comp.result.severity().to_string(),
                    })
                })
                .collect();

            json!({
                "name": c.name,
                "path": c.path,
                "comparisons": filtered_comparisons,
                "matches": c.matches,
                "drifts": c.drifts,
                "missing_production": c.missing_production,
                "missing_verus": c.missing_verus,
                "skipped": c.skipped,
                "coverage_percent": c.coverage_percent(),
            })
        })
        .collect();

    let output = json!({
        "crates": filtered_crates,
        "summary": {
            "crates_checked": report.summary.crates_checked,
            "functions_compared": report.summary.functions_compared,
            "matches": report.summary.matches,
            "drifts": report.summary.drifts,
            "missing_production": report.summary.missing_production,
            "missing_verus": report.summary.missing_verus,
            "skipped": report.summary.skipped,
            "coverage_percent": report.summary.coverage_percent,
        },
        "min_severity": min_severity.to_string(),
    });

    serde_json::to_string_pretty(&output).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

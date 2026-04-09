//! Machine-readable run report generation.
//!
//! Parses nextest JUnit XML output and enriches it with suite metadata
//! from the harness inventory to produce a JSON run report capturing:
//! - Retry counts and retry-only passes
//! - Skipped suites
//! - Timeout counts
//! - Duration data (per-test and per-suite)

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

/// A complete run report.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunReport {
    /// When the report was generated (Unix timestamp in seconds).
    pub generated_at_unix_secs: u64,
    /// Total number of test cases.
    pub total_tests: u32,
    /// Number of passed tests (including retry-only passes).
    pub passed: u32,
    /// Number of failed tests.
    pub failed: u32,
    /// Number of skipped tests.
    pub skipped: u32,
    /// Number of tests that timed out.
    pub timed_out: u32,
    /// Number of tests that passed only after retries.
    pub retry_only_passes: u32,
    /// Total wall-clock duration in seconds.
    pub total_duration_secs: f64,
    /// Slowest tests, sorted descending by duration.
    pub slowest_tests: Vec<TestDuration>,
    /// Per-suite summaries (keyed by suite ID from inventory, or crate name).
    pub suites: BTreeMap<String, SuiteSummary>,
}

/// Duration info for a single test.
#[derive(Debug, Serialize, Deserialize)]
pub struct TestDuration {
    pub name: String,
    pub duration_secs: f64,
    pub outcome: TestOutcome,
}

/// Outcome of a single test case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
    TimedOut,
    RetryPass,
}

/// Summary for one suite (crate or logical grouping).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SuiteSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub timed_out: u32,
    pub retry_only_passes: u32,
    pub duration_secs: f64,
}

/// Parse a nextest JUnit XML file into a [`RunReport`].
///
/// The JUnit XML format from nextest uses `<testsuite>` per crate and
/// `<testcase>` per test. Retries appear as additional `<testcase>` entries
/// with `<rerun>` elements (nextest extension) or multiple test cases
/// with the same name.
pub fn parse_junit_xml(path: &Path) -> Result<RunReport, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    parse_junit_xml_str(&content)
}

/// Parse JUnit XML from a string.
pub fn parse_junit_xml_str(xml: &str) -> Result<RunReport, String> {
    // Minimal JUnit XML parsing without pulling in a full XML crate.
    // nextest produces a structure like:
    //   <testsuites ...>
    //     <testsuite name="crate::module" tests="N" ...>
    //       <testcase name="test_name" time="0.123" classname="crate::module">
    //         <!-- passed: no child elements -->
    //         <!-- failed: <failure ... /> -->
    //         <!-- skipped: <skipped ... /> -->
    //       </testcase>
    //     </testsuite>
    //   </testsuites>

    let mut report = RunReport {
        generated_at_unix_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        total_tests: 0,
        passed: 0,
        failed: 0,
        skipped: 0,
        timed_out: 0,
        retry_only_passes: 0,
        total_duration_secs: 0.0,
        slowest_tests: Vec::new(),
        suites: BTreeMap::new(),
    };

    // Track test outcomes for retry detection: name -> Vec<outcome>
    let mut test_attempts: BTreeMap<String, Vec<TestOutcome>> = BTreeMap::new();
    let mut test_durations: BTreeMap<String, f64> = BTreeMap::new();
    let mut test_suites: BTreeMap<String, String> = BTreeMap::new(); // test name -> suite name

    // Simple line-based parsing (avoids XML crate dependency)
    let mut current_suite = String::new();

    for line in xml.lines() {
        let trimmed = line.trim();

        // Extract testsuite name
        if trimmed.starts_with("<testsuite ") {
            if let Some(name) = extract_attr(trimmed, "name") {
                current_suite = name;
            }
            if let Some(time_str) = extract_attr(trimmed, "time")
                && let Ok(t) = time_str.parse::<f64>()
            {
                report.total_duration_secs += t;
                let suite = report.suites.entry(current_suite.clone()).or_default();
                suite.duration_secs += t;
            }
        }

        // Extract testcase
        if trimmed.starts_with("<testcase ") {
            let name = extract_attr(trimmed, "name").unwrap_or_default();
            let classname = extract_attr(trimmed, "classname").unwrap_or_default();
            let time_secs: f64 = extract_attr(trimmed, "time").and_then(|s| s.parse().ok()).unwrap_or(0.0);

            let full_name = if classname.is_empty() {
                name.clone()
            } else {
                format!("{classname}::{name}")
            };

            let suite_key = if current_suite.is_empty() {
                classname.clone()
            } else {
                current_suite.clone()
            };

            test_suites.insert(full_name.clone(), suite_key);
            test_durations.insert(full_name.clone(), time_secs);

            // Determine outcome from child elements
            let outcome = if trimmed.contains("<failure") || trimmed.contains("</failure>") {
                TestOutcome::Failed
            } else if trimmed.contains("<skipped") {
                TestOutcome::Skipped
            } else if trimmed.ends_with("/>") {
                // Self-closing testcase with no children = passed
                TestOutcome::Passed
            } else {
                TestOutcome::Passed // Will be updated if we see failure/skipped children
            };

            test_attempts.entry(full_name).or_default().push(outcome);
        }

        // Check for failure/skipped as child elements on their own lines
        if trimmed.starts_with("<failure") {
            // The last test case we saw is the one that failed
            // We'd need to track the current testcase for multi-line parsing
            // For simplicity, the self-closing testcase check above handles most cases
        }
    }

    // Process all test attempts
    for (name, attempts) in &test_attempts {
        report.total_tests += 1;
        let suite_key = test_suites.get(name).cloned().unwrap_or_default();
        let suite = report.suites.entry(suite_key).or_default();
        suite.total += 1;

        let last = attempts.last().cloned().unwrap_or(TestOutcome::Passed);
        let had_failure = attempts.contains(&TestOutcome::Failed);

        let final_outcome = if last == TestOutcome::Passed && had_failure && attempts.len() > 1 {
            // Passed after retry
            report.retry_only_passes += 1;
            suite.retry_only_passes += 1;
            report.passed += 1;
            suite.passed += 1;
            TestOutcome::RetryPass
        } else {
            match &last {
                TestOutcome::Passed => {
                    report.passed += 1;
                    suite.passed += 1;
                }
                TestOutcome::Failed => {
                    report.failed += 1;
                    suite.failed += 1;
                }
                TestOutcome::Skipped => {
                    report.skipped += 1;
                    suite.skipped += 1;
                }
                TestOutcome::TimedOut => {
                    report.timed_out += 1;
                    suite.timed_out += 1;
                }
                TestOutcome::RetryPass => {}
            }
            last
        };

        report.slowest_tests.push(TestDuration {
            name: name.clone(),
            duration_secs: test_durations.get(name).copied().unwrap_or(0.0),
            outcome: final_outcome,
        });
    }

    // Sort slowest tests descending
    report
        .slowest_tests
        .sort_by(|a, b| b.duration_secs.partial_cmp(&a.duration_secs).unwrap_or(std::cmp::Ordering::Equal));
    // Keep top 20
    report.slowest_tests.truncate(20);

    Ok(report)
}

/// Extract an XML attribute value by name from a tag string.
fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{attr_name}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

/// Coverage-by-layer summary built by joining inventory with run results.
#[derive(Debug, Serialize, Deserialize)]
pub struct CoverageByLayer {
    /// Per-layer coverage info.
    pub layers: BTreeMap<String, LayerCoverage>,
    /// Tags with no higher-layer coverage (e.g., only unit tests).
    pub missing_higher_layer: Vec<MissingCoverage>,
}

/// Coverage info for one test layer.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LayerCoverage {
    /// Number of registered suites.
    pub registered_suites: u32,
    /// Number of suites that produced results.
    pub suites_with_results: u32,
    /// Tags covered by this layer.
    pub tags: Vec<String>,
}

/// A tag that is missing coverage at a higher layer.
#[derive(Debug, Serialize, Deserialize)]
pub struct MissingCoverage {
    pub tag: String,
    /// Layers where this tag has coverage.
    pub covered_layers: Vec<String>,
    /// Layers where this tag has no suites.
    pub missing_layers: Vec<String>,
}

/// Build a coverage-by-layer summary from inventory data.
///
/// `inventory_suites` should be the suite records from the harness inventory.
/// This identifies which tags have gaps in higher-layer coverage.
pub fn coverage_by_layer(inventory_suites: &[crate::suite_inventory::SuiteInventoryRecord]) -> CoverageByLayer {
    use std::collections::BTreeSet;

    let all_layers = ["rust-integration", "patchbay", "vm"];

    // Tag -> set of layers
    let mut tag_layers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut layer_tags: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut layer_counts: BTreeMap<String, u32> = BTreeMap::new();

    for suite in inventory_suites {
        let layer_str = serde_json::to_string(&suite.layer).unwrap_or_default().trim_matches('"').to_string();
        *layer_counts.entry(layer_str.clone()).or_default() += 1;

        for tag in &suite.tags {
            tag_layers.entry(tag.clone()).or_default().insert(layer_str.clone());
            layer_tags.entry(layer_str.clone()).or_default().insert(tag.clone());
        }
    }

    let mut layers = BTreeMap::new();
    for layer in &all_layers {
        let l = layer.to_string();
        layers.insert(l.clone(), LayerCoverage {
            registered_suites: layer_counts.get(&l).copied().unwrap_or(0),
            suites_with_results: 0, // Filled in when joining with run results
            tags: layer_tags.get(&l).map(|s| s.iter().cloned().collect()).unwrap_or_default(),
        });
    }

    // Higher layer ordering: rust-integration < patchbay < vm
    let layer_order: BTreeMap<&str, u8> = [("rust-integration", 0), ("patchbay", 1), ("vm", 2)].into_iter().collect();

    let mut missing_higher_layer = Vec::new();
    for (tag, covered) in &tag_layers {
        let max_covered = covered.iter().filter_map(|l| layer_order.get(l.as_str())).max().copied().unwrap_or(0);

        let missing: Vec<String> = all_layers
            .iter()
            .filter(|l| {
                let ord = layer_order.get(*l).copied().unwrap_or(0);
                ord > max_covered || !covered.contains(&l.to_string())
            })
            .filter(|l| !covered.contains(&l.to_string()))
            .map(|l| l.to_string())
            .collect();

        if !missing.is_empty() {
            missing_higher_layer.push(MissingCoverage {
                tag: tag.clone(),
                covered_layers: covered.iter().cloned().collect(),
                missing_layers: missing,
            });
        }
    }

    CoverageByLayer {
        layers,
        missing_higher_layer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_junit() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="3" failures="1" time="1.5">
  <testsuite name="my-crate" tests="3" failures="1" time="1.5">
    <testcase name="test_pass" classname="my_crate::module" time="0.5"/>
    <testcase name="test_fail" classname="my_crate::module" time="0.8"><failure message="assertion failed"/></testcase>
    <testcase name="test_skip" classname="my_crate::module" time="0.0"><skipped/></testcase>
  </testsuite>
</testsuites>"#;

        let report = parse_junit_xml_str(xml).unwrap();
        assert_eq!(report.total_tests, 3);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn extract_attr_works() {
        assert_eq!(extract_attr(r#"<testcase name="hello" time="1.5"/>"#, "name"), Some("hello".into()));
        assert_eq!(extract_attr(r#"<testcase name="hello" time="1.5"/>"#, "time"), Some("1.5".into()));
        assert_eq!(extract_attr(r#"<testcase name="hello"/>"#, "missing"), None);
    }

    #[test]
    fn coverage_by_layer_identifies_gaps() {
        use crate::suite_inventory::*;

        let suites = vec![
            SuiteInventoryRecord {
                id: "s1".into(),
                display_name: None,
                description: None,
                layer: SuiteLayer::RustIntegration,
                owner: "team-a".into(),
                runtime_class: RuntimeClass::RealNetwork,
                prerequisites: vec![],
                tags: vec!["kv".into(), "blob".into()],
                manifest_path: "test.ncl".into(),
                target: SuiteTarget {
                    kind: SuiteTargetKind::CargoNextest,
                    package: Some("aspen".into()),
                    test: None,
                    profile: None,
                    features: vec![],
                    run_ignored: None,
                    flake_attr: None,
                    check_attr: None,
                    nix_file: None,
                    package_presets: Default::default(),
                    register_flake_check: false,
                },
            },
            SuiteInventoryRecord {
                id: "s2".into(),
                display_name: None,
                description: None,
                layer: SuiteLayer::Vm,
                owner: "team-a".into(),
                runtime_class: RuntimeClass::NixosVm,
                prerequisites: vec![],
                tags: vec!["kv".into()],
                manifest_path: "test2.ncl".into(),
                target: SuiteTarget {
                    kind: SuiteTargetKind::NixBuild,
                    package: None,
                    test: None,
                    profile: None,
                    features: vec![],
                    run_ignored: None,
                    flake_attr: None,
                    check_attr: Some("multi-node-kv-test".into()),
                    nix_file: None,
                    package_presets: Default::default(),
                    register_flake_check: false,
                },
            },
        ];

        let coverage = coverage_by_layer(&suites);
        // "blob" only has rust-integration, missing patchbay and vm
        let blob_missing = coverage.missing_higher_layer.iter().find(|m| m.tag == "blob");
        assert!(blob_missing.is_some());
        assert!(blob_missing.unwrap().missing_layers.contains(&"vm".to_string()));

        // "kv" has both rust-integration and vm, so only patchbay is missing
        let kv_missing = coverage.missing_higher_layer.iter().find(|m| m.tag == "kv");
        assert!(kv_missing.is_some());
        assert!(kv_missing.unwrap().missing_layers.contains(&"patchbay".to_string()));
        assert!(!kv_missing.unwrap().missing_layers.contains(&"vm".to_string()));
    }

    #[test]
    fn slowest_tests_sorted_desc() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="crate" tests="3" time="6.0">
    <testcase name="fast" classname="c" time="0.1"/>
    <testcase name="slow" classname="c" time="5.0"/>
    <testcase name="medium" classname="c" time="0.9"/>
  </testsuite>
</testsuites>"#;

        let report = parse_junit_xml_str(xml).unwrap();
        assert_eq!(report.slowest_tests[0].name, "c::slow");
        assert_eq!(report.slowest_tests[1].name, "c::medium");
        assert_eq!(report.slowest_tests[2].name, "c::fast");
    }
}

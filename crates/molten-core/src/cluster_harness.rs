//! Pure receipt-first cluster run-directory assessment.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub const RUN_DIRECTORY_PASS: &str = "pass";
pub const RUN_DIRECTORY_DENY: &str = "deny";
pub const ARTIFACT_FORMAT_PRESERVES: &str = "preserves";
pub const ARTIFACT_FORMAT_TEXT: &str = "text";
pub const BLAKE3_CONTENT_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;
const BLAKE3_CONTENT_REF_LENGTH: usize = BLAKE3_CONTENT_REF_PREFIX.len() + BLAKE3_HEX_LENGTH;
const MAX_RUN_ARTIFACTS: usize = 4_096;
const _: () = assert!(MAX_RUN_ARTIFACTS > 0);

pub const REQUIRED_CLUSTER_RUN_ARTIFACT_KINDS: &[&str] = &[
    "cluster-harness-fixture-metadata",
    "cluster-harness-command-plan",
    "local-multiprocess-plan",
    "local-multiprocess-executable-run",
    "cluster-lifecycle-run",
    "cluster-harness-drift-summary",
    "cluster-harness-cleanup",
    "cluster-harness-run",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunArtifactIndexEntry {
    pub relative_path: String,
    pub artifact_kind: String,
    pub expected_ref: String,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunArtifactObservation {
    pub relative_path: String,
    pub artifact_kind: String,
    pub observed_ref: Option<String>,
    pub format: String,
    pub canonical: bool,
    pub pass_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstDivergence {
    pub relative_path: String,
    pub artifact_kind: String,
    pub expected: String,
    pub observed: String,
    pub reason: String,
    pub diagnostic_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDirectoryAssessment {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub first_divergence: Option<FirstDivergence>,
}

// r[impl molten.testing.receipt_first_cluster_harness.run_artifact_directory]
// r[impl molten.testing.receipt_first_cluster_harness.failure_triage]
pub fn assess_run_directory(
    entries: &[RunArtifactIndexEntry],
    observations: &[RunArtifactObservation],
) -> RunDirectoryAssessment {
    let mut diagnostics = Vec::new();
    let mut first_divergence = None;
    if entries.is_empty() {
        push_diagnostic(&mut diagnostics, "cluster-run-index-empty");
    }
    if entries.len() > MAX_RUN_ARTIFACTS {
        push_diagnostic(&mut diagnostics, format!("cluster-run-index-too-large:{}", entries.len()));
    }
    let observation_by_path = collect_observations(observations, &mut diagnostics, &mut first_divergence);
    let mut indexed_paths = BTreeSet::new();
    let mut indexed_kinds = BTreeSet::new();
    let mut previous_path: Option<&str> = None;

    for entry in entries.iter().take(MAX_RUN_ARTIFACTS) {
        validate_index_entry(entry, previous_path, &mut diagnostics, &mut first_divergence);
        previous_path = Some(&entry.relative_path);
        if !indexed_paths.insert(entry.relative_path.as_str()) {
            record_divergence(
                &mut diagnostics,
                &mut first_divergence,
                entry,
                &entry.expected_ref,
                "duplicate-index-path",
            );
        }
        indexed_kinds.insert(entry.artifact_kind.as_str());
        assess_indexed_artifact(
            entry,
            observation_by_path.get(entry.relative_path.as_str()).copied(),
            &mut diagnostics,
            &mut first_divergence,
        );
    }

    for required_kind in REQUIRED_CLUSTER_RUN_ARTIFACT_KINDS {
        if !indexed_kinds.contains(required_kind) {
            let diagnostic = format!("cluster-run-missing-required-kind:{required_kind}");
            push_diagnostic(&mut diagnostics, &diagnostic);
            if first_divergence.is_none() {
                first_divergence = Some(FirstDivergence {
                    relative_path: "<index>".to_string(),
                    artifact_kind: (*required_kind).to_string(),
                    expected: "present".to_string(),
                    observed: "missing".to_string(),
                    reason: diagnostic,
                    diagnostic_only: true,
                });
            }
        }
    }
    for observation in observations {
        if !indexed_paths.contains(observation.relative_path.as_str()) {
            let diagnostic = format!("cluster-run-unexpected-artifact:{}", observation.relative_path);
            push_diagnostic(&mut diagnostics, &diagnostic);
            if first_divergence.is_none() {
                first_divergence = Some(FirstDivergence {
                    relative_path: observation.relative_path.clone(),
                    artifact_kind: observation.artifact_kind.clone(),
                    expected: "absent".to_string(),
                    observed: observation.observed_ref.clone().unwrap_or_else(|| "unreadable".to_string()),
                    reason: diagnostic,
                    diagnostic_only: true,
                });
            }
        }
    }

    diagnostics.sort();
    diagnostics.dedup();
    RunDirectoryAssessment {
        decision: if diagnostics.is_empty() {
            RUN_DIRECTORY_PASS.to_string()
        } else {
            RUN_DIRECTORY_DENY.to_string()
        },
        diagnostics,
        first_divergence,
    }
}

fn collect_observations<'a>(
    observations: &'a [RunArtifactObservation],
    diagnostics: &mut Vec<String>,
    first_divergence: &mut Option<FirstDivergence>,
) -> BTreeMap<&'a str, &'a RunArtifactObservation> {
    let mut by_path = BTreeMap::new();
    for observation in observations.iter().take(MAX_RUN_ARTIFACTS) {
        if by_path.insert(observation.relative_path.as_str(), observation).is_some() {
            let diagnostic = format!("cluster-run-duplicate-observation:{}", observation.relative_path);
            push_diagnostic(diagnostics, &diagnostic);
            if first_divergence.is_none() {
                *first_divergence = Some(FirstDivergence {
                    relative_path: observation.relative_path.clone(),
                    artifact_kind: observation.artifact_kind.clone(),
                    expected: "one-observation".to_string(),
                    observed: "duplicate-observation".to_string(),
                    reason: diagnostic,
                    diagnostic_only: true,
                });
            }
        }
    }
    if observations.len() > MAX_RUN_ARTIFACTS {
        push_diagnostic(diagnostics, format!("cluster-run-observation-set-too-large:{}", observations.len()));
    }
    by_path
}

fn validate_index_entry(
    entry: &RunArtifactIndexEntry,
    previous_path: Option<&str>,
    diagnostics: &mut Vec<String>,
    first_divergence: &mut Option<FirstDivergence>,
) {
    let invalid_path = !safe_relative_path(&entry.relative_path);
    if invalid_path {
        record_divergence(diagnostics, first_divergence, entry, &entry.relative_path, "unsafe-relative-path");
    }
    if previous_path.is_some_and(|previous| previous >= entry.relative_path.as_str()) {
        record_divergence(diagnostics, first_divergence, entry, &entry.relative_path, "index-not-sorted");
    }
    if entry.artifact_kind.trim().is_empty() || entry.artifact_kind.trim() != entry.artifact_kind {
        record_divergence(diagnostics, first_divergence, entry, &entry.artifact_kind, "invalid-artifact-kind");
    }
    if !valid_content_ref(&entry.expected_ref) {
        record_divergence(diagnostics, first_divergence, entry, &entry.expected_ref, "invalid-expected-ref");
    }
    if !matches!(entry.format.as_str(), ARTIFACT_FORMAT_PRESERVES | ARTIFACT_FORMAT_TEXT) {
        record_divergence(diagnostics, first_divergence, entry, &entry.format, "unsupported-artifact-format");
    }
}

fn assess_indexed_artifact(
    entry: &RunArtifactIndexEntry,
    observation: Option<&RunArtifactObservation>,
    diagnostics: &mut Vec<String>,
    first_divergence: &mut Option<FirstDivergence>,
) {
    let Some(observation) = observation else {
        record_divergence(diagnostics, first_divergence, entry, "missing", "missing-artifact");
        return;
    };
    if observation.artifact_kind != entry.artifact_kind {
        record_divergence(diagnostics, first_divergence, entry, &observation.artifact_kind, "artifact-kind-mismatch");
    }
    if observation.format != entry.format {
        record_divergence(diagnostics, first_divergence, entry, &observation.format, "artifact-format-mismatch");
    }
    if !observation.canonical {
        record_divergence(diagnostics, first_divergence, entry, "non-canonical", "non-canonical-artifact");
    }
    match observation.observed_ref.as_deref() {
        Some(observed_ref) if observed_ref == entry.expected_ref => {}
        Some(observed_ref) => {
            record_divergence(diagnostics, first_divergence, entry, observed_ref, "content-ref-mismatch");
        }
        None => record_divergence(diagnostics, first_divergence, entry, "unreadable", "unreadable-artifact"),
    }
    if !observation.pass_eligible {
        record_divergence(diagnostics, first_divergence, entry, "deny", "artifact-not-pass-eligible");
    }
}

fn record_divergence(
    diagnostics: &mut Vec<String>,
    first_divergence: &mut Option<FirstDivergence>,
    entry: &RunArtifactIndexEntry,
    observed: &str,
    reason: &str,
) {
    let diagnostic = format!("cluster-run-{reason}:{}", entry.relative_path);
    push_diagnostic(diagnostics, &diagnostic);
    if first_divergence.is_none() {
        *first_divergence = Some(FirstDivergence {
            relative_path: entry.relative_path.clone(),
            artifact_kind: entry.artifact_kind.clone(),
            expected: entry.expected_ref.clone(),
            observed: observed.to_string(),
            reason: diagnostic,
            diagnostic_only: true,
        });
    }
}

fn push_diagnostic(diagnostics: &mut Vec<String>, diagnostic: impl Into<String>) {
    if diagnostics.len() < MAX_RUN_ARTIFACTS {
        diagnostics.push(diagnostic.into());
    }
}

fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.ends_with('/')
        && !path.contains('\t')
        && !path.contains('\n')
        && path.split('/').all(|component| !component.is_empty() && component != "." && component != "..")
}

fn valid_content_ref(reference: &str) -> bool {
    reference.len() == BLAKE3_CONTENT_REF_LENGTH
        && reference.starts_with(BLAKE3_CONTENT_REF_PREFIX)
        && reference[BLAKE3_CONTENT_REF_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(label: &str) -> String {
        const HEX_RADIX: u8 = 16;
        let digit = label.bytes().fold(0_u8, u8::wrapping_add) % HEX_RADIX;
        let hex = format!("{digit:x}");
        format!("{BLAKE3_CONTENT_REF_PREFIX}{}", hex.repeat(BLAKE3_HEX_LENGTH))
    }

    fn complete_entries() -> Vec<RunArtifactIndexEntry> {
        let mut entries = REQUIRED_CLUSTER_RUN_ARTIFACT_KINDS
            .iter()
            .enumerate()
            .map(|(index, kind)| RunArtifactIndexEntry {
                relative_path: format!("artifacts/{index:02}-{kind}.preserves"),
                artifact_kind: (*kind).to_string(),
                expected_ref: reference(kind),
                format: ARTIFACT_FORMAT_PRESERVES.to_string(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        entries
    }

    fn matching_observations(entries: &[RunArtifactIndexEntry]) -> Vec<RunArtifactObservation> {
        entries
            .iter()
            .map(|entry| RunArtifactObservation {
                relative_path: entry.relative_path.clone(),
                artifact_kind: entry.artifact_kind.clone(),
                observed_ref: Some(entry.expected_ref.clone()),
                format: entry.format.clone(),
                canonical: true,
                pass_eligible: true,
            })
            .collect()
    }

    #[test]
    fn complete_canonical_run_directory_passes_offline_assessment() {
        // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
        let entries = complete_entries();
        let assessment = assess_run_directory(&entries, &matching_observations(&entries));

        assert_eq!(assessment.decision, RUN_DIRECTORY_PASS);
        assert!(assessment.diagnostics.is_empty());
        assert!(assessment.first_divergence.is_none());
    }

    #[test]
    fn tamper_missing_kind_and_deny_artifact_report_first_divergence() {
        // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
        // r[verify molten.testing.receipt_first_cluster_harness.failure_triage]
        let mut entries = complete_entries();
        entries.remove(0);
        let mut observations = matching_observations(&entries);
        observations[0].observed_ref = Some(reference("tampered"));
        observations[0].pass_eligible = false;
        let assessment = assess_run_directory(&entries, &observations);

        assert_eq!(assessment.decision, RUN_DIRECTORY_DENY);
        assert!(assessment.diagnostics.iter().any(|item| item.contains("missing-required-kind")));
        assert!(assessment.diagnostics.iter().any(|item| item.contains("content-ref-mismatch")));
        assert!(assessment.diagnostics.iter().any(|item| item.contains("artifact-not-pass-eligible")));
        let divergence = assessment.first_divergence.expect("first divergence");
        assert!(divergence.diagnostic_only);
        assert_eq!(divergence.reason, format!("cluster-run-content-ref-mismatch:{}", entries[0].relative_path));
    }

    #[test]
    fn unsafe_paths_and_unexpected_observations_are_denied() {
        // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
        let mut entries = complete_entries();
        entries[0].relative_path = "../escape.preserves".to_string();
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        let mut observations = matching_observations(&entries);
        observations.push(RunArtifactObservation {
            relative_path: "extra.preserves".to_string(),
            artifact_kind: "unexpected".to_string(),
            observed_ref: Some(reference("unexpected")),
            format: ARTIFACT_FORMAT_PRESERVES.to_string(),
            canonical: true,
            pass_eligible: true,
        });
        let assessment = assess_run_directory(&entries, &observations);

        assert_eq!(assessment.decision, RUN_DIRECTORY_DENY);
        assert!(assessment.diagnostics.iter().any(|item| item.contains("unsafe-relative-path")));
        assert!(assessment.diagnostics.iter().any(|item| item.contains("unexpected-artifact")));
    }
}

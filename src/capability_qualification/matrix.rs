//! Test-only readback of the checked-in `tests/evidence-matrix.ncl` rows, so
//! tests can require that each row's refs are exactly what its named test or
//! producer actually computes. The Nickel source keeps one field per line.

const MATRIX: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/evidence-matrix.ncl"));

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Entry {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub evidence_scope: String,
    pub target: String,
    pub command: String,
    pub artifact_refs: Vec<String>,
    pub receipt_ref: String,
}

pub(crate) fn entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    for line in MATRIX.lines().map(str::trim) {
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let value = value.trim_end_matches(',');
        if key == "requirement_id" {
            entries.extend(current.take());
            current = Some(Entry::default());
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        match key {
            "requirement_id" => entry.requirement_id = unquote(value),
            "coverage_kind" => entry.coverage_kind = unquote(value),
            "evidence_scope" => entry.evidence_scope = unquote(value),
            "target" => entry.target = unquote(value),
            "command" => entry.command = unquote(value),
            "artifact_refs" => {
                entry.artifact_refs = value
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(unquote)
                    .collect();
            }
            "receipt_ref" => entry.receipt_ref = unquote(value),
            _ => {}
        }
    }
    entries.extend(current);
    entries
}

/// The single checked-in row whose command names `command_fragment`.
pub(crate) fn entry(command_fragment: &str) -> Entry {
    let matches: Vec<_> = entries().into_iter().filter(|entry| entry.command.contains(command_fragment)).collect();
    assert_eq!(matches.len(), 1, "exactly one evidence-matrix row must name {command_fragment}");
    matches.into_iter().next().expect("one row")
}

/// The checked-in row of one native producer scenario.
pub(crate) fn scenario_entry(scenario: super::Scenario) -> Entry {
    entry(&format!("--scenario {} ", scenario.id()))
}

/// Run one fixed scenario once through the actual runtime path and require
/// that it passes, reads back, and reproduces exactly the runtime-artifact ref
/// checked into its matrix row. The row was computed by earlier independent
/// executions, so equality is also the cross-process determinism check.
pub(crate) fn assert_scenario_bound(scenario: super::Scenario) -> preserves::IOValue {
    let programs = super::Programs::test_harness();
    let (artifact, decision) = super::runtime_artifact(scenario, &programs).expect("actual runtime execution");
    assert_eq!(
        decision,
        "pass",
        "{} artifact: {}",
        scenario.id(),
        crate::preserves_rail::to_text(&artifact).unwrap_or_default()
    );
    if !scenario.is_harness() {
        super::products::verify_observation(scenario, &artifact).expect("product observation readback");
    }
    let actual = crate::preserves_rail::canonical_hash(&artifact).expect("runtime artifact ref");
    assert_eq!(
        scenario_entry(scenario).receipt_ref,
        actual,
        "{} runtime artifact differs from its checked-in evidence-matrix row",
        scenario.id()
    );
    artifact
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

use std::collections::BTreeMap;

use super::{artifacts, model::*, report};

fn reference(label: &str) -> String {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::string(label)).expect("test ref")
}

fn subject() -> Subject {
    Subject { source_ref: reference("source"), build_ref: reference("binary"), dependency_closure_ref: reference("closure"),
        feature_set: vec![], execution_profile: "test-simulation".into(), adapter_refs: vec![reference("adapter")],
        platform: "x86_64-linux".into(), environment: Environment::Simulation }
}

fn inputs() -> (Matrix, Mapping, Input) {
    let requirement = "req.one";
    let entries = ["positive", "negative"].map(|kind| MatrixEntry { requirement_id: requirement.into(), coverage_kind: kind.into(),
        evidence_scope: "integration".into(), target: "native-scenario".into(), command: "actual producer command".into(),
        artifact_refs: vec![], receipt_ref: reference(kind), caveats: vec!["definition, not a retained run".into()] }).to_vec();
    let matrix = Matrix { schema: "molten.testing.evidence-matrix.v1".into(), entries, exemptions: vec![] };
    let mapping = Mapping { schema: "molten.capability-mapping.v1".into(), capabilities: vec![Capability {
        id: "fixture".into(), requirements: vec![requirement.into()], owners: vec!["native-owner".into()], profiles: vec!["test-simulation".into()],
        families: vec![Family { id: "finite".into(), environment: Environment::Simulation, matrix_scopes: vec!["integration".into()] }],
    }] };
    let input = Input { schema: "molten.capability-report.v2".into(), source_root_path: "not-provisioned".into(), subject: subject(),
        capabilities: vec![CapabilityInput { id: "fixture".into(), evidence: vec![] }] };
    (matrix, mapping, input)
}

// Aggregation facts are unit-test inputs only, not retained runtime evidence.
// Production observations can be bound only by the native artifact validator.
fn observation(kind: &str, decision: &str) -> Observation {
    Observation { requirement_id: "req.one".into(), coverage_kind: kind.into(), family: "finite".into(), scenario: format!("finite-{kind}"),
        artifact_ref: reference(&format!("{kind}-{decision}")), native_decision: decision.into(), canonical_decision: decision.into(),
        validated: true, exact_subject_bound: true, diagnostics: vec![] }
}

fn assert_status(matrix: &Matrix, mapping: &Mapping, input: &Input, observations: Vec<Observation>, expected: &str) -> Result<(), String> {
    let facts = BTreeMap::from([("fixture".into(), observations)]);
    let actual = report::build(matrix, mapping, input, &facts)?;
    if actual.capabilities[0].execution.status != expected { return Err(format!("expected {expected}, got {}", actual.capabilities[0].execution.status)); }
    Ok(())
}

pub fn run() -> Result<(), String> {
    let (mut matrix, mut mapping, mut input) = inputs();
    assert_status(&matrix, &mapping, &input, vec![], "not-recorded")?;
    let positive = observation("positive", "pass");
    let negative = observation("negative", "pass");
    assert_status(&matrix, &mapping, &input, vec![positive.clone()], "inconclusive")?;
    assert_status(&matrix, &mapping, &input, vec![positive.clone(), negative.clone()], "bounded-observed")?;
    assert_status(&matrix, &mapping, &input, vec![positive.clone(), negative.clone(), observation("positive", "fail")], "failed")?;
    let mut unvalidated = positive.clone(); unvalidated.validated = false;
    assert_status(&matrix, &mapping, &input, vec![positive.clone(), negative.clone(), unvalidated], "inconclusive")?;
    mapping.capabilities[0].requirements.push("req.two".into());
    assert_status(&matrix, &mapping, &input, vec![positive.clone(), negative.clone()], "inconclusive")?;
    mapping.capabilities[0].requirements.pop();
    input.subject.environment = Environment::CrossHost;
    assert_status(&matrix, &mapping, &input, vec![positive.clone(), negative.clone()], "unsupported")?;
    input.subject.environment = Environment::Simulation;
    matrix.entries.pop();
    assert_status(&matrix, &mapping, &input, vec![positive.clone(), negative.clone()], "inconclusive")?;
    matrix.exemptions.push(Exemption { requirement_id: "req.one".into(), reason_class: "reviewed-finite-control".into(),
        evidence_ref: reference("reviewed exemption artifact"), scope: "integration".into(), review_note: "explicit reviewed finite exemption control".into() });
    assert_status(&matrix, &mapping, &input, vec![positive.clone()], "bounded-observed")?;
    assert_status(&matrix, &mapping, &input, vec![], "not-recorded")?;
    matrix.exemptions[0].evidence_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000222".into();
    assert_status(&matrix, &mapping, &input, vec![positive.clone()], "inconclusive")?;
    let (matrix, mapping, input) = inputs();
    let first = report::build(&matrix, &mapping, &input, &BTreeMap::from([("fixture".into(), vec![positive.clone(), negative.clone()])]))?;
    let second = report::build(&matrix, &mapping, &input, &BTreeMap::from([("fixture".into(), vec![negative, positive])]))?;
    let canonical = |value: &Report| -> Result<Vec<u8>, String> {
        let json = serde_json::to_value(value).map_err(|error| error.to_string())?;
        molten::preserves_rail::canonical_bytes(&molten::live_reliability::checker::canonical_value(&json)?).map_err(|error| error.to_string())
    };
    if canonical(&first)? != canonical(&second)? { return Err("canonical ordering depends on evidence input order".into()); }
    artifact_controls()?;
    println!("capability report self-test ok (aggregation and real native artifact controls; no signed-source/live qualification claimed)");
    Ok(())
}

fn artifact_controls() -> Result<(), String> {
    let guard = tempfile::Builder::new().prefix("aspen-capability-controls-").tempdir().map_err(|error| error.to_string())?;
    let subject = subject();
    let mut evidence = Evidence { requirement_id: "req.one".into(), coverage_kind: "positive".into(), family: "finite".into(),
        scenario: "finite-positive".into(), subject: subject.clone(), artifact_path: "artifact.preserves".into(), artifact_ref: reference("missing") };
    let inspect = |entry: &Evidence| artifacts::inspect(entry, guard.path(), &subject, None);
    if inspect(&evidence).validated { return Err("missing file accepted".into()); }
    std::fs::write(guard.path().join(&evidence.artifact_path), br#"{"schema":"molten.capability-evidence.v1","decision":"pass"}"#).map_err(|error| error.to_string())?;
    if inspect(&evidence).validated { return Err("generic authored JSON success accepted".into()); }
    let suite = molten::capability_qualification::fixture(molten::capability_qualification::Scenario::GrantedSend).map_err(|error| error.to_string())?;
    let run = molten::harness::run_suite_value(&suite).map_err(|error| error.to_string())?;
    let text = molten::preserves_rail::to_text(&run.report_value).map_err(|error| error.to_string())?;
    std::fs::write(guard.path().join(&evidence.artifact_path), text).map_err(|error| error.to_string())?;
    evidence.artifact_ref = run.report_ref;
    let raw = inspect(&evidence);
    if !raw.validated || raw.exact_subject_bound { return Err("raw valid harness report incorrectly attests exact executed build".into()); }
    evidence.artifact_ref = "blake3:placeholder".into();
    if inspect(&evidence).validated { return Err("placeholder accepted".into()); }
    evidence.artifact_ref = reference("wrong actual hash");
    if inspect(&evidence).validated { return Err("tampered artifact identity accepted".into()); }
    let failure = molten::harness::failure_value("execute", &molten::error::Failure::invalid_harness("failure control"), vec![]);
    std::fs::write(guard.path().join(&evidence.artifact_path), molten::preserves_rail::to_text(&failure).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    evidence.artifact_ref = molten::preserves_rail::canonical_hash(&failure).map_err(|error| error.to_string())?;
    if inspect(&evidence).native_decision != "fail" { return Err("canonical failure suppressed".into()); }
    Ok(())
}

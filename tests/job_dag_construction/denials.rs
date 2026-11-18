//! Construction negative controls and the node budget, through the real nickel-eval companion
//! worker.

const BUDGET: &str = include_str!("../../config/job-graphs/construction/negative/budget.ncl");
/// Operations in one `block` instance of the budget fixture.
const BUDGET_BLOCK_OPERATIONS: usize = 16;
/// 17 compact instances expand natively to 272 operations, past the 256-node runtime ceiling.
const OVER_NODE_BUDGET_BLOCKS: u64 = 17;

fn rejection(authoring: &super::Authoring<'_>) -> super::TestResult<String> {
    match super::lower(authoring) {
        Ok(_) => Err("construction unexpectedly admitted".into()),
        Err(error) => Ok(error.to_string()),
    }
}

#[test]
fn construction_negative_controls_reject_with_their_cause() -> super::TestResult {
    let cases = [
        (
            include_str!("../../config/job-graphs/construction/negative/unknown-operation.ncl"),
            "UnknownOperation",
        ),
        (
            include_str!("../../config/job-graphs/construction/negative/foreign-registry.ncl"),
            "InvalidRegistry",
        ),
        (include_str!("../../config/job-graphs/construction/negative/fractional-value.ncl"), "exact u64"),
        (include_str!("../../config/job-graphs/construction/negative/fan-in.ncl"), "UsageViolation"),
        (
            include_str!("../../config/job-graphs/construction/negative/cycle.ncl"),
            "graph construction rejected",
        ),
        (
            include_str!("../../config/job-graphs/construction/negative/duplicate-key.ncl"),
            "evaluation rejected",
        ),
        (include_str!("../../config/job-graphs/construction/negative/effect.ncl"), "InvalidEffect"),
        (
            include_str!("../../config/job-graphs/construction/negative/ambient-import.ncl"),
            "evaluation rejected",
        ),
    ];
    for (source, cause) in cases {
        let message = rejection(&super::inline(source, serde_json::json!({}), &[]))?;
        assert!(message.contains(cause), "expected {cause}, got {message}");
    }
    let missing = rejection(&super::inline(super::SEQUENCE, serde_json::json!({}), &[super::SEQUENCE_TOTAL]))?;
    assert!(missing.contains("evaluation rejected"), "missing parameter: {missing}");
    let dangling =
        rejection(&super::inline(super::SEQUENCE, serde_json::json!({"values": [1]}), &[&["pipeline", "missing"]]))?;
    assert!(dangling.contains("does not name an operation"), "dangling output: {dangling}");
    let reserved = [molten::job_dag::ConstructionModule {
        logical_id: "choregraph/graph-construction-v1.ncl",
        source: "{}",
    }];
    let spoofed = rejection(&super::Authoring {
        source: super::SEQUENCE,
        modules: &reserved,
        parameters: serde_json::json!({"values": [1]}),
        outputs: &[],
    })?;
    assert!(spoofed.contains("shipped logical identity"), "reserved module: {spoofed}");
    let foreign = super::lower_with(
        &super::inline(super::SEQUENCE, serde_json::json!({"values": [1]}), &[]),
        "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert!(foreign.is_err(), "an unselected worker identity is rejected before evaluation");
    Ok(())
}

#[test]
fn construction_budget_is_enforced_before_admission() -> super::TestResult {
    let within = super::lower(&super::inline(BUDGET, serde_json::json!({"count": 4}), &[]))?;
    assert_eq!(within.dag.nodes.len(), 4 * BUDGET_BLOCK_OPERATIONS);
    let over = rejection(&super::inline(BUDGET, serde_json::json!({"count": OVER_NODE_BUDGET_BLOCKS}), &[]))?;
    assert!(over.contains("LimitExceeded"), "over budget: {over}");
    Ok(())
}

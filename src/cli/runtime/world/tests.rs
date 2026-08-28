use molten::world_operator::plan_world_operator_request;
use molten_core::world_operator::*;

use super::document::WorkflowDocument;
use super::*;

const TEST_GENERATION: u64 = 5;
const TEST_MAX_OPERATIONS: usize = 8;
const TEST_MAX_DEPENDENCIES: usize = 8;
const TEST_MAX_RECEIPTS: usize = 16;
const TEST_MAX_CANONICAL_BYTES: usize = 65_536;

#[test]
fn document_rejects_raw_command_and_missing_expected_head() {
    let raw_command = valid_document().replace("\"operations\":", "\"raw_command\":\"rm -rf /\",\"operations\":");
    let error = serde_json::from_str::<WorkflowDocument>(&raw_command).expect_err("raw command field denied");
    assert!(error.to_string().contains("unknown field"));

    let missing_head = valid_document().replace(&format!("\"expected_head\":\"{}\",", reference("world")), "");
    let error = serde_json::from_str::<WorkflowDocument>(&missing_head).expect_err("missing expected head denied");
    assert!(error.to_string().contains("expected_head"));
}

#[test]
fn document_parses_only_closed_typed_operation_and_profile_vocabularies() {
    let document: WorkflowDocument = serde_json::from_str(&valid_document()).expect("valid document");
    let request = document.into_request().expect("typed request");
    assert_eq!(request.operations.len(), 1);
    assert_eq!(request.operations[0].kind, WorldOperationKind::Inspect);

    let unknown_operation = valid_document().replace("\"kind\":\"inspect\"", "\"kind\":\"shell\"");
    let document: WorkflowDocument = serde_json::from_str(&unknown_operation).expect("document shape");
    let error = document.into_request().expect_err("unknown operation denied");
    assert!(error.to_string().contains("unsupported world workflow operation"));
}

#[test]
fn mutation_apply_writes_a_denial_receipt_without_an_admitted_handler() {
    use std::os::fd::AsRawFd;

    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../tests/fixtures/world-operator/logical/request.json"))
            .expect("logical request value");
    let checkpoint = value["operations"]
        .as_array()
        .expect("operations array")
        .iter()
        .find(|operation| operation["kind"] == "checkpoint")
        .expect("checkpoint operation")
        .clone();
    value["operations"] = serde_json::Value::Array(vec![checkpoint]);
    value["operations"][0]["dependencies"] = serde_json::Value::Array(Vec::new());

    let document: WorkflowDocument = serde_json::from_value(value.clone()).expect("checkpoint document");
    let request = document.into_request().expect("checkpoint request");
    let plan = plan_world_operator_request(&request).expect("checkpoint plan");

    let temporary = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary command directory");
    let host_root =
        std::fs::read_link(format!("/proc/self/fd/{}", temporary.as_raw_fd())).expect("temporary host path");
    let request_path = host_root.join("request.json");
    let plan_path = host_root.join("plan.preserves");
    let receipt_path = host_root.join("receipt.preserves");
    let summary_path = host_root.join("summary.preserves");
    std::fs::write(&request_path, serde_json::to_vec_pretty(&value).expect("request json")).expect("write request");

    let result = run_world_command(WorldCommand::Checkpoint(WorldMutationArgs {
        request: request_path,
        plan_out: plan_path.clone(),
        summary_out: Some(summary_path.clone()),
        apply_plan_ref: Some(plan.plan.plan_ref),
        receipt_out: Some(receipt_path.clone()),
    }));
    assert!(result.is_err());
    assert!(std::fs::metadata(plan_path).expect("plan metadata").len() > 0);
    assert!(std::fs::metadata(receipt_path).expect("receipt metadata").len() > 0);
    assert!(std::fs::metadata(summary_path).expect("summary metadata").len() > 0);
}

#[test]
fn retained_logical_and_opaque_dogfood_artifacts_are_fresh() {
    assert_fixture_fresh(
        include_str!("../../../../tests/fixtures/world-operator/logical/request.json"),
        include_bytes!("../../../../tests/fixtures/world-operator/logical/plan.preserves"),
        include_bytes!("../../../../tests/fixtures/world-operator/logical/receipt.preserves"),
        include_bytes!("../../../../tests/fixtures/world-operator/logical/summary.preserves"),
    );
    assert_fixture_fresh(
        include_str!("../../../../tests/fixtures/world-operator/opaque/request.json"),
        include_bytes!("../../../../tests/fixtures/world-operator/opaque/plan.preserves"),
        include_bytes!("../../../../tests/fixtures/world-operator/opaque/receipt.preserves"),
        include_bytes!("../../../../tests/fixtures/world-operator/opaque/summary.preserves"),
    );
}

fn assert_fixture_fresh(request: &str, plan: &[u8], receipt: &[u8], summary: &[u8]) {
    let document: WorkflowDocument = serde_json::from_str(request).expect("fixture request document");
    let request = document.into_request().expect("fixture typed request");
    let run = plan_world_operator_request(&request).expect("fixture workflow plan");
    assert_eq!(run.plan_record.bytes, plan);
    assert_eq!(run.receipt_record.bytes, receipt);
    assert_eq!(run.summary_record.bytes, summary);
}

fn valid_document() -> String {
    let world_ref = reference("world");
    let profile_ref = reference("profile");
    format!(
        "{{\"schema\":\"{WORLD_WORKFLOW_REQUEST_SCHEMA}\",\"request_ref\":\"{}\",\"world_ref\":\"{world_ref}\",\"branch_id\":\"world/test\",\"expected_head\":\"{world_ref}\",\"expected_generation\":{TEST_GENERATION},\"policy_ref\":\"{}\",\"authority_observation_ref\":\"{}\",\"limits\":{{\"limits_ref\":\"{}\",\"max_operations\":{TEST_MAX_OPERATIONS},\"max_dependencies_per_operation\":{TEST_MAX_DEPENDENCIES},\"max_receipt_links\":{TEST_MAX_RECEIPTS},\"max_canonical_bytes\":{TEST_MAX_CANONICAL_BYTES}}},\"profiles\":[{{\"profile_ref\":\"{profile_ref}\",\"kind\":\"logical\",\"status\":\"admitted\",\"status_ref\":\"{}\"}}],\"observations\":[{{\"kind\":\"profile\",\"observation_ref\":\"{}\",\"subject_ref\":\"{profile_ref}\",\"admitted\":true}}],\"operations\":[{{\"operation_id\":\"{}\",\"kind\":\"inspect\",\"subject_ref\":\"{world_ref}\",\"profile_ref\":\"{profile_ref}\",\"dependencies\":[]}}]}}",
        reference("request"),
        reference("policy"),
        reference("authority"),
        reference("limits"),
        reference("profile-status"),
        reference("profile-observation"),
        reference("inspect-operation"),
    )
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

const README: &str = include_str!("../README.md");
const OPERATOR_RECEIPTS: &str = include_str!("../docs/operator-receipts.md");
const DOGFOOD_RECEIPT: &str = include_str!("../crates/aspen-dogfood/src/receipt.rs");
const DOGFOOD_MAIN: &str = include_str!("../crates/aspen-dogfood/src/main.rs");
const CI_MESSAGES: &str = include_str!("../crates/aspen-client-api/src/messages/ci.rs");
const CI_COMMAND: &str = include_str!("../crates/aspen-cli/src/bin/aspen-cli/commands/ci.rs");

#[test]
fn operator_receipts_doc_is_discoverable() {
    assert!(README.contains("docs/operator-receipts.md"));
    assert!(README.contains("Operator Receipts"));
    assert!(OPERATOR_RECEIPTS.contains("# Operator Receipts"));
}

#[test]
fn operator_receipts_doc_tracks_receipt_schemas_and_commands() {
    assert!(OPERATOR_RECEIPTS.contains("aspen.dogfood.run-receipt.v1"));
    assert!(OPERATOR_RECEIPTS.contains("aspen.ci.run-receipt.v1"));
    assert!(OPERATOR_RECEIPTS.contains("receipts cluster-show <run-id> --json"));
    assert!(OPERATOR_RECEIPTS.contains("aspen-cli --json ci receipt <run-id>"));
    assert!(OPERATOR_RECEIPTS.contains("elapsed_ms"));
    assert!(OPERATOR_RECEIPTS.contains("artifact metadata"));

    assert!(DOGFOOD_RECEIPT.contains("DOGFOOD_RUN_RECEIPT_SCHEMA"));
    assert!(DOGFOOD_RECEIPT.contains("pub elapsed_ms: Option<u64>"));
    assert!(DOGFOOD_MAIN.contains("ClusterShow"));
    assert!(CI_MESSAGES.contains("CI_RUN_RECEIPT_SCHEMA"));
    assert!(CI_MESSAGES.contains("pub artifacts: Vec<CiArtifactInfo>"));
    assert!(CI_COMMAND.contains("CiGetRunReceipt"));
}

#[test]
fn operator_receipts_doc_preserves_secret_redaction_guidance() {
    assert!(OPERATOR_RECEIPTS.contains("[REDACTED]"));
    assert!(OPERATOR_RECEIPTS.contains("Do not paste cluster tickets"));
}

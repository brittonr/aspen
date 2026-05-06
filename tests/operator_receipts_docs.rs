const DOGFOOD_RECEIPT: &str = include_str!("../crates/aspen-dogfood/src/receipt.rs");
const DOGFOOD_MAIN: &str = include_str!("../crates/aspen-dogfood/src/main.rs");
const CI_MESSAGES: &str = include_str!("../crates/aspen-client-api/src/messages/ci.rs");
const CI_COMMAND: &str = include_str!("../crates/aspen-cli/src/bin/aspen-cli/commands/ci.rs");

fn read_repo_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[test]
fn operator_receipts_doc_is_discoverable() {
    let Some(readme) = read_repo_file("README.md") else {
        // Nix's cleaned Cargo test source omits non-Cargo docs files. Local Cargo
        // runs keep this assertion strict; cleaned-source builds keep compile-time
        // source anchors below.
        return;
    };
    let operator_receipts = std::fs::read_to_string("docs/operator-receipts.md")
        .expect("operator receipts doc should exist when README is present");

    assert!(readme.contains("docs/operator-receipts.md"));
    assert!(readme.contains("Operator Receipts"));
    assert!(operator_receipts.contains("# Operator Receipts"));
}

#[test]
fn operator_receipts_doc_tracks_receipt_schemas_and_commands() {
    if let Some(operator_receipts) = read_repo_file("docs/operator-receipts.md") {
        assert!(operator_receipts.contains("aspen.dogfood.run-receipt.v1"));
        assert!(operator_receipts.contains("aspen.ci.run-receipt.v1"));
        assert!(operator_receipts.contains("receipts cluster-show <run-id> --json"));
        assert!(operator_receipts.contains("aspen-cli --json ci receipt <run-id>"));
        assert!(operator_receipts.contains("elapsed_ms"));
        assert!(operator_receipts.contains("artifact metadata"));
        assert!(operator_receipts.contains("Acceptance evidence trail"));
        assert!(operator_receipts.contains("dogfood-20260505T202756Z"));
        assert!(operator_receipts.contains("dogfood-20260506T220958Z"));
        assert!(operator_receipts.contains("a3f2cad78a6760f3782302bf68d15104db948123"));
        assert!(operator_receipts.contains("Fix dogfood CI build artifact reuse"));
    }

    assert!(DOGFOOD_RECEIPT.contains("DOGFOOD_RUN_RECEIPT_SCHEMA"));
    assert!(DOGFOOD_RECEIPT.contains("pub elapsed_ms: Option<u64>"));
    assert!(DOGFOOD_MAIN.contains("ClusterShow"));
    assert!(CI_MESSAGES.contains("CI_RUN_RECEIPT_SCHEMA"));
    assert!(CI_MESSAGES.contains("pub artifacts: Vec<CiArtifactInfo>"));
    assert!(CI_COMMAND.contains("CiGetRunReceipt"));
}

#[test]
fn operator_receipts_doc_preserves_secret_redaction_guidance() {
    let Some(operator_receipts) = read_repo_file("docs/operator-receipts.md") else {
        return;
    };
    assert!(operator_receipts.contains("[REDACTED]"));
    assert!(operator_receipts.contains("Do not paste cluster tickets"));
}

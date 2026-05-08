#[test]
fn runtime_service_contract_docs_define_non_activation_boundary() {
    let docs = include_str!("../docs/runtime-service-contract.md");

    assert!(docs.contains("canonical_runtime_service_contract"));
    assert!(docs.contains("RuntimeServiceContract"));
    assert!(docs.contains("A validated contract is not an activation claim"));
    assert!(docs.contains("only marks a route `active` when"));
    assert!(docs.contains("running` and `healthy"));
    assert!(docs.contains("runtime_receipt_correlation"));
    assert!(docs.contains("backend execution ID"));
}

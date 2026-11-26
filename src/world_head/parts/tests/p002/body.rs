#[test]
fn invalid_bounds_denial_receipt_carries_closed_kebab_case_codes() {
    // r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let mut request = signed_request(&mut signer, create_claim());
    request.bounds.max_signers = 0;
    let mut authority = TestAuthority {
        admitted: true,
        calls: 0,
    };

    let denied = execute_world_head_transition(&mut store, &mut authority, &request).expect("bounds denial result");

    assert_eq!(denied.status, WorldHeadExecutionStatus::Denied);
    assert!(denied.issues.contains(&molten_core::world_head::WorldHeadIssue::InvalidBounds(
        molten_core::world_head::WorldHeadBound::MaxSigners
    )));
    let text = crate::preserves_rail::to_text(&denied.receipt.value).expect("receipt text");
    assert!(text.contains("\"invalid-bounds:max-signers\""), "{text}");
    assert!(text.contains("\"invalid-bounds:signature-threshold\""), "{text}");
    assert!(!text.contains("max_signers"), "{text}");
    assert!(!text.contains("InvalidBounds"), "{text}");
}

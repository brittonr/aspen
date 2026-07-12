use super::*;

const SECRET_MODE: u32 = 0o600;
const UNSAFE_SECRET_MODE: u32 = 0o644;

fn state_root(label: &str) -> (crate::test_support::TestWorkspace, NodeStateRoot) {
    let workspace = crate::test_support::TestWorkspace::new(label).expect("test workspace");
    let state = workspace.state().expect("state root");
    let dir = state.dir().try_clone().expect("clone state capability");
    (workspace, NodeStateRoot::from_dir(dir))
}

#[test]
fn pure_paths_accept_bounded_relative_locators_and_reject_authority_smuggling() {
    // r[verify molten.node.cap_std_namespaces]
    let base = NodeStatePath::parse("control/inbox").expect("fixed path");
    assert_eq!(base.join_segment("request.preserves").expect("leaf").display(), "control/inbox/request.preserves");
    for invalid_value in [
        "",
        ".",
        "../escape",
        "/absolute",
        "C:\\escape",
        "https://remote",
        "iroh:ticket",
    ] {
        assert!(NodeStatePath::parse(invalid_value).is_err(), "{invalid_value} must deny");
    }
    assert!(base.join_segment("nested/request").is_err());
}

#[test]
fn namespace_io_is_sorted_bounded_and_root_bound() {
    // r[verify molten.node.cap_std_state_root]
    // r[verify molten.node.cap_std_validation]
    let (_left_workspace, left) = state_root("node-state-left");
    let (_right_workspace, right) = state_root("node-state-right");
    left.create_layout().expect("left layout");
    right.create_layout().expect("right layout");
    let left_inbox = left.control_inbox().expect("left inbox");
    let right_inbox = right.control_inbox().expect("right inbox");
    let first = NodeStatePath::parse("b.preserves").expect("first path");
    let second = NodeStatePath::parse("a.preserves").expect("second path");
    left_inbox.write(&first, b"b").expect("write first");
    left_inbox.write(&second, b"a").expect("write second");
    let entries = left_inbox.list_entries().expect("list entries");
    assert_eq!(entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), vec![
        "a.preserves",
        "b.preserves"
    ]);
    assert_eq!(left_inbox.read_entry(&entries[0], MAX_NODE_STATE_FILE_BYTES).expect("read entry"), b"a");
    let error = right_inbox.remove_entry(&entries[0]).expect_err("wrong-root entry must deny");
    assert!(error.to_string().contains("different root or namespace"));
    assert!(left_inbox.try_exists(&second).expect("left entry remains"));
}

#[cfg(unix)]
#[test]
fn namespace_listing_rejects_non_utf8_entry_names() {
    // r[verify molten.node.cap_std_validation]
    use std::os::unix::ffi::OsStringExt;

    const INVALID_UTF8_ENTRY_NAME: [u8; 1] = [u8::MAX];
    let (workspace, root) = state_root("node-state-non-utf8-entry");
    root.create_layout().expect("layout");
    let state = workspace.state().expect("state capability");
    let plan = workspace.process_bridge().plan(&state).expect("diagnostic bridge");
    let invalid_name = std::ffi::OsString::from_vec(INVALID_UTF8_ENTRY_NAME.to_vec());
    std::fs::write(plan.path().join("control/inbox").join(invalid_name), b"invalid").expect("invalid entry");

    let error = root.control_inbox().expect("inbox").list_entries().expect_err("non-UTF-8 entry must deny");
    assert!(error.to_string().contains("valid UTF-8"));
}

#[test]
fn entries_are_bound_to_the_exact_namespace_view() {
    // r[verify molten.node.cap_std_validation]
    let (_workspace, root) = state_root("node-state-namespace-view-binding");
    root.create_layout().expect("layout");
    let ingress = root.control_ingress().expect("ingress");
    let leaf = NodeStatePath::parse("same.preserves").expect("leaf");
    ingress.write(&leaf, b"parent").expect("parent value");
    let topic_path = NodeStatePath::parse("topic").expect("topic");
    let topic = ingress.open_subdir(&topic_path).expect("topic view");
    topic.write(&leaf, b"child").expect("child value");
    let entry = ingress
        .list_entries()
        .expect("parent entries")
        .into_iter()
        .find(|entry| entry.name == "same.preserves")
        .expect("parent entry");

    let error = topic.read_entry(&entry, MAX_NODE_STATE_FILE_BYTES).expect_err("cross-view entry must deny");
    assert!(error.to_string().contains("namespace view"));
    assert_eq!(ingress.read(&leaf, MAX_NODE_STATE_FILE_BYTES).expect("parent remains"), b"parent");
    assert_eq!(topic.read(&leaf, MAX_NODE_STATE_FILE_BYTES).expect("child remains"), b"child");
}

#[test]
fn missing_parent_is_absent_without_weakening_invalid_component_denials() {
    // r[verify molten.node.cap_std_state_root]
    let (_workspace, root) = state_root("node-state-missing-parent");
    let missing = NodeStatePath::parse("control/node.lock.preserves").expect("missing path");
    assert!(!root.try_exists(&missing).expect("missing parent is absent"));

    let invalid = NodeStatePath::parse("control/../escape");
    assert!(invalid.is_err());
}

#[test]
fn nested_store_handles_share_the_open_node_authority() {
    // r[verify molten.node.cap_std_namespaces]
    // r[verify molten.node.cap_std_validation]
    let (_workspace, root) = state_root("node-state-nested-stores");
    root.create_layout().expect("layout");
    let artifact_root = root.artifact_store().expect("artifact root");
    let ledger_root = root.ledger_store().expect("ledger root");
    let chunk_root = root.chunk_store().expect("chunk root");
    let delivery_root = root.delivery_store().expect("delivery root");

    let payload =
        crate::preserves_rail::record("node-state-nested-store-fixture", vec![crate::preserves_rail::string(
            "payload",
        )]);
    let installed =
        crate::artifacts::install_artifact_with_root(&artifact_root, &crate::artifacts::ArtifactInstallInput {
            kind: "steel".to_string(),
            payload: payload.clone(),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: Vec::new(),
            evidence_refs: Vec::new(),
            installer_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("installer"))
                .expect("installer ref"),
            capability_refs: vec![
                crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("node-state-capability"))
                    .expect("capability ref"),
            ],
        })
        .expect("install artifact");
    assert_eq!(
        crate::artifacts::read_payload_with_root(&artifact_root, &installed.artifact_ref)
            .expect("read artifact payload"),
        payload
    );

    let ledger_ref = crate::ledger::import_artifact_with_root(&ledger_root, &installed.receipt_value)
        .expect("import ledger artifact");
    crate::ledger::read_artifact_with_root(&ledger_root, &ledger_ref.artifact_ref).expect("read ledger artifact");
    let chunk = crate::chunk_store::put_bytes_with_root(
        &chunk_root,
        "node-state-test",
        b"chunk payload",
        crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE,
    )
    .expect("put chunk payload");
    crate::chunk_store::read_object_with_root(&chunk_root, &chunk.manifest_ref).expect("read chunk payload");

    let scope_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("scope")).expect("scope ref");
    let delivery = crate::delivery_idempotency::check_with_root(crate::delivery_idempotency::CapabilityCheckInput {
        root: &delivery_root,
        request: crate::delivery_idempotency::CheckRequest {
            scope_profile: crate::delivery_idempotency::SCOPE_CONTROL_COMMAND,
            scope_ref: &scope_ref,
            producer: "node:test",
            consumer: "node:test",
            sequence: 1,
            intent: "node-state-test",
            payload_ref: &installed.artifact_ref,
            policy_refs: &[],
            evidence_refs: &[],
            semantic_result_ref: Some(&installed.artifact_ref),
            gap_policy: crate::delivery_idempotency::GapPolicy::Deny,
        },
    })
    .expect("record delivery decision");
    assert!(delivery.should_commit_side_effect);
}

#[cfg(unix)]
#[test]
fn symlinked_and_non_regular_leaves_deny_before_read_write_or_remove() {
    // r[verify molten.node.cap_std_validation]
    use std::os::unix::fs::symlink;

    let (workspace, root) = state_root("node-state-symlink-denial");
    root.create_layout().expect("layout");
    let state = workspace.state().expect("state capability");
    let plan = workspace.process_bridge().plan(&state).expect("diagnostic bridge");
    let outside = plan.path().join("outside");
    std::fs::write(&outside, b"outside").expect("outside file");
    let linked = plan.path().join("control/inbox/linked.preserves");
    symlink(&outside, &linked).expect("symlink leaf");
    let inbox = root.control_inbox().expect("inbox");
    let linked_path = NodeStatePath::parse("linked.preserves").expect("linked path");
    assert!(inbox.read(&linked_path, MAX_NODE_STATE_FILE_BYTES).is_err());
    assert!(inbox.write(&linked_path, b"replacement").is_err());
    assert!(inbox.remove_regular_file(&linked_path).is_err());
    assert_eq!(std::fs::read(&outside).expect("outside bytes"), b"outside");

    std::fs::create_dir(plan.path().join("control/inbox/directory.preserves")).expect("directory leaf");
    let directory = NodeStatePath::parse("directory.preserves").expect("directory path");
    assert!(inbox.read(&directory, MAX_NODE_STATE_FILE_BYTES).is_err());
}

#[cfg(unix)]
#[test]
fn restricted_secret_creation_and_unsafe_permission_observation_are_explicit() {
    // r[verify molten.node.cap_std_identity_secret]
    use std::os::unix::fs::PermissionsExt;

    let (workspace, root) = state_root("node-state-secret-permissions");
    let secrets = root.secrets().expect("secret namespace");
    let path = NodeStatePath::parse("node-endpoint.secret").expect("secret path");
    secrets.write_restricted(&path, b"secret\n", SECRET_MODE).expect("restricted write");
    assert_eq!(secrets.unix_mode(&path).expect("secret mode").expect("unix mode") & UNSAFE_SECRET_MODE, SECRET_MODE);

    let state = workspace.state().expect("state capability");
    let plan = workspace.process_bridge().plan(&state).expect("diagnostic bridge");
    let host_path = plan.path().join("identity/node-endpoint.secret");
    let mut permissions = std::fs::metadata(&host_path).expect("secret metadata").permissions();
    permissions.set_mode(UNSAFE_SECRET_MODE);
    std::fs::set_permissions(&host_path, permissions).expect("unsafe permissions");
    assert_ne!(secrets.unix_mode(&path).expect("unsafe mode").expect("unix mode") & UNSAFE_SECRET_MODE, SECRET_MODE);
}

#[cfg(unix)]
#[test]
fn observed_secret_handle_resists_leaf_replacement() {
    // r[verify molten.node.cap_std_identity_secret]
    // r[verify molten.node.cap_std_validation]
    let (workspace, root) = state_root("node-state-secret-replacement");
    let secrets = root.secrets().expect("secret namespace");
    let path = NodeStatePath::parse("node-endpoint.secret").expect("secret path");
    secrets.write_restricted(&path, b"original\n", SECRET_MODE).expect("original secret");
    let observation = secrets.observe_file(&path).expect("observe secret");

    let state = workspace.state().expect("state capability");
    let plan = workspace.process_bridge().plan(&state).expect("diagnostic bridge");
    let host_path = plan.path().join("identity/node-endpoint.secret");
    std::fs::rename(&host_path, host_path.with_extension("observed")).expect("move observed secret");
    std::fs::write(&host_path, b"replacement\n").expect("replacement secret");

    let NodeStateFileObservation::Regular(file) = observation else {
        panic!("regular secret observation expected");
    };
    assert_eq!(file.read_bounded(MAX_NODE_SECRET_BYTES).expect("observed bytes"), b"original\n");
    assert_eq!(std::fs::read(&host_path).expect("replacement bytes"), b"replacement\n");
}

#[cfg(unix)]
#[test]
fn opened_root_remains_bound_after_host_path_replacement() {
    // r[verify molten.node.cap_std_state_root]
    let workspace = crate::test_support::TestWorkspace::new("node-state-root-replacement").expect("workspace");
    let state = workspace.state().expect("state root");
    let plan = workspace.process_bridge().plan(&state).expect("diagnostic bridge");
    let root = NodeStateRoot::open(plan.path()).expect("open root");
    let marker = NodeStatePath::parse("identity/original.preserves").expect("marker path");
    root.write(&marker, b"original").expect("write original");

    let moved = plan.path().with_extension("opened-root");
    std::fs::rename(plan.path(), &moved).expect("rename opened root path");
    std::fs::create_dir(plan.path()).expect("create replacement root");
    std::fs::create_dir(plan.path().join("identity")).expect("create replacement identity");
    std::fs::write(plan.path().join("identity/original.preserves"), b"replacement").expect("replacement marker");

    assert_eq!(root.read(&marker, MAX_NODE_STATE_FILE_BYTES).expect("read opened authority"), b"original");
    assert_eq!(
        std::fs::read(plan.path().join("identity/original.preserves")).expect("replacement bytes"),
        b"replacement"
    );
}

//! Request replacement and lock redirect tests.

#[test]
fn discovered_request_replacement_denies_without_cleanup() {
    // r[verify molten.node.cap_std_request_lifecycle]
    // r[verify molten.node.cap_std_validation]
    let root_path =
        super::mailbox::initialized_control_root("node-control-request-replacement", "node:request-replacement");
    let request = super::super::running::status_request().expect("status request");
    let submitted = super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root_path,
        request_value: &request.value,
    })
    .expect("submit request");
    let root = crate::node_state::DirectoryRoot::open(&root_path).expect("state root");
    let pending =
        super::super::leases::pending_control_request_by_name(&root, &submitted.inbox_entry).expect("discover request");
    let replacement = super::super::running::shutdown_request().expect("replacement request");
    let replacement_text = crate::preserves_rail::to_text(&replacement.value).expect("replacement text");
    let entry_path = crate::node_state::RelativePath::parse(&submitted.inbox_entry).expect("entry path");
    let inbox = root.control_inbox().expect("inbox");
    inbox.write(&entry_path, replacement_text.as_bytes()).expect("replace request");

    let error =
        super::super::submission::dispatch_pending_control_request(&root, pending).expect_err("replacement must deny");
    assert!(error.to_string().contains("changed between discovery and dispatch"));
    assert_eq!(
        inbox
            .read_to_string(&entry_path, crate::node_state::MAX_NODE_STATE_FILE_BYTES)
            .expect("replacement remains"),
        replacement_text
    );
}

#[test]
fn stale_logical_request_entry_denies_before_dispatch() {
    // r[verify molten.node.cap_std_request_lifecycle]
    // r[verify molten.node.cap_std_validation]
    let root_path = super::mailbox::initialized_control_root("node-control-stale-entry", "node:stale-entry");
    let error =
        super::super::submission::dispatch_control_request_entry(&super::super::requests::ControlDispatchEntryInput {
            state_root: &root_path,
            request_entry: Some("missing.preserves"),
        })
        .expect_err("stale entry must deny");
    assert!(error.to_string().contains("is not pending"));
}

#[cfg(unix)]
#[test]
fn replaced_control_lock_cannot_redirect_cleanup() {
    // r[verify molten.node.cap_std_request_lifecycle]
    // r[verify molten.node.cap_std_validation]

    let root_path = super::mailbox::initialized_control_root("node-control-lock-replacement", "node:lock-replacement");
    let root = crate::node_state::DirectoryRoot::open(&root_path).expect("state root");
    let outside_root = crate::test_support::process_workspace("node_control_lock_outside").expect("outside workspace");
    std::fs::create_dir_all(&outside_root).expect("outside root");
    let outside_path = outside_root.join("outside.lock");
    std::fs::write(&outside_path, b"outside").expect("outside lock target");
    let lock_path = root_path.join(super::super::settings::CONTROL_LOCK_FILE);
    std::fs::remove_file(&lock_path).expect("remove original lock");
    std::os::unix::fs::symlink(&outside_path, &lock_path).expect("replace lock with symlink");

    let error = super::super::leases::remove_active_lock(&root).expect_err("replacement lock cleanup must deny");
    assert!(error.to_string().contains("regular file"));
    assert_eq!(std::fs::read(&outside_path).expect("outside bytes"), b"outside");
}

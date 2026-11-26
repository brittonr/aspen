//! Active node locks, artifact import, and pending control requests.

pub(super) fn current_startup_receipt<Root: super::storage::NodeStateAuthority + ?Sized>(
    source: &Root,
) -> crate::error::Result<crate::node_runtime::NodeStartupReceipt> {
    let root = source.acquire_node_state_root()?;
    let startup_value =
        super::storage::read_preserves(&root, &super::storage::fixed_node_path(super::settings::STARTUP_FILE)?)?;
    crate::node_runtime::parse_node_startup_receipt(&startup_value)
}

pub(super) fn write_active_lock(
    root: &crate::node_state::DirectoryRoot,
    startup_receipt_ref: &str,
) -> crate::error::Result<()> {
    let binding = super::tracking::begin_node_run_binding(root, startup_receipt_ref)?;
    let lock_value = active_lock_value(root, startup_receipt_ref, &binding.run_ref)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::CONTROL_LOCK_FILE)?,
        &lock_value,
    )?;
    super::storage::import_artifact(root, &lock_value)?;
    Ok(())
}

pub(super) fn require_active_lock(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<crate::node_runtime::NodeRunBinding> {
    let lock_path = super::storage::fixed_node_path(super::settings::CONTROL_LOCK_FILE)?;
    if !root.try_exists(&lock_path)? {
        return Err(crate::error::Failure::invalid_harness("node control dispatch requires active node lock"));
    }
    let lock_value = super::storage::read_preserves(root, &lock_path)?;
    let binding = super::tracking::current_node_run_binding(root)?;
    let startup = current_startup_receipt(root)?;
    super::guards::validate_member_ref(&binding.startup_receipt_ref, &startup.receipt_ref, "current node run startup")?;
    let run_ref = super::tracking::active_lock_run_ref(&lock_value, &binding)?;
    super::guards::validate_member_ref(&run_ref, &binding.run_ref, "node lock current run")?;
    Ok(binding)
}

pub(super) fn remove_active_lock(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<()> {
    let path = super::storage::fixed_node_path(super::settings::CONTROL_LOCK_FILE)?;
    if root.try_exists(&path)? {
        root.remove_regular_file(&path)?;
    }
    Ok(())
}

pub(super) fn active_lock_value(
    root: &crate::node_state::DirectoryRoot,
    startup_receipt_ref: &str,
    run_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("node-control-lock-v2", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA_V2),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(root)?,
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(startup_receipt_ref)]),
        crate::preserves_rail::record("owner", vec![crate::preserves_rail::string(&super::storage::local_ref(
            "node-control-owner",
            run_ref,
        )?)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-file-v2",
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("not-authority-token"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-state-root"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        crate::preserves_rail::record("run", vec![crate::preserves_rail::string(run_ref)]),
    ]))
}

pub(super) struct PendingControlRequest {
    pub(super) entry: crate::node_state::Entry,
    pub(super) content_ref: String,
}

pub(super) fn first_pending_control_request(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<PendingControlRequest> {
    next_pending_control_request(root)?
        .ok_or_else(|| crate::error::Failure::invalid_harness("node control inbox has no pending requests"))
}

pub(super) fn next_pending_control_request<Root: super::storage::NodeStateAuthority + ?Sized>(
    source: &Root,
) -> crate::error::Result<Option<PendingControlRequest>> {
    let root = source.acquire_node_state_root()?;
    Ok(pending_control_requests(&root)?.into_iter().next())
}

pub(super) fn pending_control_request_by_name(
    root: &crate::node_state::DirectoryRoot,
    name: &str,
) -> crate::error::Result<PendingControlRequest> {
    crate::node_state::RelativePath::parse("control/inbox")?.join_segment(name)?;
    pending_control_requests(root)?
        .into_iter()
        .find(|request| request.entry.name == name)
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness(format!("node control inbox entry {name} is not pending"))
        })
}

pub(super) fn pending_control_requests(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<Vec<PendingControlRequest>> {
    let inbox = root.control_inbox()?;
    let mut requests = Vec::with_capacity(super::settings::MAX_PENDING_CONTROL_REQUESTS);
    for entry in inbox.list_entries()? {
        if entry.kind != crate::node_state::EntryKind::RegularFile
            || !entry.name.ends_with(".preserves")
            || entry.name.contains("receipt")
        {
            continue;
        }
        if requests.len() >= super::settings::MAX_PENDING_CONTROL_REQUESTS {
            return Err(crate::error::Failure::invalid_harness("too many pending node control requests"));
        }
        let bytes = inbox.read_entry(&entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
        requests.push(PendingControlRequest {
            entry,
            content_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        });
    }
    Ok(requests)
}

pub(super) fn archive_dispatched_request(
    root: &crate::node_state::DirectoryRoot,
    request_entry: &crate::node_state::Entry,
    request_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let request_ref = crate::preserves_rail::canonical_hash(request_value)?;
    let archived = super::paths::control_outbox_request_path(&request_ref)?;
    super::storage::write_preserves(root, &archived, request_value)?;
    root.control_inbox()?.remove_entry(request_entry)
}

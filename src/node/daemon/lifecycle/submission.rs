//! Control request submission, dispatch entry, and the bounded control loop.

pub fn submit_control_request(
    input: &super::requests::ControlSubmitInput<'_>,
) -> crate::error::Result<super::outcomes::ControlSubmit> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    submit_control_request_with_root(&root, input.request_value)
}

pub fn submit_control_request_with_root(
    root: &crate::node_state::DirectoryRoot,
    request_value: &preserves::IOValue,
) -> crate::error::Result<super::outcomes::ControlSubmit> {
    super::storage::ensure_state_layout(root)?;
    super::ingress::submit(root, request_value)
}

pub fn dispatch_control_request(
    input: &super::requests::ControlDispatchInput<'_>,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    super::storage::validate_state_root(input.state_root)?;
    let request_entry = compatibility_request_entry(input.state_root, input.request_path)?;
    let root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    dispatch_control_request_with_root(&root, request_entry)
}

pub fn dispatch_control_request_entry(
    input: &super::requests::ControlDispatchEntryInput<'_>,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    dispatch_control_request_with_root(&root, input.request_entry)
}

fn compatibility_request_entry<'a>(
    state_root: &std::path::Path,
    request_path: Option<&'a std::path::Path>,
) -> crate::error::Result<Option<&'a str>> {
    let Some(request_path) = request_path else {
        return Ok(None);
    };
    let mut components = request_path.components();
    if let (Some(std::path::Component::Normal(entry)), None) = (components.next(), components.next()) {
        return entry.to_str().map(Some).ok_or_else(|| {
            crate::error::Failure::invalid_harness("node control compatibility request entry must be UTF-8")
        });
    }

    let relative = request_path.strip_prefix(state_root).map_err(|_| {
        crate::error::Failure::invalid_harness(
            "node control compatibility request path must name the selected state root inbox",
        )
    })?;
    if relative.parent() != Some(std::path::Path::new(super::settings::CONTROL_INBOX_DIR)) {
        return Err(crate::error::Failure::invalid_harness(
            "node control compatibility request path must name the selected state root inbox",
        ));
    }
    let entry = relative.file_name().and_then(std::ffi::OsStr::to_str).ok_or_else(|| {
        crate::error::Failure::invalid_harness("node control compatibility request entry must be UTF-8")
    })?;
    crate::node_state::RelativePath::parse(super::settings::CONTROL_INBOX_DIR)?.join_segment(entry)?;
    Ok(Some(entry))
}

pub fn dispatch_control_request_with_root(
    root: &crate::node_state::DirectoryRoot,
    request_entry: Option<&str>,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    super::storage::ensure_state_layout(root)?;
    super::leases::require_active_lock(root)?;
    let pending = match request_entry {
        Some(name) => super::leases::pending_control_request_by_name(root, name)?,
        None => super::leases::first_pending_control_request(root)?,
    };
    dispatch_pending_control_request(root, pending).map(|observed| observed.dispatch)
}

pub(super) fn dispatch_pending_control_request(
    root: &crate::node_state::DirectoryRoot,
    pending: super::leases::PendingControlRequest,
) -> crate::error::Result<super::tracking::ObservedControlDispatch> {
    let delivery_root = root.delivery_store()?;
    let mut publication_store = crate::delivery_idempotency::PublicationStore::open(&delivery_root)?;
    publication_store.require_healthy()?;
    let inbox = root.control_inbox()?;
    let bytes = inbox.read_entry(&pending.entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    let observed_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    if observed_ref != pending.content_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control inbox entry {} changed between discovery and dispatch",
            pending.entry.name
        )));
    }
    let text = String::from_utf8(bytes).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("node control request is not UTF-8: {error}"))
    })?;
    let request_value = crate::preserves_rail::parse_text(&text)?;
    let request = crate::node_runtime::parse_control_request(&request_value)?;
    super::storage::import_artifact(root, &request_value)?;
    if let Some(prior) = super::handlers::prior_dispatch_for_request(root, &request)? {
        super::ingress::dispatch::complete(root, &request, &mut publication_store)?;
        super::leases::archive_dispatched_request(root, &pending.entry, &request.value)?;
        super::handlers::write_dispatch_queue_receipt(root, &request, "duplicate-dispatch")?;
        return Ok(super::tracking::ObservedControlDispatch {
            dispatch: prior,
            is_duplicate: true,
        });
    }
    super::ingress::dispatch::run(root, &request, &mut publication_store, || {
        let dispatch = match request.operation.as_str() {
            "status" => super::handlers::dispatch_status_request(root, &request)?,
            "shutdown" => super::handlers::dispatch_shutdown_request(root, &request)?,
            "install" => super::installs::dispatch_install_request(root, &request)?,
            "run" => super::launches::dispatch_run_request(root, &request)?,
            "gate" => super::gating::dispatch_gate_request(root, &request)?,
            other => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "node control request operation unsupported: {other}"
                )));
            }
        };
        super::leases::archive_dispatched_request(root, &pending.entry, &request.value)?;
        super::handlers::write_dispatch_queue_receipt(root, &request, "dispatch")?;
        Ok(super::tracking::ObservedControlDispatch {
            dispatch,
            is_duplicate: false,
        })
    })
}

pub fn run_control_loop(
    input: &super::requests::ControlLoopInput<'_>,
) -> crate::error::Result<super::outcomes::ControlLoop> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    run_control_loop_with_root(&root, input.max_requests)
}

pub fn run_control_loop_with_root(
    root: &crate::node_state::DirectoryRoot,
    maximum_requests: u64,
) -> crate::error::Result<super::outcomes::ControlLoop> {
    super::storage::ensure_state_layout(root)?;
    let max_requests = super::guards::validate_loop_request_limit(maximum_requests)?;
    let binding = super::leases::require_active_lock(root)?;
    let startup = super::leases::current_startup_receipt(root)?;
    let lock_value =
        super::storage::read_preserves(root, &super::storage::fixed_node_path(super::settings::CONTROL_LOCK_FILE)?)?;
    let lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    let (heartbeat_receipt_ref, heartbeat_value) =
        super::tracking::write_control_loop_heartbeat(root, &startup.receipt_ref, &lock_ref)?;

    let mut processed_request_refs = Vec::with_capacity(max_requests);
    let mut dispatch_receipt_refs = Vec::with_capacity(max_requests);
    let mut diagnostics = Vec::new();
    let mut has_stopped = false;
    for _ in 0..max_requests {
        let Some(pending) = super::leases::next_pending_control_request(root)? else {
            break;
        };
        let observed = dispatch_pending_control_request(root, pending)?;
        let lifecycle = super::tracking::classify_control_dispatch_run(root, &binding, &observed)?;
        processed_request_refs.push(observed.dispatch.request_ref);
        dispatch_receipt_refs.push(observed.dispatch.control_receipt_ref);
        match lifecycle {
            crate::node_runtime::NodeRunLifecycle::Stopped => {
                has_stopped = true;
                break;
            }
            crate::node_runtime::NodeRunLifecycle::Active => {}
            crate::node_runtime::NodeRunLifecycle::Unresolved => {
                crate::bounded::push(
                    &mut diagnostics,
                    "node control shutdown has unresolved current-run observations".to_string(),
                    2,
                    "node control loop diagnostics",
                )?;
                break;
            }
        }
    }
    if processed_request_refs.len() == max_requests && super::leases::next_pending_control_request(root)?.is_some() {
        diagnostics.push("node control loop reached max requests with pending inbox entries".to_string());
    }
    let is_denied = !diagnostics.is_empty();
    let decision = if is_denied { "deny" } else { "pass" };
    let loop_value = super::operations::loop_receipt_value(&super::operations::LoopReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        heartbeat_receipt_ref: &heartbeat_receipt_ref,
        max_requests: maximum_requests,
        processed_request_refs: &processed_request_refs,
        dispatch_receipt_refs: &dispatch_receipt_refs,
        has_stopped,
        diagnostics: &diagnostics,
    })?;
    let loop_receipt_ref = crate::preserves_rail::canonical_hash(&loop_value)?;
    super::storage::write_preserves(root, &super::paths::control_loop_receipt_path(&loop_receipt_ref)?, &loop_value)?;
    super::storage::import_artifact(root, &loop_value)?;
    Ok(super::outcomes::ControlLoop {
        loop_receipt_ref,
        loop_receipt_value: loop_value,
        heartbeat_receipt_ref,
        heartbeat_receipt_value: heartbeat_value,
        processed_request_refs,
        dispatch_receipt_refs,
        has_stopped,
        is_denied,
    })
}

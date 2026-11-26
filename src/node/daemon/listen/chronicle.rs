//! Read-only control history over delivery and control receipts.

/// The most entries one control history read lists in each section.
const MAX_CONTROL_HISTORY_ENTRIES: usize = 4096;

const DELIVER_RECEIPT_SUFFIX: &str = ".deliver.receipt.preserves";

const CONTROL_RECEIPT_SUFFIX: &str = ".control-receipt.preserves";

#[derive(Debug, Clone, Copy)]
pub struct ControlHistoryInput<'a> {
    pub state_root: &'a std::path::Path,
}

/// The latest delivery outcome the node recorded for one received envelope.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ControlHistoryDelivery {
    pub envelope_ref: String,
    pub request_ref: String,
    pub from_peer: String,
    pub sequence: u64,
    pub decision: String,
    pub phase: String,
}

/// One dispatched request and its control receipt.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ControlHistoryProcessed {
    pub request_ref: String,
    pub decision: String,
    pub receipt_ref: String,
}

/// The node's control history: deliveries, pending requests, and processed
/// requests, each in canonical reference order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ControlHistory {
    pub deliveries: Vec<ControlHistoryDelivery>,
    pub pending_request_refs: Vec<String>,
    pub processed: Vec<ControlHistoryProcessed>,
}

/// Read the node's control history without writing to the state root.
// r[impl molten.outside_in.observation]
pub fn control_history_local(input: &ControlHistoryInput<'_>) -> crate::error::Result<ControlHistory> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open_existing(input.state_root)?;
    control_history_with_root(&root)
}

pub fn control_history_with_root(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<ControlHistory> {
    let mut deliveries = history_deliveries(root)?;
    deliveries.sort();
    let mut pending_request_refs = super::leases::pending_control_requests(root)?
        .into_iter()
        .map(|pending| pending_request_ref(root, &pending))
        .collect::<crate::error::Result<Vec<_>>>()?;
    pending_request_refs.sort();
    let mut processed = history_processed(root)?;
    processed.sort();
    Ok(ControlHistory {
        deliveries,
        pending_request_refs,
        processed,
    })
}

fn pending_request_ref(
    root: &crate::node_state::DirectoryRoot,
    pending: &super::leases::PendingControlRequest,
) -> crate::error::Result<String> {
    let bytes = root.control_inbox()?.read_entry(&pending.entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    crate::preserves_rail::canonical_hash(&history_value(bytes, "node control inbox request")?)
}

fn history_deliveries(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Vec<ControlHistoryDelivery>> {
    let receipts_path = super::storage::fixed_node_path(super::settings::CONTROL_INGRESS_DIR)?.join("receipts")?;
    if !root.try_exists(&receipts_path)? {
        return Ok(Vec::new());
    }
    let receipts = root.control_ingress()?.open_subdir(&crate::node_state::RelativePath::parse("receipts")?)?;
    history_entries(&receipts, DELIVER_RECEIPT_SUFFIX)?
        .iter()
        .map(|entry| {
            let bytes = receipts.read_entry(entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
            let receipt =
                super::receipts::parse_control_ingress_receipt(&history_value(bytes, "node control ingress receipt")?)?;
            Ok(ControlHistoryDelivery {
                envelope_ref: receipt.envelope_ref,
                request_ref: receipt.request_ref,
                from_peer: receipt.from_peer,
                sequence: receipt.sequence,
                decision: receipt.decision,
                phase: receipt.phase,
            })
        })
        .collect()
}

fn history_processed(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Vec<ControlHistoryProcessed>> {
    let outbox = root.control_outbox()?;
    history_entries(&outbox, CONTROL_RECEIPT_SUFFIX)?
        .iter()
        .map(|entry| {
            let bytes = outbox.read_entry(entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
            let receipt = crate::node_runtime::parse_control_receipt(&history_value(bytes, "node control receipt")?)?;
            Ok(ControlHistoryProcessed {
                request_ref: receipt.request_ref,
                decision: receipt.decision,
                receipt_ref: receipt.receipt_ref,
            })
        })
        .collect()
}

fn history_entries(
    scope: &crate::node_state::Scope,
    suffix: &str,
) -> crate::error::Result<Vec<crate::node_state::Entry>> {
    let mut selected = Vec::new();
    for entry in scope.list_entries()? {
        if entry.kind != crate::node_state::EntryKind::RegularFile || !entry.name.ends_with(suffix) {
            continue;
        }
        if selected.len() >= MAX_CONTROL_HISTORY_ENTRIES {
            return Err(crate::error::Failure::invalid_harness("node control history entry bound exceeded"));
        }
        selected.push(entry);
    }
    Ok(selected)
}

fn history_value(bytes: Vec<u8>, label: &str) -> crate::error::Result<preserves::IOValue> {
    let text = String::from_utf8(bytes)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("{label} is not UTF-8: {error}")))?;
    crate::preserves_rail::parse_text(&text)
}

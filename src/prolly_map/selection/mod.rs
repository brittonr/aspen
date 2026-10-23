// The configuration-selected Prolly store seam.
//
// The selection names the physical backend behind the Prolly block-store and
// retention ports. Redb is the default; the marble-plus-art spike is selected
// only by configuration and only when the `marble-object-store` feature is
// compiled. Selecting a backend changes no Prolly semantics: both serve the
// same ports, and the core decides every map, publication, and GC question.

use molten_core::marble_store::*;

use super::*;

mod dispatch;

/// The schema of the generated store selection.
pub const PROLLY_STORE_SELECTION_SCHEMA: &str = "molten.prolly-store-selection.v1";

/// The physical backend behind the Prolly ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProllyStoreBackend {
    /// The current Redb store: the default.
    Redb,
    /// The marble-plus-art spike.
    MarbleArt,
}

/// One reviewed store selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProllyStoreSelection {
    pub backend: ProllyStoreBackend,
    pub marble: MarbleStoreProfile,
}

/// Where each backend keeps its state.
pub struct ProllyStoreRoots<'a> {
    /// The node-state storage namespace of the Redb store.
    pub storage: &'a molten_node_host::node_state::Scope,
    /// The heap directory of the marble store; marble opens it by path.
    pub marble_heap: Option<&'a std::path::Path>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionDocument {
    schema: String,
    backend: String,
    marble: MarbleDocument,
    non_claims: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MarbleDocument {
    catalog_ids: RangeDocument,
    block_ids: RangeDocument,
    maintenance_every_committed_batches: u32,
    executor_queue_depth: u32,
    read_cache_entries: u32,
    max_object_bytes: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RangeDocument {
    start: u64,
    end: u64,
}

/// Parse the generated store selection JSON and validate its marble profile.
// r[impl aspen.marble_store.spike]
pub fn parse_prolly_store_selection(json: &str) -> ProllyPortResult<ProllyStoreSelection> {
    let document: SelectionDocument = serde_json::from_str(json)
        .map_err(|error| ProllyPortError::new("prolly-store-selection", error.to_string(), false))?;
    if document.schema != PROLLY_STORE_SELECTION_SCHEMA {
        return Err(selection_error("the store selection schema is not supported"));
    }
    if document.non_claims.is_empty() {
        return Err(selection_error("the store selection states no non-claims"));
    }
    let backend = match document.backend.as_str() {
        "redb" => ProllyStoreBackend::Redb,
        "marble-art" => ProllyStoreBackend::MarbleArt,
        _ => return Err(selection_error("the store backend is not recognized")),
    };
    let marble = MarbleStoreProfile {
        layout: ObjectIdLayout {
            catalog: ObjectIdRange {
                start: document.marble.catalog_ids.start,
                end: document.marble.catalog_ids.end,
            },
            blocks: ObjectIdRange {
                start: document.marble.block_ids.start,
                end: document.marble.block_ids.end,
            },
        },
        maintenance: MaintenanceSchedule {
            every_committed_batches: document.marble.maintenance_every_committed_batches,
        },
        executor_queue_depth: document.marble.executor_queue_depth,
        read_cache_entries: document.marble.read_cache_entries,
        max_object_bytes: document.marble.max_object_bytes,
    };
    if !validate_store_profile(&marble).is_empty() {
        return Err(selection_error("the marble store profile is out of bounds"));
    }
    Ok(ProllyStoreSelection { backend, marble })
}

/// The store a selection opened.
pub enum SelectedProllyStore {
    Redb(LocalProllyBlockStore),
    #[cfg(feature = "marble-object-store")]
    MarbleArt(super::marble::MarbleArtBlockStore),
}

/// Open the selected backend.
pub fn open_prolly_store(
    roots: &ProllyStoreRoots<'_>,
    selection: &ProllyStoreSelection,
) -> ProllyPortResult<SelectedProllyStore> {
    match selection.backend {
        ProllyStoreBackend::Redb => LocalProllyBlockStore::open(roots.storage).map(SelectedProllyStore::Redb),
        ProllyStoreBackend::MarbleArt => open_marble(roots, selection),
    }
}

#[cfg(feature = "marble-object-store")]
fn open_marble(
    roots: &ProllyStoreRoots<'_>,
    selection: &ProllyStoreSelection,
) -> ProllyPortResult<SelectedProllyStore> {
    let heap = roots.marble_heap.ok_or_else(|| selection_error("the marble store needs a heap directory"))?;
    super::marble::MarbleArtBlockStore::open(heap, &selection.marble).map(SelectedProllyStore::MarbleArt)
}

#[cfg(not(feature = "marble-object-store"))]
fn open_marble(
    _roots: &ProllyStoreRoots<'_>,
    _selection: &ProllyStoreSelection,
) -> ProllyPortResult<SelectedProllyStore> {
    Err(ProllyPortError::new(
        "prolly-store-backend-unavailable",
        "the marble-object-store feature is not compiled",
        false,
    ))
}

fn selection_error(detail: &'static str) -> ProllyPortError {
    ProllyPortError::new("prolly-store-selection", detail, false)
}

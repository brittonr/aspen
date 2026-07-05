// Dataspace watch informers — revision cursors, watch events, informer
// snapshots, and selector authority bounds.
//
// Pure core DTOs and validation for resource watch streams built on
// Molten's dataspace model. Borrows the list/watch/informer idea from
// Kubernetes but stays in canonical Preserves/capability/replay terms.
//
// Type aliases and common helpers are inherited from p000.

const WATCH_EVENT_SCHEMA: &str = "molten.resource.watch-event.v1";
const INFORMER_SNAPSHOT_SCHEMA: &str = "molten.resource.informer-snapshot.v1";

const MAX_WATCH_EVENTS: usize = 4096;
const MAX_CURSOR_DELTA: u64 = 1_000_000;
const MAX_SELECTOR_LABELS: usize = 64;
const MAX_SELECTOR_FIELDS: usize = 16;
const _: () = assert!(MAX_WATCH_EVENTS > 0);
const _: () = assert!(MAX_CURSOR_DELTA > 0);

// ---------------------------------------------------------------------------
// Watch event DTOs
// ---------------------------------------------------------------------------

/// Kind of resource watch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WatchEventKind {
    Added,
    Modified,
    Deleted,
    Bookmark,
    Compacted,
}

impl WatchEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WatchEventKind::Added => "added",
            WatchEventKind::Modified => "modified",
            WatchEventKind::Deleted => "deleted",
            WatchEventKind::Bookmark => "bookmark",
            WatchEventKind::Compacted => "compacted",
        }
    }
}

/// Revision cursor — a deterministic replay boundary, not a wall-clock timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RevisionCursor {
    pub cursor: u64,
}

impl RevisionCursor {
    pub fn new(cursor: u64) -> Self {
        Self { cursor }
    }

    pub fn next(self) -> Self {
        Self {
            cursor: self.cursor.saturating_add(1),
        }
    }

    pub fn distance(self, other: RevisionCursor) -> u64 {
        if other.cursor >= self.cursor {
            other.cursor - self.cursor
        } else {
            self.cursor - other.cursor
        }
    }
}

/// Watch event binding resource ref, generation, cursor, authority, and evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    pub resource_ref: String,
    pub resource_type: String,
    pub scope_ref: String,
    pub generation: u64,
    pub kind: WatchEventKind,
    pub prior_cursor: RevisionCursor,
    pub next_cursor: RevisionCursor,
    pub admission_receipt_refs: Vec<String>,
    pub selector_refs: Vec<String>,
    pub observer_authority_refs: Vec<String>,
    pub event_body_ref: String,
    pub evidence_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Watch selector DTOs
// ---------------------------------------------------------------------------

/// A watch selector backed by explicit observer authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchSelector {
    pub scope_ref: String,
    pub resource_types: Vec<String>,
    pub label_selectors: Vec<LabelSelector>,
    pub field_selectors: Vec<FieldSelector>,
    pub is_cross_scope: bool,
}

/// Label-based selector for resource filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelSelector {
    pub key: String,
    pub operator: SelectorOperator,
    pub values: Vec<String>,
}

/// Field-based selector for resource filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSelector {
    pub field: String,
    pub operator: SelectorOperator,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorOperator {
    Equals,
    NotEquals,
    In,
    NotIn,
    Exists,
    DoesNotExist,
}

impl SelectorOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            SelectorOperator::Equals => "equals",
            SelectorOperator::NotEquals => "not-equals",
            SelectorOperator::In => "in",
            SelectorOperator::NotIn => "not-in",
            SelectorOperator::Exists => "exists",
            SelectorOperator::DoesNotExist => "does-not-exist",
        }
    }
}

// ---------------------------------------------------------------------------
// Informer snapshot DTOs
// ---------------------------------------------------------------------------

/// An informer snapshot that proves list/watch cache consistency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformerSnapshot {
    pub initial_list_ref: String,
    pub starting_cursor: RevisionCursor,
    pub applied_watch_event_refs: Vec<String>,
    pub final_cursor: RevisionCursor,
    pub selector_refs: Vec<String>,
    pub observer_authority_refs: Vec<String>,
    pub cache_state_ref: String,
}

/// Input to informer snapshot validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformerValidationInput {
    pub initial_list_ref: String,
    pub starting_cursor: RevisionCursor,
    pub watch_events: Vec<WatchEvent>,
    pub final_cursor: RevisionCursor,
    pub snapshot: InformerSnapshot,
}

/// Result of informer validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformerValidationResult {
    pub pass: bool,
    pub cache_current: bool,
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pure core: watch event validation
// ---------------------------------------------------------------------------

/// Validate an ordered sequence of watch events.
pub fn validate_watch_events(events: &[WatchEvent]) -> Result<Vec<String>> {
    if events.is_empty() {
        return Ok(Vec::new());
    }
    if events.len() > MAX_WATCH_EVENTS {
        return Err(MoltenError::invalid_harness(format!(
            "watch event count {} exceeds maximum {MAX_WATCH_EVENTS}",
            events.len()
        )));
    }

    // Validate each event individually
    for (i, event) in events.iter().enumerate() {
        require_ref(&event.resource_ref, "watch event resource ref")?;
        validate_non_empty(&event.resource_type, "watch event resource type")?;
        require_ref(&event.scope_ref, "watch event scope ref")?;
        if event.generation == 0 {
            return Err(MoltenError::invalid_harness(format!(
                "watch event {} has zero generation", i
            )));
        }
        require_ref(&event.event_body_ref, "watch event body ref")?;

        if event.next_cursor.cursor <= event.prior_cursor.cursor {
            return Err(MoltenError::invalid_harness(format!(
                "watch event {} has non-advancing cursor: {} -> {}",
                i, event.prior_cursor.cursor, event.next_cursor.cursor,
            )));
        }
    }

    // Validate ordering: cursors must advance monotonically
    for window in events.windows(2) {
        let prev = &window[0];
        let curr = &window[1];
        if curr.prior_cursor != prev.next_cursor {
            return Err(MoltenError::invalid_harness(format!(
                "cursor gap between event {} -> {}: expected prior {} but got {}",
                prev.next_cursor.cursor,
                curr.next_cursor.cursor,
                prev.next_cursor.cursor,
                curr.prior_cursor.cursor,
            )));
        }
    }

    let event_refs: Vec<String> = events.iter().map(|e| e.event_body_ref.clone()).collect();
    Ok(event_refs)
}

/// Validate that a cursor is not stale (too far behind).
pub fn validate_cursor_freshness(
    resume_cursor: RevisionCursor,
    retained_window_start: RevisionCursor,
) -> Result<()> {
    if resume_cursor.cursor < retained_window_start.cursor {
        return Err(MoltenError::invalid_harness(format!(
            "stale cursor {} before retained window start {}, must relist",
            resume_cursor.cursor, retained_window_start.cursor,
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pure core: selector validation
// ---------------------------------------------------------------------------

/// Validate a watch selector against observer authority.
pub fn validate_selector_authority(
    selector: &WatchSelector,
    has_cross_scope_authority: bool,
    has_label_authority: &[String],
) -> Result<()> {
    if selector.is_cross_scope && !has_cross_scope_authority {
        return Err(MoltenError::invalid_harness(
            "cross-scope selector denied: missing cross-scope authority",
        ));
    }

    if selector.label_selectors.len() > MAX_SELECTOR_LABELS {
        return Err(MoltenError::invalid_harness(format!(
            "label selector count {} exceeds maximum {MAX_SELECTOR_LABELS}",
            selector.label_selectors.len()
        )));
    }

    if selector.field_selectors.len() > MAX_SELECTOR_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "field selector count {} exceeds maximum {MAX_SELECTOR_FIELDS}",
            selector.field_selectors.len()
        )));
    }

    // Each label selector must be authorized
    for label in &selector.label_selectors {
        if !has_label_authority.contains(&label.key) {
            return Err(MoltenError::invalid_harness(format!(
                "unauthorized label selector: {}", label.key
            )));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pure core: informer snapshot validation
// ---------------------------------------------------------------------------

/// Validate informer snapshot consistency.
pub fn validate_informer_snapshot(input: &InformerValidationInput) -> InformerValidationResult {
    // Validate that applied events match declared events
    if input.snapshot.applied_watch_event_refs.len() != input.watch_events.len() {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "event count mismatch: snapshot declares {} events but {} were provided",
                input.snapshot.applied_watch_event_refs.len(),
                input.watch_events.len(),
            )],
        };
    }

    // Validate starting cursor matches
    if input.snapshot.starting_cursor != input.starting_cursor {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "starting cursor mismatch: snapshot {:?} vs input {:?}",
                input.snapshot.starting_cursor, input.starting_cursor,
            )],
        };
    }

    // Validate cursor advances through all events
    let mut expected_cursor = input.starting_cursor;
    for (i, event) in input.watch_events.iter().enumerate() {
        if event.prior_cursor != expected_cursor {
            return InformerValidationResult {
                pass: false,
                cache_current: false,
                diagnostics: vec![format!(
                    "event {} cursor mismatch: expected {:?} but got {:?}",
                    i, expected_cursor, event.prior_cursor,
                )],
            };
        }
        expected_cursor = event.next_cursor;
    }

    // Validate final cursor
    if expected_cursor != input.final_cursor {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "final cursor mismatch: expected {:?} after all events but got {:?}",
                expected_cursor, input.final_cursor,
            )],
        };
    }

    // Validate snapshot final cursor matches
    if input.snapshot.final_cursor != input.final_cursor {
        return InformerValidationResult {
            pass: false,
            cache_current: false,
            diagnostics: vec![format!(
                "snapshot final cursor {:?} does not match expected {:?}",
                input.snapshot.final_cursor, input.final_cursor,
            )],
        };
    }

    InformerValidationResult {
        pass: true,
        cache_current: true,
        diagnostics: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn watch_event_to_value(event: &WatchEvent) -> IoValue {
    record("watch-event-v1", vec![
        string(&event.resource_ref),
        string(&event.resource_type),
        string(&event.scope_ref),
        u64_value(event.generation),
        string(event.kind.as_str()),
        u64_value(event.prior_cursor.cursor),
        u64_value(event.next_cursor.cursor),
        refs_sequence(&event.admission_receipt_refs),
        refs_sequence(&event.selector_refs),
        refs_sequence(&event.observer_authority_refs),
        string(&event.event_body_ref),
        refs_sequence(&event.evidence_refs),
    ])
}
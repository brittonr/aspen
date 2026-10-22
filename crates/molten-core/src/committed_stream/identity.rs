//! The stable identities of the committed-change stream.

use std::collections::BTreeSet;

use super::*;

const EVENT_IDENTITY_CONTEXT: &str = "onixresearch.molten.committed-stream.event.identity.v1";
const CURSOR_IDENTITY_CONTEXT: &str = "onixresearch.molten.committed-stream.cursor.identity.v1";

/// The identity of a committed change event.
///
/// The identity binds the service, the epoch, the revision, the ordinal, and
/// the payload reference, so a replayed event keeps one identity.
pub fn committed_change_identity(event: &CommittedChangeEvent) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(EVENT_IDENTITY_CONTEXT);
    for field in [
        event.service_id.as_str(),
        &event.stream_epoch.to_string(),
        &event.revision.to_string(),
        &event.ordinal.to_string(),
        event.payload_ref.as_str(),
    ] {
        let length = u64::try_from(field.len()).unwrap_or(u64::MAX);
        hasher.update(&length.to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

/// The identity of a stream cursor.
pub fn stream_cursor_identity(cursor: &StreamCursor) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(CURSOR_IDENTITY_CONTEXT);
    for field in [
        cursor.service_id.as_str(),
        &cursor.stream_epoch.to_string(),
        &cursor.revision.to_string(),
        &cursor.ordinal.to_string(),
    ] {
        let length = u64::try_from(field.len()).unwrap_or(u64::MAX);
        hasher.update(&length.to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

// r[impl molten.observations.authorization]
/// Remove the repeated event identities from a replayed sequence.
///
/// The first occurrence of an identity wins, so the result keeps the commit
/// order. Deduplication never authorizes: an event that was already seen is
/// still an event that a reader must admit.
pub fn deduplicate_events(events: &[CommittedChangeEvent]) -> DeduplicationOutcome {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut unique = Vec::with_capacity(events.len());
    let mut duplicate_refs = Vec::new();
    for event in events {
        if seen.insert(event.event_ref.as_str()) {
            unique.push(event.clone());
        } else {
            duplicate_refs.push(event.event_ref.clone());
        }
    }
    DeduplicationOutcome {
        events: unique,
        duplicate_refs,
    }
}

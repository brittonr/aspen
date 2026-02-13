//! Helper functions for Automerge handlers.

use aspen_automerge::DocumentMetadata;
use aspen_client_api::AutomergeDocumentMetadata;

/// Convert internal DocumentMetadata to API response type.
pub(crate) fn convert_metadata(meta: DocumentMetadata) -> AutomergeDocumentMetadata {
    AutomergeDocumentMetadata {
        document_id: meta.id.to_string(),
        namespace: meta.namespace,
        title: meta.title,
        description: meta.description,
        created_at_ms: meta.created_at_ms,
        updated_at_ms: meta.updated_at_ms,
        size_bytes: meta.size_bytes,
        change_count: meta.change_count,
        heads: meta.heads,
        creator_actor_id: meta.creator_actor_id,
        tags: meta.tags,
    }
}

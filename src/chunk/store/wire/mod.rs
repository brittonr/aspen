//! Canonical Preserves codec for manifests, chunk refs, transforms, and receipts.

pub(super) mod decode;
pub(super) mod encode;
pub(super) mod fields;
pub(super) mod outcome;
pub(super) mod policy;

const CHUNK_TRANSFORMS_SCHEMA: &str = "molten.chunk-store.transforms.v1";

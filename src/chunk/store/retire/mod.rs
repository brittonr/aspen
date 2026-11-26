//! Chunk garbage collection. Unpinned manifests and unreachable chunks are deleted only when
//! retention admission, evaluation, and (outside dry runs) the execution gate for a matching apply
//! ref all pass.

mod admission;
pub(super) mod sweep;

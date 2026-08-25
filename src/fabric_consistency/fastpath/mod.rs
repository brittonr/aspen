#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "pure model validators borrow bounded immutable profiles and traces"
)]
#![allow(
    tigerstyle::non_trait_imports,
    reason = "the bounded model keeps deterministic collection and sibling component types explicit within this isolated module"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public fast-path model types retain domain names that remain clear outside the module"
)]
#![allow(
    tigerstyle::unbounded_collection_growth,
    reason = "profile admission and fault-corpus constants impose finite model bounds before exploration"
)]
#![allow(
    tigerstyle::usize_in_public_api,
    reason = "model-only collection counts and sequence indexes use the host collection index type and never cross a wire boundary"
)]

mod conflict;
mod evidence;
mod fault;
mod profile;
mod recovery;
mod stable;

pub use conflict::*;
pub use evidence::*;
pub use fault::*;
pub use profile::*;
pub use recovery::*;
pub use stable::*;

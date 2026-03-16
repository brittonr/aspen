//! Streaming output buffer with head/tail truncation and incremental search.

pub mod streaming_output;
pub mod output_search;

pub use streaming_output::{StreamingConfig, StreamingOutput, StreamingOutputManager};
pub use output_search::{OutputSearch, SearchMatch, SearchMode, apply_search_highlights};

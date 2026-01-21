//! HTTP endpoint handlers for the Nix binary cache protocol.

pub mod cache_info;
pub mod nar;
pub mod narinfo;

pub use cache_info::handle_cache_info;
pub use nar::{HttpRange, NarDownload, extract_blob_hash, prepare_nar_download};
pub use narinfo::{extract_store_hash, handle_narinfo};

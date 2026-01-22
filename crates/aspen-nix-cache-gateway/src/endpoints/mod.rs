//! HTTP endpoint handlers for the Nix binary cache protocol.

pub mod cache_info;
pub mod nar;
pub mod narinfo;

pub use cache_info::handle_cache_info;
pub use nar::HttpRange;
pub use nar::NarDownload;
pub use nar::extract_blob_hash;
pub use nar::prepare_nar_download;
pub use narinfo::extract_store_hash;
pub use narinfo::handle_narinfo;

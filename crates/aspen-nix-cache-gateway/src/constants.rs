//! Tiger Style constants for the Nix cache gateway.
//!
//! All resource bounds are explicit and documented.

use std::time::Duration;

/// Chunk size for streaming NAR responses (64 KB).
///
/// Balances syscall overhead against memory usage. Larger chunks reduce
/// syscalls but increase memory per concurrent stream.
pub const STREAM_CHUNK_SIZE: usize = 64 * 1024;

/// Maximum duration for a single NAR streaming transfer (10 minutes).
///
/// Prevents indefinite transfers from blocking resources. Large NARs
/// (several GB) may need this limit increased.
pub const MAX_STREAM_DURATION: Duration = Duration::from_secs(600);

/// Maximum concurrent HTTP/3 connections.
///
/// Limits resource usage from the gateway. Each connection can have
/// multiple streams.
pub const MAX_GATEWAY_CONNECTIONS: u32 = 100;

/// Maximum concurrent streams per connection.
///
/// Limits parallel requests from a single client.
pub const MAX_STREAMS_PER_CONNECTION: u32 = 100;

/// Request timeout for non-streaming requests (30 seconds).
///
/// Used for /nix-cache-info and /{hash}.narinfo endpoints.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Default cache priority (30).
///
/// Lower values have higher priority. Official cache.nixos.org uses 40,
/// so local caches typically use 30 to be preferred.
pub const DEFAULT_CACHE_PRIORITY: u32 = 30;

/// Default store directory.
pub const DEFAULT_STORE_DIR: &str = "/nix/store";

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Chunk size must be positive and power of 2 for efficiency
const _: () = assert!(STREAM_CHUNK_SIZE > 0);
const _: () = assert!(STREAM_CHUNK_SIZE.count_ones() == 1); // power of 2

// Connection limits must be positive
const _: () = assert!(MAX_GATEWAY_CONNECTIONS > 0);
const _: () = assert!(MAX_STREAMS_PER_CONNECTION > 0);

// Priority must be reasonable (lower = higher priority)
const _: () = assert!(DEFAULT_CACHE_PRIORITY > 0);

//! Constants for SOPS operations.
//!
//! All limits follow Tiger Style: fixed bounds to prevent resource exhaustion.

/// Maximum SOPS file size (1 MB).
pub const MAX_SOPS_FILE_SIZE: usize = 1_048_576;

/// Maximum number of values in a SOPS file.
pub const MAX_VALUE_COUNT: u32 = 10_000;

/// Maximum key path length in bytes.
pub const MAX_KEY_PATH_LENGTH: u32 = 512;

/// Default Transit mount point.
pub const DEFAULT_TRANSIT_MOUNT: &str = "transit";

/// Default Transit key name for SOPS data key encryption.
pub const DEFAULT_TRANSIT_KEY: &str = "sops-data-key";

/// Default Unix socket path for the gRPC key service.
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/aspen-sops.sock";

/// SOPS version string written to metadata.
pub const SOPS_VERSION: &str = "3.9.0";

/// AES-256-GCM nonce size in bytes.
pub const AES_GCM_NONCE_SIZE: usize = 12;

/// AES-256-GCM tag size in bytes.
pub const AES_GCM_TAG_SIZE: usize = 16;

/// Data key size in bits for Transit generation.
pub const DATA_KEY_BITS: u32 = 256;

/// Data key size in bytes.
pub const DATA_KEY_BYTES: usize = 32;

/// Environment variable for cluster ticket.
pub const ENV_CLUSTER_TICKET: &str = "ASPEN_CLUSTER_TICKET";

// ============================================================================
// Compile-Time Constant Assertions (Tiger Style)
// ============================================================================

const _: () = assert!(MAX_SOPS_FILE_SIZE > 0);
const _: () = assert!(MAX_VALUE_COUNT > 0);
const _: () = assert!(MAX_KEY_PATH_LENGTH > 0);
const _: () = assert!(AES_GCM_NONCE_SIZE == 12);
const _: () = assert!(AES_GCM_TAG_SIZE == 16);
const _: () = assert!(DATA_KEY_BITS == 256);
const _: () = assert!(DATA_KEY_BYTES == 32);

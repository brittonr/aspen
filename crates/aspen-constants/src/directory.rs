//! Directory Layer constants following Tiger Style principles.
//!
//! These constants define resource bounds for the Directory Layer implementation,
//! ensuring predictable resource usage and preventing unbounded operations.

// =============================================================================
// Directory Structure Limits
// =============================================================================

/// Maximum depth of directory paths.
/// Tiger Style: Bounded to prevent stack overflow in recursive operations.
pub const MAX_DIRECTORY_DEPTH: u32 = 32;

/// Maximum length of a single path component in bytes.
/// Tiger Style: Bounded to fit within key size limits.
pub const MAX_PATH_COMPONENT_LENGTH_BYTES: u32 = 256;

/// Maximum number of subdirectories returned by list().
/// Tiger Style: Bounded result set prevents memory exhaustion.
pub const MAX_LIST_RESULTS: u32 = 10_000;

/// Maximum directories to remove in recursive delete.
/// Tiger Style: Bounded to prevent runaway deletions.
pub const MAX_RECURSIVE_DELETE: u32 = 1_000;

// =============================================================================
// High-Contention Allocator (HCA) Configuration
// =============================================================================

/// Maximum candidates to try per allocation attempt before expanding window.
/// Tiger Style: Bounded iterations prevent infinite loops under contention.
pub const HCA_MAX_CANDIDATES_PER_ATTEMPT: u32 = 20;

/// Maximum window size for HCA.
/// Tiger Style: Upper bound on window to prevent unbounded scanning.
pub const HCA_MAX_WINDOW_SIZE: u64 = 8192;

/// Initial window size for new HCA instances.
/// Small window reduces conflicts when allocation count is low.
pub const HCA_INITIAL_WINDOW_SIZE: u64 = 64;

/// Medium window size threshold.
/// When counter >= 255, window expands to this size.
pub const HCA_MEDIUM_WINDOW_SIZE: u64 = 1024;

/// Counter threshold for medium window.
pub const HCA_MEDIUM_WINDOW_THRESHOLD: u64 = 255;

/// Counter threshold for large window.
pub const HCA_LARGE_WINDOW_THRESHOLD: u64 = 65535;

// =============================================================================
// Reserved Prefixes
// =============================================================================

/// Reserved prefix byte for directory layer metadata.
/// Per FoundationDB convention, 0xFE is reserved for directory layer internals.
/// All directory metadata (path mappings, HCA state) is stored under this prefix.
pub const DIRECTORY_PREFIX: u8 = 0xFE;

/// System prefix byte (0xFF) is reserved and should not be used.
/// This is already reserved by Aspen for internal system keys.
pub const SYSTEM_RESERVED_PREFIX: u8 = 0xFF;

// =============================================================================
// Key Prefixes (under DIRECTORY_PREFIX)
// =============================================================================

/// Prefix for directory node metadata within the directory subspace.
/// Format: (0xFE, "node", path..., field) -> value
pub const DIR_NODE_PREFIX: &str = "node";

/// Prefix for HCA allocator state within the directory subspace.
/// Format: (0xFE, "hca", field) -> value
pub const DIR_HCA_PREFIX: &str = "hca";

/// HCA counter key suffix.
pub const HCA_COUNTER_KEY: &str = "counter";

/// HCA window start key suffix.
pub const HCA_WINDOW_START_KEY: &str = "window_start";

/// HCA candidates prefix for claimed markers.
pub const HCA_CANDIDATES_PREFIX: &str = "candidates";

// =============================================================================
// Directory Metadata Fields
// =============================================================================

/// Field name for directory layer type.
pub const DIR_FIELD_LAYER: &str = "layer";

/// Field name for allocated prefix.
pub const DIR_FIELD_PREFIX: &str = "prefix";

/// Field name for creation timestamp.
pub const DIR_FIELD_CREATED_AT_MS: &str = "created_at_ms";

// =============================================================================
// Default Values
// =============================================================================

/// Default layer type for directories.
pub const DEFAULT_LAYER_TYPE: &str = "default";

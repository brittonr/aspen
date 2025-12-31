//! Error types for DNS operations.
//!
//! Provides specific error types for DNS record management and client operations.

use snafu::Snafu;

use super::types::RecordType;

/// Errors that can occur during DNS operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DnsError {
    /// Invalid domain name format.
    #[snafu(display("invalid domain name: {reason}"))]
    InvalidDomain {
        /// Description of why the domain is invalid.
        reason: String,
    },

    /// Invalid record data.
    #[snafu(display("invalid record data for {record_type}: {reason}"))]
    InvalidRecordData {
        /// The record type being validated.
        record_type: RecordType,
        /// Description of why the data is invalid.
        reason: String,
    },

    /// TTL value out of allowed range.
    #[snafu(display("TTL {ttl} out of range [{min}, {max}]"))]
    InvalidTtl {
        /// The provided TTL value.
        ttl: u32,
        /// Minimum allowed TTL.
        min: u32,
        /// Maximum allowed TTL.
        max: u32,
    },

    /// Record not found.
    #[snafu(display("record not found: {domain} {record_type}"))]
    RecordNotFound {
        /// The domain that was queried.
        domain: String,
        /// The record type that was queried.
        record_type: RecordType,
    },

    /// Zone not found.
    #[snafu(display("zone not found: {zone}"))]
    ZoneNotFound {
        /// The zone name that was queried.
        zone: String,
    },

    /// Zone already exists.
    #[snafu(display("zone already exists: {zone}"))]
    ZoneAlreadyExists {
        /// The zone name that already exists.
        zone: String,
    },

    /// Maximum zones exceeded.
    #[snafu(display("maximum zones ({max}) exceeded"))]
    MaxZonesExceeded {
        /// The maximum number of zones allowed.
        max: u32,
    },

    /// Maximum records per domain exceeded.
    #[snafu(display("maximum records per domain ({max}) exceeded for {domain}"))]
    MaxRecordsExceeded {
        /// The domain that exceeded the limit.
        domain: String,
        /// The maximum number of records allowed.
        max: u32,
    },

    /// Batch size exceeded.
    #[snafu(display("batch size ({size}) exceeds maximum ({max})"))]
    BatchSizeExceeded {
        /// The requested batch size.
        size: u32,
        /// The maximum allowed batch size.
        max: u32,
    },

    /// Serialization error.
    #[snafu(display("serialization error: {reason}"))]
    Serialization {
        /// Description of the serialization failure.
        reason: String,
    },

    /// Storage operation failed.
    #[snafu(display("storage error: {reason}"))]
    Storage {
        /// Description of the storage failure.
        reason: String,
    },

    /// CNAME conflict - CNAME records cannot coexist with other record types.
    #[snafu(display("CNAME conflict: {domain} already has {existing_type} records"))]
    CnameConflict {
        /// The domain with the conflict.
        domain: String,
        /// The existing record type that conflicts with CNAME.
        existing_type: RecordType,
    },

    /// Zone is disabled.
    #[snafu(display("zone {zone} is disabled"))]
    ZoneDisabled {
        /// The zone that is disabled.
        zone: String,
    },

    /// DNS server failed to start.
    #[snafu(display("DNS server failed to start: {reason}"))]
    ServerStart {
        /// Description of the server start failure.
        reason: String,
    },

    /// Invalid domain name.
    #[snafu(display("invalid domain '{domain}': {reason}"))]
    InvalidDomainWithName {
        /// The invalid domain name.
        domain: String,
        /// Description of why the domain is invalid.
        reason: String,
    },
}

/// Errors that can occur during DNS client operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DnsClientError {
    /// Failed to parse the ticket.
    #[snafu(display("invalid ticket: {reason}"))]
    InvalidTicket {
        /// Description of why the ticket is invalid.
        reason: String,
    },

    /// Connection to cluster failed.
    #[snafu(display("connection failed: {reason}"))]
    ConnectionFailed {
        /// Description of the connection failure.
        reason: String,
    },

    /// Sync operation failed.
    #[snafu(display("sync failed: {reason}"))]
    SyncFailed {
        /// Description of the sync failure.
        reason: String,
    },

    /// Sync operation timed out.
    #[snafu(display("sync timed out after {duration_ms}ms"))]
    SyncTimeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },

    /// Client is not synced yet.
    #[snafu(display("client not synced: current status is {status}"))]
    NotSynced {
        /// The current sync status.
        status: super::types::SyncStatus,
    },

    /// Maximum subscribers reached.
    #[snafu(display("max subscribers ({max}) exceeded"))]
    MaxSubscribers {
        /// The maximum number of subscribers allowed.
        max: usize,
    },

    /// Maximum cached records reached.
    #[snafu(display("cache capacity ({max}) reached"))]
    CacheCapacity {
        /// The maximum number of cached records.
        max: usize,
    },

    /// Record not found in cache.
    #[snafu(display("record not found in cache: {domain} {record_type}"))]
    CacheRecordNotFound {
        /// The domain that was queried.
        domain: String,
        /// The record type that was queried.
        record_type: RecordType,
    },

    /// Internal error.
    #[snafu(display("internal error: {reason}"))]
    Internal {
        /// Description of the internal error.
        reason: String,
    },
}

/// Result type for DNS server operations.
pub type DnsResult<T> = Result<T, DnsError>;

/// Result type for DNS client operations.
pub type DnsClientResult<T> = Result<T, DnsClientError>;

impl From<serde_json::Error> for DnsError {
    fn from(err: serde_json::Error) -> Self {
        DnsError::Serialization {
            reason: err.to_string(),
        }
    }
}

impl From<aspen_core::KeyValueStoreError> for DnsError {
    fn from(err: aspen_core::KeyValueStoreError) -> Self {
        DnsError::Storage {
            reason: err.to_string(),
        }
    }
}

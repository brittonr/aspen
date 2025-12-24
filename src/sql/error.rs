//! Error types for SQL operations on Redb.

use std::sync::Arc;

use snafu::Snafu;

/// Errors from SQL operations on Redb storage.
#[derive(Debug, Snafu)]
pub enum SqlError {
    /// Failed to begin a read transaction.
    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        /// The underlying transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to open a database table.
    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        /// The underlying table error.
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    /// Failed to read from storage.
    #[snafu(display("storage read failed: {source}"))]
    StorageRead {
        /// The underlying storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to deserialize a value.
    #[snafu(display("deserialization failed: {message}"))]
    Deserialize {
        /// Error message from deserialization.
        message: String,
    },

    /// DataFusion query execution error.
    #[snafu(display("DataFusion error: {source}"))]
    DataFusion {
        /// The underlying DataFusion error.
        source: Arc<datafusion::error::DataFusionError>,
    },

    /// Arrow error during batch construction.
    #[snafu(display("Arrow error: {source}"))]
    Arrow {
        /// The underlying Arrow error.
        source: Arc<arrow::error::ArrowError>,
    },

    /// Query timeout exceeded.
    #[snafu(display("query timed out after {duration_ms}ms"))]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },

    /// Internal error.
    #[snafu(display("internal error: {message}"))]
    Internal {
        /// Description of the internal error.
        message: String,
    },
}

impl From<datafusion::error::DataFusionError> for SqlError {
    fn from(err: datafusion::error::DataFusionError) -> Self {
        SqlError::DataFusion { source: Arc::new(err) }
    }
}

impl From<arrow::error::ArrowError> for SqlError {
    fn from(err: arrow::error::ArrowError) -> Self {
        SqlError::Arrow { source: Arc::new(err) }
    }
}

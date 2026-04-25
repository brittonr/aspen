//! Error types for RedbKvStorage operations.

use std::io;
use std::path::PathBuf;

use snafu::Snafu;

/// Errors from RedbKvStorage operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum SharedStorageError {
    #[snafu(display("failed to open redb database at {}: {source}", path.display()))]
    OpenDatabase {
        path: PathBuf,
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    #[snafu(display("failed to commit transaction: {source}"))]
    Commit {
        #[snafu(source(from(redb::CommitError, Box::new)))]
        source: Box<redb::CommitError>,
    },

    #[snafu(display("failed to insert into table: {source}"))]
    Insert {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to get from table: {source}"))]
    Get {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to remove from table: {source}"))]
    Remove {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to iterate table range: {source}"))]
    Range {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to serialize data: {source}"))]
    Serialize {
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize {
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("storage lock poisoned: {context}"))]
    LockPoisoned { context: String },

    #[snafu(display("batch size {} exceeds maximum {}", size, max))]
    BatchTooLarge { size: u32, max: u32 },

    #[snafu(display("{reason}"))]
    Internal { reason: String },
}

impl From<SharedStorageError> for io::Error {
    fn from(err: SharedStorageError) -> io::Error {
        io::Error::other(err.to_string())
    }
}

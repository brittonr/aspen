// The bounded read session of the local Prolly block store.
//
// One session holds one Redb read transaction and its blocks table for the
// whole session. The session enforces its block and byte bounds, refuses every
// request after it is closed or exhausted, and releases the transaction on
// close. No Redb type crosses the port.

use molten_core::prolly_map::*;
use redb::ReadableDatabase;

use super::store::*;
use super::*;

/// A session opens exactly one read transaction, when it is opened.
const SESSION_TRANSACTIONS: u32 = 1;

pub(super) struct LocalReadSession {
    transaction: Option<redb::ReadTransaction>,
    table: Option<redb::ReadOnlyTable<&'static str, &'static [u8]>>,
    bounds: ReadSessionBounds,
    blocks_read: u32,
    bytes_copied: u64,
    exhausted: bool,
}

impl LocalReadSession {
    pub(super) fn open(database: &redb::Database, bounds: ReadSessionBounds) -> ProllyPortResult<Self> {
        let transaction = database
            .begin_read()
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        let table = transaction
            .open_table(PROLLY_BLOCKS_TABLE)
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        Ok(Self {
            transaction: Some(transaction),
            table: Some(table),
            bounds,
            blocks_read: 0,
            bytes_copied: 0,
            exhausted: false,
        })
    }
}

impl ProllyReadSession for LocalReadSession {
    fn read_block(&mut self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        if self.exhausted {
            return Err(read_session_error("prolly-read-session-exhausted", "the read session bounds are exhausted"));
        }
        let Some(table) = self.table.as_ref() else {
            return Err(read_session_error("prolly-read-session-closed", "the read session is closed"));
        };
        if self.blocks_read >= self.bounds.max_blocks {
            self.exhausted = true;
            return Err(read_session_error(
                "prolly-read-session-block-bound",
                "the read session block bound is reached",
            ));
        }
        let bytes = table
            .get(node_ref.as_str())
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?
            .map(|guard| guard.value().to_vec());
        let length = match bytes.as_ref() {
            Some(bytes) => u64::try_from(bytes.len()).ok(),
            None => Some(0),
        };
        let copied = length.and_then(|length| self.bytes_copied.checked_add(length));
        let Some(copied) = copied.filter(|copied| *copied <= self.bounds.max_bytes) else {
            self.exhausted = true;
            return Err(read_session_error("prolly-read-session-byte-bound", "the read session byte bound is reached"));
        };
        self.blocks_read = self.blocks_read.saturating_add(1);
        self.bytes_copied = copied;
        Ok(bytes)
    }

    fn close(&mut self) -> ReadSessionObservation {
        self.table = None;
        self.transaction = None;
        ReadSessionObservation {
            blocks_read: self.blocks_read,
            bytes_copied: self.bytes_copied,
            transactions_opened: SESSION_TRANSACTIONS,
            end: if self.exhausted {
                ReadSessionEnd::BoundExhausted
            } else {
                ReadSessionEnd::Closed
            },
        }
    }
}

pub(super) fn read_session_error(code: &'static str, detail: &'static str) -> ProllyPortError {
    ProllyPortError::new(code, detail, false)
}


// Storage-fault handling for the Redb durable adapter.
//
// Every mutation reports one typed persistence observation to the pure core.
// A commit error is an unknown outcome, never a durable failure or success,
// and it quarantines this adapter instance. Only an explicit reopen with a
// read-back reconciliation, followed by a fresh observed synchronization of
// the reconciled content, returns the adapter to service. Retrying the call
// that failed proves nothing.

/// A Redb storage backend factory. Each reopen asks it for a fresh handle to
/// the same medium.
pub type RedbBackendFactory = std::sync::Arc<dyn Fn() -> Box<dyn redb::StorageBackend> + Send + Sync>;

/// Where the adapter's database lives.
#[derive(Clone)]
pub enum DatabaseSource {
    /// The store file under the capability-rooted durable store.
    File,
    /// A supplied Redb storage backend.
    Backend(RedbBackendFactory),
}

#[derive(Debug)]
struct BoxedBackend(Box<dyn redb::StorageBackend>);

impl redb::StorageBackend for BoxedBackend {
    fn len(&self) -> Result<u64, std::io::Error> {
        self.0.len()
    }

    fn read(&self, offset: u64, out: &mut [u8]) -> Result<(), std::io::Error> {
        self.0.read(offset, out)
    }

    fn set_len(&self, len: u64) -> Result<(), std::io::Error> {
        self.0.set_len(len)
    }

    fn sync_data(&self) -> Result<(), std::io::Error> {
        self.0.sync_data()
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<(), std::io::Error> {
        self.0.write(offset, data)
    }

    fn close(&self) -> Result<(), std::io::Error> {
        self.0.close()
    }
}

/// The state an explicit reopen reconciles against.
#[derive(Debug, Clone, Copy)]
pub enum ReconciliationReference<'a> {
    /// This adapter instance's last acknowledged state. An instance that
    /// opened over a repaired store has none.
    Acknowledged,
    /// A reference the caller established by other means, such as protocol
    /// recovery from peers.
    Supplied(&'a DurableState),
    /// No reference: accept the read-back only if it decodes, its recovery
    /// evaluation admits it, and its snapshot payloads match. A repair stays
    /// recorded in `is_repair_observed`.
    SelfConsistent,
}

/// The database could not be reopened; the adapter is gone.
#[derive(Debug)]
pub struct ReopenFailure {
    pub error: crate::error::Failure,
}

/// One persistence attempt: what storage observably did, and the error that
/// explains anything but an acknowledgement.
struct PersistenceAttempt {
    observation: PersistenceObservation,
    error: Option<crate::error::Failure>,
}

impl PersistenceAttempt {
    const fn synchronized() -> Self {
        Self {
            observation: PersistenceObservation::SyncObserved,
            error: None,
        }
    }

    const fn process_visible() -> Self {
        Self {
            observation: PersistenceObservation::AppliedProcessVisible,
            error: None,
        }
    }

    fn not_applied(error: crate::error::Failure) -> Self {
        Self {
            observation: PersistenceObservation::NotApplied,
            error: Some(error),
        }
    }
}

/// Run one immediate-durability Redb write transaction and report what storage
/// observably did. A failure before commit aborts the transaction, so nothing
/// applied. A commit error cannot prove non-application. A successful commit
/// observed its synchronization.
// r[impl molten.fabric_durability.storage_fault_outcomes]
fn write_observed(
    database: &redb::Database,
    fill: impl FnOnce(&redb::WriteTransaction) -> crate::error::Result<()>,
) -> PersistenceAttempt {
    let write = match database.begin_write() {
        Ok(write) => write,
        Err(error) => return PersistenceAttempt::not_applied(adapter_error(error)),
    };
    if let Err(error) = fill(&write) {
        drop(write);
        return PersistenceAttempt::not_applied(error);
    }
    match write.commit() {
        Ok(()) => PersistenceAttempt::synchronized(),
        Err(error) => PersistenceAttempt {
            observation: PersistenceObservation::OutcomeUnknown,
            error: Some(adapter_error(error)),
        },
    }
}

/// Open the database with a repair callback that records any local repair.
///
/// Redb rebuilds its allocator both when it initializes a new store and when
/// it recovers an existing store that was not shut down cleanly. Only the
/// second is a repair, so the callback records one only for a store that held
/// content before this open.
fn open_database(
    root: &crate::local_store::DurableStoreRoot,
    source: &DatabaseSource,
    is_repaired: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> crate::error::Result<redb::Database> {
    let mut builder = redb::Database::builder();
    match source {
        DatabaseSource::File => {
            let file = root.root().open_database_file(&crate::local_store::Locator::parse(STORE_FILE)?)?;
            let is_existing = file.metadata().map_err(adapter_error)?.len() > 0;
            arm_repair_flag(&mut builder, is_existing, is_repaired);
            builder.create_file(file).map_err(adapter_error)
        }
        DatabaseSource::Backend(factory) => {
            let backend = factory();
            let is_existing = backend.len().map_err(adapter_error)? > 0;
            arm_repair_flag(&mut builder, is_existing, is_repaired);
            builder.create_with_backend(BoxedBackend(backend)).map_err(adapter_error)
        }
    }
}

fn arm_repair_flag(
    builder: &mut redb::Builder,
    is_existing: bool,
    is_repaired: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    if is_existing {
        let flag = std::sync::Arc::clone(is_repaired);
        builder.set_repair_callback(move |_session| flag.store(true, std::sync::atomic::Ordering::Release));
    }
}

/// Rewrite the whole durable content in one fresh transaction, so the
/// reconciled state is durable by this transaction's own observed
/// synchronization rather than by any earlier failed one.
fn rewrite_durable_state(database: &redb::Database, state: &DurableState) -> PersistenceAttempt {
    write_observed(database, |write| {
        rewrite_table(write, LOG_TABLE, state.durable_log.iter().map(|record| Ok((record.sequence, encode_log_record(record)?))))?;
        rewrite_table(
            write,
            ORDERED_TABLE,
            state.ordered.iter().map(|(key, value)| Ok((key.as_slice(), encode_versioned_value(value)?))),
        )?;
        rewrite_table(
            write,
            SNAPSHOT_TABLE,
            state.snapshots.iter().map(|(reference, snapshot)| Ok((reference.as_str(), encode_snapshot(snapshot)?))),
        )?;
        rewrite_table(
            write,
            EFFECT_TABLE,
            state.effects.iter().map(|(identifier, effect)| Ok((identifier.as_str(), encode_effect(effect)?))),
        )
    })
}

fn rewrite_table<'k, K>(
    write: &redb::WriteTransaction,
    definition: redb::TableDefinition<K, &[u8]>,
    rows: impl Iterator<Item = crate::error::Result<(K::SelfType<'k>, Vec<u8>)>>,
) -> crate::error::Result<()>
where
    K: redb::Key + 'static,
{
    write.delete_table(definition).map_err(adapter_error)?;
    let mut table = write.open_table(definition).map_err(adapter_error)?;
    for row in rows {
        let (key, value) = row?;
        table.insert(key, value.as_slice()).map_err(adapter_error)?;
    }
    Ok(())
}

fn quarantine_error(reason: QuarantineReason) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("durable adapter is quarantined: {reason:?}"))
}

impl RedbDurableStateAdapter {
    /// The adapter's health. A quarantined adapter serves only status and reopen.
    pub const fn health(&self) -> AdapterHealth {
        self.health
    }

    /// Whether storage repaired its own structures at any open of this
    /// adapter. Reconciliation never clears it.
    pub const fn is_repair_observed(&self) -> bool {
        self.is_repair_observed
    }

    fn admit(&self, operation: AdapterOperation) -> crate::error::Result<()> {
        admit_adapter_operation(self.health, operation).map_err(quarantine_error)
    }

    /// Settle one admitted transition by its persistence attempt. Only an
    /// acknowledgement advances the served state.
    // r[impl molten.fabric_durability.adapter_quarantine]
    fn settle(
        &mut self,
        requested: DurabilityLevel,
        attempt: PersistenceAttempt,
        transition: DurableTransition,
    ) -> crate::error::Result<TransitionEnvelope> {
        let classification = classify_persistence(requested, attempt.observation);
        self.health = health_after(self.health, classification);
        if let PersistenceClassification::Acknowledged(_) = classification {
            let canonical = durable_transition(&self.profile, &transition)?;
            self.state = transition.next;
            return Ok(canonical);
        }
        if classification == PersistenceClassification::OutcomeUnknown {
            self.pending = Some(transition.next);
        }
        Err(attempt.error.unwrap_or_else(|| {
            crate::error::Failure::invalid_harness(format!("durable adapter persistence was {classification:?}"))
        }))
    }
}

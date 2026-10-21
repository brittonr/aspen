const STORE_PUBLICATIONS: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("delivery_publication_intents_v1");
const STORE_PUBLICATION_HEALTH: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("delivery_publication_health_v1");
const PUBLICATION_HEALTH_KEY: &str = "publication-adapter";
const MAX_PUBLICATION_RECORD_BYTES: usize = 16_384;

/// Capability-owned serialization and persistence for local publication effects.
/// Unknown persistence poisons this instance and latches separate quarantine evidence.
pub struct PublicationStore<'a> {
    root: &'a crate::local_store::DeliveryStoreRoot,
    _lease: std::fs::File,
    quarantine: transactional_reconciliation_core::QuarantineStatus,
}

#[derive(Debug, Clone)]
pub struct PublicationIntent {
    pub operation_ref: String,
    pub payload_ref: String,
    pub metadata: IoValue,
}

impl<'a> PublicationStore<'a> {
    pub fn open(root: &'a crate::local_store::DeliveryStoreRoot) -> Result<Self> {
        let lease = root.root().open_database_file(&crate::local_store::Locator::parse("publication.lock")?)?;
        lease.lock().map_err(store_error)?;
        let db = open_publication_database(root)?;
        let quarantine = read_publication_health(&db)?;
        Ok(Self { root, _lease: lease, quarantine })
    }

    pub fn quarantine_status(&self) -> transactional_reconciliation_core::QuarantineStatus {
        self.quarantine
    }

    pub fn require_healthy(&self) -> Result<()> {
        if self.quarantine == transactional_reconciliation_core::QuarantineStatus::Clear {
            Ok(())
        } else {
            Err(MoltenError::invalid_harness("publication storage is quarantined; explicit storage reconciliation is required"))
        }
    }

    // r[impl molten.audit_f03.recover_publication]
    pub fn reserve(&mut self, input: CheckRequest<'_>, metadata: &IoValue) -> Result<Decision> {
        self.require_healthy()?;
        let operation = validated_check_operation(input)?;
        let result = (|| {
            let db = open_publication_database(self.root)?;
            check_operation_in_store(&db, input, operation, Some(metadata))
        })();
        self.observe_persistence(result)
    }

    pub fn read(&mut self, operation_ref: &str, payload_ref: &str) -> Result<Option<PublicationIntent>> {
        self.require_healthy()?;
        require_ref(operation_ref, "publication operation ref")?;
        require_ref(payload_ref, "publication payload ref")?;
        let result = (|| {
            let db = open_publication_database(self.root)?;
            let read = db.begin_read().map_err(store_error)?;
            let table = read.open_table(STORE_PUBLICATIONS).map_err(store_error)?;
            let key = publication_key(operation_ref, payload_ref);
            let Some(bytes) = table.get(key.as_str()).map_err(store_error)? else { return Ok(None); };
            let intent = decode_publication_intent(bytes.value())?;
            if intent.operation_ref != operation_ref || intent.payload_ref != payload_ref {
                return Err(MoltenError::invalid_harness("publication intent key conflicts with canonical identity"));
            }
            Ok(Some(intent))
        })();
        self.observe_persistence(result)
    }

    pub fn for_payload(&mut self, payload_ref: &str) -> Result<Vec<PublicationIntent>> {
        self.require_healthy()?;
        require_ref(payload_ref, "publication payload ref")?;
        let result = (|| {
            let db = open_publication_database(self.root)?;
            read_publications_for_payload(&db, payload_ref)
        })();
        self.observe_persistence(result)
    }

    pub fn update(&mut self, intent: &PublicationIntent, metadata: &IoValue) -> Result<()> {
        self.require_healthy()?;
        let result = (|| {
            let db = open_publication_database(self.root)?;
            let write = db.begin_write().map_err(store_error)?;
            update_publication_intent(&write, intent, metadata)?;
            write.commit().map_err(store_error)
        })();
        self.observe_persistence(result)
    }

    // r[impl molten.audit_f03.uncertainty]
    pub fn quarantine(&mut self) -> Result<()> {
        self.quarantine = transactional_reconciliation_core::QuarantineStatus::Quarantined(
            transactional_reconciliation_core::QuarantineReason::CommitOutcomeUnknown,
        );
        let db = open_publication_database(self.root)?;
        let write = db.begin_write().map_err(store_error)?;
        {
            let mut table = write.open_table(STORE_PUBLICATION_HEALTH).map_err(store_error)?;
            table.insert(PUBLICATION_HEALTH_KEY, b"commit-outcome-unknown".as_slice()).map_err(store_error)?;
        }
        write.commit().map_err(store_error)
    }

    fn observe_persistence<T>(&mut self, result: Result<T>) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => match self.quarantine() {
                Ok(()) => Err(error),
                Err(quarantine_error) => Err(MoltenError::invalid_harness(format!(
                    "publication outcome unknown: {error}; quarantine persistence also failed: {quarantine_error}"
                ))),
            },
        }
    }
}

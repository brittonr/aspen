
// Explicit reopen and read-back reconciliation of the Redb durable adapter.

impl RedbDurableStateAdapter {
    /// Close and reopen the database, then reconcile the read-back.
    ///
    /// The adapter serves again only when the read-back equals the reference
    /// or the uncertain mutation's state, no repair ran during the reopen, and
    /// a fresh transaction rewriting that content observed its own
    /// synchronization. Otherwise it stays quarantined over the reopened store.
    // r[impl molten.fabric_durability.adapter_quarantine]
    pub fn reopen_and_reconcile(self, reference: ReconciliationReference<'_>) -> Result<Self, ReopenFailure> {
        let Self {
            profile,
            state,
            root,
            database,
            health,
            pending,
            source,
            publication,
            is_reference_trusted,
            is_repair_observed,
        } = self;
        drop(database);
        let is_repaired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let database = open_database(&root, &source, &is_repaired).map_err(|error| ReopenFailure { error })?;
        let mut reopened = Self {
            profile,
            state,
            root,
            database,
            health,
            pending,
            source,
            publication,
            is_reference_trusted,
            is_repair_observed,
        };
        let read_back = match load_state(&reopened.database, &reopened.profile.profile, reopened.state.descriptor.clone()) {
            Ok(read_back) => read_back,
            Err(_) => {
                reopened.health = AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent);
                return Ok(reopened);
            }
        };
        let is_repaired_now = is_repaired.load(std::sync::atomic::Ordering::Acquire);
        reopened.is_repair_observed |= is_repaired_now;
        let reconciled = match reference {
            ReconciliationReference::SelfConsistent => {
                let inventory = RecoveryInventory {
                    active_generation: read_back.descriptor.generation,
                    expected_schema_ref: read_back.descriptor.value_schema_ref.clone(),
                    permit_repair: false,
                    permit_quarantine: false,
                };
                reconcile_self_consistent(
                    evaluate_recovery(&read_back, &inventory).disposition,
                    snapshot_payloads_match(&reopened.root, &read_back),
                )
            }
            ReconciliationReference::Acknowledged | ReconciliationReference::Supplied(_) => {
                let acknowledged = match reference {
                    ReconciliationReference::Supplied(reference) => Some(reference),
                    _ => reopened.is_reference_trusted.then_some(&reopened.state),
                };
                reconcile_after_reopen(&ReconciliationInput {
                    acknowledged,
                    pending: reopened.pending.as_ref(),
                    read_back: &read_back,
                    is_repair_observed: is_repaired_now,
                })
            }
        };
        if let Err(reason) = reconciled {
            reopened.health = AdapterHealth::Quarantined(reason);
            return Ok(reopened);
        }
        if !snapshot_payloads_match(&reopened.root, &read_back) {
            reopened.health = AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent);
            return Ok(reopened);
        }
        let attempt = rewrite_durable_state(&reopened.database, &read_back);
        match classify_persistence(DurabilityLevel::MachineLoss, attempt.observation) {
            PersistenceClassification::Acknowledged(_) => {
                reopened.health = AdapterHealth::Serving;
                reopened.state = read_back;
                reopened.pending = None;
                reopened.is_reference_trusted = true;
            }
            _ => reopened.health = AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown),
        }
        Ok(reopened)
    }
}

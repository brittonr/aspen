use super::*;

pub(super) fn aggregate(
    ledger: &ReplicaEvidenceLedger,
    state: &ReplicaState,
    production_admitted: bool,
) -> Result<ReplicaAggregateHealthEvidence> {
    let status = if state.lifecycle == ReplicaLifecycle::Stopped {
        "stopped"
    } else if ledger.saturated || ledger.diagnostic.is_some() || !state.pending_reads.is_empty() {
        "degraded"
    } else {
        "healthy"
    };
    let selected_refs = crate::preserves_rail::sequence(
        ledger.records.iter().map(|record| crate::preserves_rail::string(&record.evidence_ref)).collect(),
    );
    let diagnostic_ref = ledger
        .diagnostic
        .as_ref()
        .map(|diagnostic| crate::preserves_rail::content_ref_from_bytes(diagnostic.as_bytes()));
    let diagnostic_value = diagnostic_ref.as_deref().map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |reference| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)]),
    );
    let evidence_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "raft-aggregate-health-evidence-v1",
        vec![
            crate::preserves_rail::string(&ledger.group_binding_ref),
            crate::preserves_rail::u64_value(ledger.service_generation),
            crate::preserves_rail::string(&ledger.node_id),
            crate::preserves_rail::string(status),
            crate::preserves_rail::u64_value(state.current_term),
            crate::preserves_rail::u64_value(state.commit_index),
            crate::preserves_rail::u64_value(state.last_applied),
            crate::preserves_rail::u64_value(
                u64::try_from(ledger.records.len())
                    .map_err(|_| MoltenError::invalid_harness("live Raft evidence record count exceeds u64"))?,
            ),
            crate::preserves_rail::u64_value(ledger.suppressed_heartbeat_count),
            crate::preserves_rail::bool_value(ledger.saturated),
            diagnostic_value,
            selected_refs,
            crate::preserves_rail::bool_value(production_admitted),
        ],
    ))?;
    Ok(ReplicaAggregateHealthEvidence {
        status: status.to_string(),
        selected_record_count: ledger.records.len(),
        suppressed_heartbeat_count: ledger.suppressed_heartbeat_count,
        saturated: ledger.saturated,
        diagnostic: ledger.diagnostic.clone(),
        evidence_ref,
        production_admitted,
    })
}

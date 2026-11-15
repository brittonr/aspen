use super::*;

pub(super) fn input(
    directory: &std::path::Path,
) -> crate::error::Result<Option<crate::fabric_simulation::bounded_client::Input>> {
    let path = directory.join(crate::fabric_simulation::bounded_client::CLIENT_INPUT_FILE);
    if !path.try_exists().map_err(crate::error::Failure::from)? {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(crate::error::Failure::from)?;
    crate::fabric_simulation::bounded_client::decode_input(&bytes).map(Some)
}

pub(super) fn sample(
    directory: &std::path::Path,
    node: &crate::fabric_consistency::raft::live_cluster::Node,
    endpoint: &str,
    phase: &str,
) -> crate::error::Result<()> {
    let Some(input) = input(directory)? else {
        return Ok(());
    };
    let state = node.service.state();
    let handler = node.service.ports().application.handler();
    let request = super::super::tests::test_ref(REQUEST_LABEL);
    let count = handler.applied_request_refs.iter().filter(|reference| **reference == request).count();
    let observation = from_node(&state.node_id, endpoint, node)?;
    let owner_ref = crate::preserves_rail::canonical_hash(&value(&observation))?;
    write_value(&directory.join(format!("{}-client-{phase}-owner.preserves", state.node_id)), &value(&observation))?;
    let health = node.service.ports().durability.adapter().health();
    let sample = crate::preserves_rail::record("molten-client-native-sample-v1", vec![
        crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(&input)?),
        crate::preserves_rail::string(&state.node_id),
        crate::preserves_rail::string(&request),
        crate::preserves_rail::u64_value(count as u64),
        crate::preserves_rail::string(owner_ref),
        crate::preserves_rail::string(health_name(health)),
        crate::preserves_rail::bool_value(state.completed_requests.contains_key(&request)),
        crate::preserves_rail::u64_value(state.current_term),
        // Local restore provenance is observed before peer ingress. Voting
        // admission is a separate recovery protocol, not the source of bytes.
        crate::preserves_rail::string(
            if observation.request_completed
                && (observation.application_applied || observation.application_restored)
                && (observation.durable_record_count > 0 || observation.durable_snapshot_count > 0)
                && !observation.recovery_ref.is_empty()
            {
                "local"
            } else if observation.durable_record_count == 0 && observation.durable_snapshot_count == 0 {
                "absent"
            } else {
                "unavailable"
            },
        ),
        crate::preserves_rail::string(&observation.recovery_ref),
    ]);
    write_value(&directory.join(format!("{}-client-{phase}.preserves", state.node_id)), &sample)
}

/// Intentional test-only semantic defect: bypass the apply-once adapter guard
/// and execute the actual handler twice after the first admitted commit. The
/// ordinary service implementation and its guard are not changed.
pub(super) fn duplicate_if_requested(
    node: &mut crate::fabric_consistency::raft::live_cluster::Node,
    directory: &std::path::Path,
) -> crate::error::Result<()> {
    let Some(input) = input(directory)? else {
        return Ok(());
    };
    if input.control != crate::fabric_simulation::bounded_client::SemanticControl::DuplicateApplication {
        return Ok(());
    }
    let marker = directory.join("client-mutant-reached.preserves");
    if marker.is_file() {
        return Ok(());
    }
    let request = super::super::tests::test_ref(REQUEST_LABEL);
    let entry = node
        .service
        .state()
        .log
        .iter()
        .find(|entry| entry.request_ref == request)
        .ok_or_else(|| crate::error::Failure::invalid_harness("mutant admitted entry unavailable"))?;
    let command = Command {
        request_ref: entry.request_ref.clone(),
        command_ref: entry.command_ref.clone(),
        command_schema_ref: entry.command_schema_ref.clone(),
    };
    let receipt = node.service.ports_mut().application.handler_mut().apply_batch(&[command])?;
    let count = node
        .service
        .ports()
        .application
        .handler()
        .applied_request_refs
        .iter()
        .filter(|reference| **reference == request)
        .count();
    if count > 1 {
        write_value(
            &marker,
            &crate::preserves_rail::record("molten-client-mutant-reached-v1", vec![
                crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(&input)?),
                crate::preserves_rail::string(crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY),
                crate::preserves_rail::string(receipt),
                crate::preserves_rail::u64_value(count as u64),
            ]),
        )?;
    }
    Ok(())
}

pub(super) struct RecoveryHorizon {
    start_term: u64,
    interval_start: Option<(u64, u64)>,
    is_complete: bool,
}

impl RecoveryHorizon {
    pub(super) fn new(start_term: u64) -> Self {
        Self {
            start_term,
            interval_start: None,
            is_complete: false,
        }
    }

    pub(super) fn observe(
        &mut self,
        directory: &std::path::Path,
        node_id: &str,
        step: u64,
        term: u64,
        input: &crate::fabric_simulation::bounded_client::Input,
    ) -> crate::error::Result<()> {
        if self.is_complete {
            return Ok(());
        }
        if term <= self.start_term || directory.join(PARTITION_FILE).is_file() {
            self.interval_start = None;
            return Ok(());
        }
        // The first advanced, fault-off observation starts the interval; only
        // subsequent iterations count toward the requested recovery duration.
        let (first_step, first_term) = *self.interval_start.get_or_insert((step, term));
        if step - first_step != input.recovery_ticks {
            return Ok(());
        }
        let path = directory.join(format!("{node_id}-client-horizon.preserves"));
        write_value(
            &path,
            &crate::preserves_rail::record("molten-client-native-horizon-v2", vec![
                crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(input)?),
                crate::preserves_rail::string(node_id),
                crate::preserves_rail::u64_value(self.start_term),
                crate::preserves_rail::u64_value(first_step),
                crate::preserves_rail::u64_value(step),
                crate::preserves_rail::u64_value(first_term),
                crate::preserves_rail::u64_value(term),
                crate::preserves_rail::bool_value(true),
            ]),
        )?;
        self.is_complete = true;
        Ok(())
    }
}

fn health_name(health: crate::fabric_durability::AdapterHealth) -> &'static str {
    match health {
        crate::fabric_durability::AdapterHealth::Serving => "serving",
        crate::fabric_durability::AdapterHealth::Quarantined(reason) => match reason {
            crate::fabric_durability::QuarantineReason::OutcomeUnknown => "outcome-unknown",
            crate::fabric_durability::QuarantineReason::CorruptOrInconsistent => "corrupt-or-inconsistent",
            crate::fabric_durability::QuarantineReason::RepairedOnOpen => "repaired-on-open",
            crate::fabric_durability::QuarantineReason::ReconciliationMismatch => "reconciliation-mismatch",
            crate::fabric_durability::QuarantineReason::ReferenceUnavailable => "reference-unavailable",
        },
    }
}

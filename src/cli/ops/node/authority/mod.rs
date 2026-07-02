pub(crate) fn control_request(input: super::command::authority::Request) -> molten::error::Result<()> {
    let super::command::authority::Request {
        operation,
        out,
        target,
        payload,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs,
    } = input;
    let value = molten::node_runtime::node_control_request_value(&molten::node_runtime::ControlRequestValueInput {
        operation: &operation,
        target_ref: target.as_deref(),
        payload_ref: payload.as_deref(),
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &evidence_refs,
    })?;
    super::core::write_file(&out, &molten::preserves_rail::to_text(&value)?)?;
    println!(
        "node control request {} written to {}",
        molten::preserves_rail::canonical_hash(&value)?,
        out.display()
    );
    Ok(())
}

pub(crate) fn provenance_fixture(input: super::command::authority::Provenance) -> molten::error::Result<()> {
    let value = molten::provenance::synthetic_reviewed_provenance_record(&input.artifact_ref)?;
    super::core::write_file(&input.out, &molten::preserves_rail::to_text(&value)?)?;
    println!(
        "node provenance fixture {} written to {}",
        molten::preserves_rail::canonical_hash(&value)?,
        input.out.display()
    );
    Ok(())
}

pub(crate) fn grant_fixture(input: super::command::authority::GrantFixture) -> molten::error::Result<()> {
    let super::command::authority::GrantFixture {
        state_root,
        peer,
        node,
        operations,
        target_scope,
        resource_scope,
        epoch,
        expires_at,
        policy_refs,
        revocation_refs,
        evidence_refs,
        out,
    } = input;
    let operations = if operations.is_empty() {
        vec!["status".to_string()]
    } else {
        operations
    };
    let value =
        molten::node_daemon::node_control_authority_grant_value(&molten::node_daemon::ControlAuthorityGrantInput {
            peer_id: &peer,
            node_id: &node,
            operations: &operations,
            target_scope: &target_scope,
            resource_scope: &resource_scope,
            epoch,
            expires_at,
            policy_refs: &policy_refs,
            revocation_refs: &revocation_refs,
            evidence_refs: &evidence_refs,
        })?;
    let grant_ref = molten::preserves_rail::canonical_hash(&value)?;
    super::core::write_file(&out, &molten::preserves_rail::to_text(&value)?)?;
    if let Some(state_root) = state_root.as_ref() {
        molten::node_daemon::import_node_control_authority_grant(state_root, &value)?;
    }
    println!("node authority grant {} written to {}", grant_ref, out.display());
    Ok(())
}

pub(crate) fn grant_import(input: super::command::authority::GrantImport) -> molten::error::Result<()> {
    let super::command::authority::GrantImport {
        state_root,
        grant,
        peer,
        node,
        operations,
        target_scope,
        resource_scope,
        as_of_epoch,
        receipt_out,
    } = input;
    let grant_value = super::core::read_preserves_file(&grant)?;
    let imported = molten::node_daemon::import_node_control_authority_grant_checked(
        &molten::node_daemon::ControlAuthorityGrantImportInput {
            state_root: &state_root,
            grant_value: &grant_value,
            expected_peer: peer.as_deref(),
            expected_node: node.as_deref(),
            expected_operations: &operations,
            expected_target_scope: target_scope.as_deref(),
            expected_resource_scope: resource_scope.as_deref(),
            as_of_epoch,
        },
    )?;
    super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node authority grant import receipt",
        &imported.receipt_value,
    )?;
    println!(
        "node authority grant import decision={} grant={} imported={} diagnostics={}",
        imported.decision,
        imported.grant_ref,
        imported.imported_refs.len(),
        imported.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn policy_fixture(input: super::command::authority::PolicyFixture) -> molten::error::Result<()> {
    let super::command::authority::PolicyFixture {
        state_root,
        max_restarts,
        restart_window_ticks,
        heartbeat_timeout_ticks,
        shutdown_drain_ticks,
        allow_stale_lock_recovery,
        policy_refs,
        evidence_refs,
        out,
    } = input;
    let value = molten::node_daemon::node_control_supervisor_policy_value(
        &molten::node_daemon::ControlSupervisorPolicyInput {
            max_restarts,
            restart_window_ticks,
            heartbeat_timeout_ticks,
            shutdown_drain_ticks,
            stale_lock_recovery: allow_stale_lock_recovery,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
        },
    )?;
    let policy_ref = molten::preserves_rail::canonical_hash(&value)?;
    super::core::write_file(&out, &molten::preserves_rail::to_text(&value)?)?;
    if let Some(state_root) = state_root.as_ref() {
        molten::node_daemon::import_node_control_supervisor_policy(state_root, &value)?;
    }
    println!("node supervisor policy {} written to {}", policy_ref, out.display());
    Ok(())
}

pub(crate) fn ticket_export(input: super::command::authority::TicketExport) -> molten::error::Result<()> {
    let super::command::authority::TicketExport {
        state_root,
        topic,
        policy_refs,
        evidence_refs,
        out,
    } = input;
    let ticket =
        molten::node_daemon::export_node_control_live_ticket(&molten::node_daemon::ControlLiveTicketExportInput {
            state_root: &state_root,
            topic: &topic,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
        })?;
    super::core::write_file(&out, &molten::preserves_rail::to_text(&ticket.value)?)?;
    println!("node live ticket {} written to {}", ticket.ticket_ref, out.display());
    Ok(())
}

pub(crate) fn ticket_import(input: super::command::authority::TicketImport) -> molten::error::Result<()> {
    let super::command::authority::TicketImport {
        state_root,
        ticket,
        peer_admission,
        expected_node,
        expected_topic,
        expected_endpoint,
        expected_peer,
        as_of_sequence,
        receipt_out,
    } = input;
    let ticket_value = super::core::read_preserves_file(&ticket)?;
    let peer_admission_value =
        peer_admission.as_ref().map(|path| super::core::read_preserves_file(path)).transpose()?;
    let imported =
        molten::node_daemon::import_node_control_live_ticket(&molten::node_daemon::ControlLiveTicketImportInput {
            state_root: &state_root,
            ticket_value: &ticket_value,
            peer_admission_value: peer_admission_value.as_ref(),
            expected_node: expected_node.as_deref(),
            expected_topic: expected_topic.as_deref(),
            expected_endpoint: expected_endpoint.as_deref(),
            expected_peer: expected_peer.as_deref(),
            as_of_sequence,
        })?;
    super::core::emit_named_receipt(receipt_out.as_ref(), "node live ticket import receipt", &imported.receipt_value)?;
    println!(
        "node live ticket import decision={} ticket={} admission={} imported={} diagnostics={}",
        imported.decision,
        imported.ticket_ref,
        imported.peer_admission_ref.as_deref().unwrap_or("none"),
        imported.imported_refs.len(),
        imported.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn peer_admit(input: super::command::authority::PeerAdmit) -> molten::error::Result<()> {
    let super::command::authority::PeerAdmit {
        state_root,
        peer,
        sequence,
        expires_at,
        policy_refs,
        evidence_refs,
        receipt_out,
        ticket,
    } = input;
    let ticket_value = super::core::read_preserves_file(&ticket)?;
    let admission =
        molten::node_daemon::admit_node_control_live_peer(&molten::node_daemon::ControlLivePeerAdmitInput {
            state_root: &state_root,
            ticket_value: &ticket_value,
            peer_id: &peer,
            sequence,
            expires_at,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
        })?;
    super::core::emit_named_receipt(receipt_out.as_ref(), "node live peer admission", &admission.value)?;
    println!(
        "node live peer admit decision={} admission={} peer={} node={} topic={}",
        admission.decision, admission.admission_ref, admission.peer_id, admission.node_id, admission.topic
    );
    Ok(())
}

type Command = super::command::Command;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

mod readiness;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        command @ Command::EvidenceExport { .. } => evidence_export(command),
        command @ Command::Durability { .. } => durability(command),
        command @ Command::FaultCase { .. } => fault_case(command),
        command @ Command::ResourceEnvelope { .. } => resource_envelope(command),
        command @ Command::FaultMatrix { .. } => fault_matrix(command),
        command @ Command::RunReceipt { .. } => run_receipt(command),
        command @ Command::DeploymentProfile { .. }
        | command @ Command::BackupRestoreDrill { .. }
        | command @ Command::UpgradeRollbackDrill { .. }
        | command @ Command::ObservabilitySlo { .. }
        | command @ Command::RunbookCheck { .. }
        | command @ Command::ThreatModel { .. }
        | command @ Command::SecurityDrill { .. }
        | command @ Command::RedactionAudit { .. }
        | command @ Command::SupplyChainReview { .. }
        | command @ Command::BoundaryNegativeSuite { .. }
        | command @ Command::IncidentResponseDrill { .. }
        | command @ Command::SecurityReadinessReport { .. }
        | command @ Command::PilotDecision { .. }
        | command @ Command::ReleaseCandidateGate { .. } => readiness(command),
        Command::Show { artifact } => show(artifact),
    }
}

fn evidence_export(command: Command) -> Outcome<()> {
    let Command::EvidenceExport {
        node,
        node_evidence,
        artifacts,
        logs,
        out,
    } = command
    else {
        return Err(wrong_handler("evidence-export"));
    };
    let node_evidence_ref = super::io::preserves_file_ref(&node_evidence)?;
    let artifact_refs = super::io::preserves_file_refs(&artifacts)?;
    let log_refs = super::io::raw_file_refs(&logs)?;
    let value = molten::prod_soak::evidence_export_value(&molten::prod_soak::EvidenceExportInput {
        node: &node,
        node_evidence_ref: &node_evidence_ref,
        artifact_refs: &artifact_refs,
        log_refs: &log_refs,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(out.as_ref(), &value, &format!("prod-soak evidence-export ref={reference} node={node}"))
}

fn durability(command: Command) -> Outcome<()> {
    let Command::Durability {
        scenario,
        queued_control_refs,
        recovery_refs,
        ledger_refs,
        chunk_refs,
        retention_refs,
        decision,
        diagnostics,
        caveats,
        out,
    } = command
    else {
        return Err(wrong_handler("durability"));
    };
    let value = molten::prod_soak::durability_value(&molten::prod_soak::DurabilityInput {
        decision: &decision,
        scenario: &scenario,
        queued_control_refs: &queued_control_refs,
        recovery_refs: &recovery_refs,
        ledger_refs: &ledger_refs,
        chunk_refs: &chunk_refs,
        retention_refs: &retention_refs,
        diagnostics: &diagnostics,
        caveats: &caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(
        out.as_ref(),
        &value,
        &format!("prod-soak durability ref={reference} decision={decision} scenario={scenario}"),
    )
}

fn fault_case(command: Command) -> Outcome<()> {
    let Command::FaultCase {
        scenario,
        fault_kind,
        injection,
        expected_outcome,
        evidence_refs,
        denial_refs,
        decision,
        replay_status,
        diagnostics,
        caveats,
        out,
    } = command
    else {
        return Err(wrong_handler("fault-case"));
    };
    let value = molten::prod_soak::fault_case_value(&molten::prod_soak::FaultCaseInput {
        decision: &decision,
        scenario: &scenario,
        fault_kind: &fault_kind,
        injection: &injection,
        expected_outcome: &expected_outcome,
        evidence_refs: &evidence_refs,
        denial_refs: &denial_refs,
        replay_status: &replay_status,
        diagnostics: &diagnostics,
        caveats: &caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(
        out.as_ref(),
        &value,
        &format!("prod-soak fault-case ref={reference} decision={decision} fault={fault_kind}"),
    )
}

fn resource_envelope(command: Command) -> Outcome<()> {
    let Command::ResourceEnvelope {
        scenario,
        queue_depth,
        max_queue_depth,
        receipt_bytes,
        max_receipt_bytes,
        store_bytes,
        max_store_bytes,
        delivery_latency_ms,
        max_delivery_latency_ms,
        recovery_time_ms,
        max_recovery_time_ms,
        pressure_refs,
        denial_refs,
        decision,
        diagnostics,
        caveats,
        out,
    } = command
    else {
        return Err(wrong_handler("resource-envelope"));
    };
    let value = molten::prod_soak::resource_envelope_value(&molten::prod_soak::ResourceEnvelopeInput {
        decision: &decision,
        scenario: &scenario,
        queue_depth,
        max_queue_depth,
        receipt_bytes,
        max_receipt_bytes,
        store_bytes,
        max_store_bytes,
        delivery_latency_ms,
        max_delivery_latency_ms,
        recovery_time_ms,
        max_recovery_time_ms,
        pressure_refs: &pressure_refs,
        denial_refs: &denial_refs,
        diagnostics: &diagnostics,
        caveats: &caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(
        out.as_ref(),
        &value,
        &format!(
            "prod-soak resource-envelope ref={reference} decision={decision} queue={queue_depth}/{max_queue_depth}"
        ),
    )
}

fn fault_matrix(command: Command) -> Outcome<()> {
    let Command::FaultMatrix {
        scenario,
        fault_cases,
        fault_kinds,
        decision,
        diagnostics,
        caveats,
        out,
    } = command
    else {
        return Err(wrong_handler("fault-matrix"));
    };
    let fault_case_refs = super::io::preserves_file_refs(&fault_cases)?;
    let value = molten::prod_soak::fault_matrix_value(&molten::prod_soak::FaultMatrixInput {
        decision: &decision,
        scenario: &scenario,
        fault_case_refs: &fault_case_refs,
        fault_kinds: &fault_kinds,
        diagnostics: &diagnostics,
        caveats: &caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(
        out.as_ref(),
        &value,
        &format!("prod-soak fault-matrix ref={reference} decision={decision} faults={}", fault_kinds.len()),
    )
}

fn run_receipt(command: Command) -> Outcome<()> {
    let Command::RunReceipt {
        topology,
        node_evidence,
        scenario,
        fault_profile,
        peer_ticket_refs,
        control_refs,
        remote_service_refs,
        job_refs,
        coordination_refs,
        evidence_exports,
        fault_refs,
        durability_refs,
        resource_refs,
        decision,
        replay_status,
        diagnostics,
        logs,
        caveats,
        out,
    } = command
    else {
        return Err(wrong_handler("run"));
    };
    let topology_ref = super::io::preserves_file_ref(&topology)?;
    let node_evidence_refs = super::io::preserves_file_refs(&node_evidence)?;
    let evidence_export_refs = super::io::preserves_file_refs(&evidence_exports)?;
    let log_refs = super::io::raw_file_refs(&logs)?;
    let value = molten::prod_soak::run_value(&molten::prod_soak::RunInput {
        decision: &decision,
        scenario: &scenario,
        topology_ref: &topology_ref,
        fault_profile: &fault_profile,
        node_evidence_refs: &node_evidence_refs,
        peer_ticket_refs: &peer_ticket_refs,
        control_refs: &control_refs,
        remote_service_refs: &remote_service_refs,
        job_refs: &job_refs,
        coordination_refs: &coordination_refs,
        evidence_export_refs: &evidence_export_refs,
        fault_refs: &fault_refs,
        durability_refs: &durability_refs,
        resource_refs: &resource_refs,
        replay_status: &replay_status,
        diagnostics: &diagnostics,
        log_refs: &log_refs,
        caveats: &caveats,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(
        out.as_ref(),
        &value,
        &format!("prod-soak run ref={reference} decision={decision} scenario={scenario}"),
    )
}

fn readiness(command: Command) -> Outcome<()> {
    readiness::run(command)
}

fn emit_value(out: Option<&FilePath>, value: &preserves::IOValue, summary: &str) -> Outcome<()> {
    let is_written_to_file = super::io::write_optional_preserves(out, value)?;
    super::io::print_or_log_summary(is_written_to_file, summary);
    Ok(())
}

fn show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let rendered = molten::preserves_rail::to_text(&value)?;
    let kind = super::command::artifact_kind(&rendered);
    println!("prod-soak {kind} ref={reference} path={}", artifact.display());
    Ok(())
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("prod-soak {name} handler called with another command"))
}

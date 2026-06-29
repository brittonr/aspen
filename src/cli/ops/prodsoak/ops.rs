type Command = super::command::Command;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

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
    let value = molten::prod_soak::evidence_export_value(&molten::prod_soak::ProdSoakEvidenceExportInput {
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
    let value = molten::prod_soak::durability_value(&molten::prod_soak::ProdSoakDurabilityInput {
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
    let value = molten::prod_soak::fault_case_value(&molten::prod_soak::ProdSoakFaultCaseInput {
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
    let value = molten::prod_soak::resource_envelope_value(&molten::prod_soak::ProdSoakResourceEnvelopeInput {
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
    let value = molten::prod_soak::fault_matrix_value(&molten::prod_soak::ProdSoakFaultMatrixInput {
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
        node_control_refs,
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
    let value = molten::prod_soak::run_value(&molten::prod_soak::ProdSoakRunInput {
        decision: &decision,
        scenario: &scenario,
        topology_ref: &topology_ref,
        fault_profile: &fault_profile,
        node_evidence_refs: &node_evidence_refs,
        peer_ticket_refs: &peer_ticket_refs,
        node_control_refs: &node_control_refs,
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
    let (value, out, kind, subject, decision) = match command {
        Command::DeploymentProfile {
            decision,
            profile_name,
            state_layout_refs,
            required_adapter_refs,
            source_gate_refs,
            resource_limit_refs,
            redaction_setting_refs,
            live_transport_refs,
            startup_expectation_refs,
            shutdown_expectation_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::deployment_profile_value(&molten::prod_readiness::DeploymentProfileInput {
                decision: &decision,
                profile_name: &profile_name,
                state_layout_refs: &state_layout_refs,
                required_adapter_refs: &required_adapter_refs,
                source_gate_refs: &source_gate_refs,
                resource_limit_refs: &resource_limit_refs,
                redaction_setting_refs: &redaction_setting_refs,
                live_transport_refs: &live_transport_refs,
                startup_expectation_refs: &startup_expectation_refs,
                shutdown_expectation_refs: &shutdown_expectation_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "deployment-profile",
            profile_name,
            decision,
        ),
        Command::BackupRestoreDrill {
            decision,
            drill_name,
            ledger_refs,
            redb_refs,
            chunk_refs,
            identity_refs,
            retention_pin_refs,
            source_gate_refs,
            restore_verification_refs,
            tamper_denial_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::backup_restore_drill_value(&molten::prod_readiness::BackupRestoreDrillInput {
                decision: &decision,
                drill_name: &drill_name,
                ledger_refs: &ledger_refs,
                redb_refs: &redb_refs,
                chunk_refs: &chunk_refs,
                identity_refs: &identity_refs,
                retention_pin_refs: &retention_pin_refs,
                source_gate_refs: &source_gate_refs,
                restore_verification_refs: &restore_verification_refs,
                tamper_denial_refs: &tamper_denial_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "backup-restore-drill",
            drill_name,
            decision,
        ),
        Command::UpgradeRollbackDrill {
            decision,
            plan_name,
            migration_refs,
            smoke_refs,
            rollback_eligibility_refs,
            irreversible_exclusion_refs,
            post_rollback_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::upgrade_rollback_drill_value(&molten::prod_readiness::UpgradeRollbackDrillInput {
                decision: &decision,
                plan_name: &plan_name,
                migration_refs: &migration_refs,
                smoke_refs: &smoke_refs,
                rollback_eligibility_refs: &rollback_eligibility_refs,
                irreversible_exclusion_refs: &irreversible_exclusion_refs,
                post_rollback_refs: &post_rollback_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "upgrade-rollback-drill",
            plan_name,
            decision,
        ),
        Command::ObservabilitySlo {
            decision,
            snapshot_name,
            adapter_health_refs,
            queue_depth,
            max_queue_depth,
            control_loop_refs,
            resource_pressure_refs,
            retention_drift_refs,
            source_gate_freshness_refs,
            live_transport_refs,
            import_export_failure_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::observability_slo_value(&molten::prod_readiness::ObservabilitySloInput {
                decision: &decision,
                snapshot_name: &snapshot_name,
                adapter_health_refs: &adapter_health_refs,
                queue_depth,
                max_queue_depth,
                control_loop_refs: &control_loop_refs,
                resource_pressure_refs: &resource_pressure_refs,
                retention_drift_refs: &retention_drift_refs,
                source_gate_freshness_refs: &source_gate_freshness_refs,
                live_transport_refs: &live_transport_refs,
                import_export_failure_refs: &import_export_failure_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "observability-slo",
            snapshot_name,
            decision,
        ),
        Command::RunbookCheck {
            decision,
            runbook_name,
            operation,
            canonical_artifact_refs,
            denial_fixture_refs,
            auxiliary_log_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::runbook_check_value(&molten::prod_readiness::RunbookCheckInput {
                decision: &decision,
                runbook_name: &runbook_name,
                operation: &operation,
                canonical_artifact_refs: &canonical_artifact_refs,
                denial_fixture_refs: &denial_fixture_refs,
                auxiliary_log_refs: &auxiliary_log_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "runbook-check",
            runbook_name,
            decision,
        ),
        Command::ThreatModel {
            decision,
            model_name,
            threat_entries,
            mapped_gate_refs,
            drill_refs,
            negative_suite_refs,
            unresolved_risk_refs,
            pilot_consequence_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::threat_model_value(&molten::prod_readiness::ThreatModelInput {
                decision: &decision,
                model_name: &model_name,
                threat_entries: &threat_entries,
                mapped_gate_refs: &mapped_gate_refs,
                drill_refs: &drill_refs,
                negative_suite_refs: &negative_suite_refs,
                unresolved_risk_refs: &unresolved_risk_refs,
                pilot_consequence_refs: &pilot_consequence_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "threat-model",
            model_name,
            decision,
        ),
        Command::SecurityDrill {
            decision,
            drill_kind,
            scenario,
            pass_evidence_refs,
            denial_refs,
            cleanup_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::security_drill_value(&molten::prod_readiness::SecurityDrillInput {
                decision: &decision,
                drill_kind: &drill_kind,
                scenario: &scenario,
                pass_evidence_refs: &pass_evidence_refs,
                denial_refs: &denial_refs,
                cleanup_refs: &cleanup_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "security-drill",
            drill_kind,
            decision,
        ),
        Command::RedactionAudit {
            decision,
            audit_name,
            surface_refs,
            redaction_refs,
            reveal_gate_refs,
            plaintext_denial_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::redaction_audit_value(&molten::prod_readiness::RedactionAuditInput {
                decision: &decision,
                audit_name: &audit_name,
                surface_refs: &surface_refs,
                redaction_refs: &redaction_refs,
                reveal_gate_refs: &reveal_gate_refs,
                plaintext_denial_refs: &plaintext_denial_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "redaction-audit",
            audit_name,
            decision,
        ),
        Command::SupplyChainReview {
            decision,
            review_name,
            release_refs,
            source_gate_refs,
            provenance_refs,
            build_verify_refs,
            signed_keyring_refs,
            sensitive_artifact_refs,
            mismatch_denial_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::supply_chain_review_value(&molten::prod_readiness::SupplyChainReviewInput {
                decision: &decision,
                review_name: &review_name,
                release_refs: &release_refs,
                source_gate_refs: &source_gate_refs,
                provenance_refs: &provenance_refs,
                build_verify_refs: &build_verify_refs,
                signed_keyring_refs: &signed_keyring_refs,
                sensitive_artifact_refs: &sensitive_artifact_refs,
                mismatch_denial_refs: &mismatch_denial_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "supply-chain-review",
            review_name,
            decision,
        ),
        Command::BoundaryNegativeSuite {
            decision,
            suite_name,
            preserves_parser_refs,
            receipt_validator_refs,
            source_gate_refs,
            repro_bundle_refs,
            node_ingress_refs,
            provenance_refs,
            plugin_hostcall_refs,
            malformed_denial_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::boundary_negative_suite_value(
                &molten::prod_readiness::BoundaryNegativeSuiteInput {
                    decision: &decision,
                    suite_name: &suite_name,
                    preserves_parser_refs: &preserves_parser_refs,
                    receipt_validator_refs: &receipt_validator_refs,
                    source_gate_refs: &source_gate_refs,
                    repro_bundle_refs: &repro_bundle_refs,
                    node_ingress_refs: &node_ingress_refs,
                    provenance_refs: &provenance_refs,
                    plugin_hostcall_refs: &plugin_hostcall_refs,
                    malformed_denial_refs: &malformed_denial_refs,
                    diagnostics: &diagnostics,
                },
            )?,
            out,
            "boundary-negative-suite",
            suite_name,
            decision,
        ),
        Command::IncidentResponseDrill {
            decision,
            incident_kind,
            scenario,
            detection_refs,
            containment_refs,
            recovery_refs,
            next_step_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::incident_response_drill_value(
                &molten::prod_readiness::IncidentResponseDrillInput {
                    decision: &decision,
                    incident_kind: &incident_kind,
                    scenario: &scenario,
                    detection_refs: &detection_refs,
                    containment_refs: &containment_refs,
                    recovery_refs: &recovery_refs,
                    next_step_refs: &next_step_refs,
                    diagnostics: &diagnostics,
                },
            )?,
            out,
            "incident-response-drill",
            incident_kind,
            decision,
        ),
        Command::SecurityReadinessReport {
            decision,
            report_name,
            threat_model_refs,
            supply_chain_refs,
            drill_refs,
            redaction_audit_refs,
            boundary_suite_refs,
            incident_response_refs,
            unresolved_risk_refs,
            pilot_recommendation,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::security_readiness_report_value(
                &molten::prod_readiness::SecurityReadinessReportInput {
                    decision: &decision,
                    report_name: &report_name,
                    threat_model_refs: &threat_model_refs,
                    supply_chain_refs: &supply_chain_refs,
                    drill_refs: &drill_refs,
                    redaction_audit_refs: &redaction_audit_refs,
                    boundary_suite_refs: &boundary_suite_refs,
                    incident_response_refs: &incident_response_refs,
                    unresolved_risk_refs: &unresolved_risk_refs,
                    pilot_recommendation: &pilot_recommendation,
                    diagnostics: &diagnostics,
                },
            )?,
            out,
            "security-readiness-report",
            report_name,
            decision,
        ),
        Command::PilotDecision {
            decision,
            scope,
            allowed_workloads,
            denied_workloads,
            rollback_triggers,
            stop_conditions,
            operator_review_refs,
            caveats,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::pilot_decision_value(&molten::prod_readiness::PilotDecisionInput {
                decision: &decision,
                scope: &scope,
                allowed_workloads: &allowed_workloads,
                denied_workloads: &denied_workloads,
                rollback_triggers: &rollback_triggers,
                stop_conditions: &stop_conditions,
                operator_review_refs: &operator_review_refs,
                caveats: &caveats,
                diagnostics: &diagnostics,
            })?,
            out,
            "pilot-decision",
            scope,
            decision,
        ),
        Command::ReleaseCandidateGate {
            decision,
            candidate,
            source_ref,
            rust_validation_refs,
            nextest_refs,
            nix_check_refs,
            cairn_validation_refs,
            octet_refs,
            dogfood_refs,
            bundle_verify_refs,
            promotion_refs,
            export_verify_refs,
            source_gate_status,
            source_gate_caveats,
            pilot_decision_refs,
            diagnostics,
            out,
        } => (
            molten::prod_readiness::release_candidate_gate_value(&molten::prod_readiness::ReleaseCandidateGateInput {
                decision: &decision,
                candidate: &candidate,
                source_ref: &source_ref,
                rust_validation_refs: &rust_validation_refs,
                nextest_refs: &nextest_refs,
                nix_check_refs: &nix_check_refs,
                cairn_validation_refs: &cairn_validation_refs,
                octet_refs: &octet_refs,
                dogfood_refs: &dogfood_refs,
                bundle_verify_refs: &bundle_verify_refs,
                promotion_refs: &promotion_refs,
                export_verify_refs: &export_verify_refs,
                source_gate_status: &source_gate_status,
                source_gate_caveats: &source_gate_caveats,
                pilot_decision_refs: &pilot_decision_refs,
                diagnostics: &diagnostics,
            })?,
            out,
            "release-candidate-gate",
            candidate,
            decision,
        ),
        _ => return Err(wrong_handler("readiness")),
    };
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    emit_value(
        out.as_ref(),
        &value,
        &format!("prod-readiness {kind} ref={reference} decision={decision} subject={subject}"),
    )
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

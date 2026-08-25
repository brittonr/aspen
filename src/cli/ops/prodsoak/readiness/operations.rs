pub(super) fn deployment_profile(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::DeploymentProfile {
        decision,
        profile_name,
        schema_id,
        schema_version,
        source_language,
        profile_identity,
        profile_ref,
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
    } = command
    else {
        return Err(super::super::wrong_handler("deployment-profile"));
    };
    let profile_identity = profile_identity.unwrap_or_else(|| profile_name.clone());
    Ok(super::Emission {
        value: molten::prod_readiness::deployment_profile_value(&molten::prod_readiness::DeploymentProfileInput {
            decision: &decision,
            profile_name: &profile_name,
            schema_id: &schema_id,
            schema_version,
            source_language: &source_language,
            profile_identity: &profile_identity,
            profile_ref: &profile_ref,
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
        kind: "deployment-profile",
        subject: profile_name,
        decision,
    })
}

pub(super) fn backup_restore_drill(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::BackupRestoreDrill {
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
    } = command
    else {
        return Err(super::super::wrong_handler("backup-restore-drill"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::backup_restore_drill_value(&molten::prod_readiness::BackupRestoreDrillInput {
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
        kind: "backup-restore-drill",
        subject: drill_name,
        decision,
    })
}

pub(super) fn upgrade_rollback_drill(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::UpgradeRollbackDrill {
        decision,
        plan_name,
        migration_refs,
        smoke_refs,
        rollback_eligibility_refs,
        irreversible_exclusion_refs,
        post_rollback_refs,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("upgrade-rollback-drill"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::upgrade_rollback_drill_value(
            &molten::prod_readiness::UpgradeRollbackDrillInput {
                decision: &decision,
                plan_name: &plan_name,
                migration_refs: &migration_refs,
                smoke_refs: &smoke_refs,
                rollback_eligibility_refs: &rollback_eligibility_refs,
                irreversible_exclusion_refs: &irreversible_exclusion_refs,
                post_rollback_refs: &post_rollback_refs,
                diagnostics: &diagnostics,
            },
        )?,
        out,
        kind: "upgrade-rollback-drill",
        subject: plan_name,
        decision,
    })
}

pub(super) fn observability_slo(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::ObservabilitySlo {
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
    } = command
    else {
        return Err(super::super::wrong_handler("observability-slo"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::observability_slo_value(&molten::prod_readiness::ObservabilitySloInput {
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
        kind: "observability-slo",
        subject: snapshot_name,
        decision,
    })
}

pub(super) fn runbook_check(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::RunbookCheck {
        decision,
        runbook_name,
        operation,
        canonical_artifact_refs,
        denial_fixture_refs,
        auxiliary_log_refs,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("runbook-check"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::runbook_check_value(&molten::prod_readiness::RunbookCheckInput {
            decision: &decision,
            runbook_name: &runbook_name,
            operation: &operation,
            canonical_artifact_refs: &canonical_artifact_refs,
            denial_fixture_refs: &denial_fixture_refs,
            auxiliary_log_refs: &auxiliary_log_refs,
            diagnostics: &diagnostics,
        })?,
        out,
        kind: "runbook-check",
        subject: runbook_name,
        decision,
    })
}

pub(super) fn pilot_decision(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::PilotDecision {
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
    } = command
    else {
        return Err(super::super::wrong_handler("pilot-decision"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::pilot_decision_value(&molten::prod_readiness::PilotDecisionInput {
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
        kind: "pilot-decision",
        subject: scope,
        decision,
    })
}

pub(super) fn release_candidate_gate(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::ReleaseCandidateGate {
        decision,
        candidate,
        source_ref,
        rust_validation_bindings,
        nextest_bindings,
        nix_check_bindings,
        cairn_validation_bindings,
        octet_bindings,
        dogfood_bindings,
        bundle_verify_bindings,
        promotion_bindings,
        export_verify_bindings,
        source_gate_status,
        source_gate_caveats,
        pilot_decision_bindings,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("release-candidate-gate"));
    };
    let rust_validation_bindings = parse_candidate_evidence_bindings("Rust validation", rust_validation_bindings)?;
    let nextest_bindings = parse_candidate_evidence_bindings("nextest", nextest_bindings)?;
    let nix_check_bindings = parse_candidate_evidence_bindings("Nix check", nix_check_bindings)?;
    let cairn_validation_bindings = parse_candidate_evidence_bindings("Cairn validation", cairn_validation_bindings)?;
    let octet_bindings = parse_candidate_evidence_bindings("Octet", octet_bindings)?;
    let dogfood_bindings = parse_candidate_evidence_bindings("dogfood", dogfood_bindings)?;
    let bundle_verify_bindings = parse_candidate_evidence_bindings("release bundle verify", bundle_verify_bindings)?;
    let promotion_bindings = parse_candidate_evidence_bindings("promotion", promotion_bindings)?;
    let export_verify_bindings = parse_candidate_evidence_bindings("export verify", export_verify_bindings)?;
    let pilot_decision_bindings = parse_candidate_evidence_bindings("pilot decision", pilot_decision_bindings)?;
    let rust_validation_evidence = borrow_candidate_evidence_bindings(&rust_validation_bindings);
    let nextest_evidence = borrow_candidate_evidence_bindings(&nextest_bindings);
    let nix_check_evidence = borrow_candidate_evidence_bindings(&nix_check_bindings);
    let cairn_validation_evidence = borrow_candidate_evidence_bindings(&cairn_validation_bindings);
    let octet_evidence = borrow_candidate_evidence_bindings(&octet_bindings);
    let dogfood_evidence = borrow_candidate_evidence_bindings(&dogfood_bindings);
    let bundle_verify_evidence = borrow_candidate_evidence_bindings(&bundle_verify_bindings);
    let promotion_evidence = borrow_candidate_evidence_bindings(&promotion_bindings);
    let export_verify_evidence = borrow_candidate_evidence_bindings(&export_verify_bindings);
    let pilot_decision_evidence = borrow_candidate_evidence_bindings(&pilot_decision_bindings);
    Ok(super::Emission {
        value: molten::prod_readiness::release_candidate_gate_value(
            &molten::prod_readiness::ReleaseCandidateGateInput {
                decision: &decision,
                candidate: &candidate,
                source_ref: &source_ref,
                rust_validation_evidence: &rust_validation_evidence,
                nextest_evidence: &nextest_evidence,
                nix_check_evidence: &nix_check_evidence,
                cairn_validation_evidence: &cairn_validation_evidence,
                octet_evidence: &octet_evidence,
                dogfood_evidence: &dogfood_evidence,
                bundle_verify_evidence: &bundle_verify_evidence,
                promotion_evidence: &promotion_evidence,
                export_verify_evidence: &export_verify_evidence,
                source_gate_status: &source_gate_status,
                source_gate_caveats: &source_gate_caveats,
                pilot_decision_evidence: &pilot_decision_evidence,
                diagnostics: &diagnostics,
            },
        )?,
        out,
        kind: "release-candidate-gate",
        subject: candidate,
        decision,
    })
}

struct OwnedCandidateEvidenceBinding {
    artifact_ref: String,
    source_ref: String,
}

fn parse_candidate_evidence_bindings(
    label: &'static str,
    values: Vec<String>,
) -> super::super::Outcome<Vec<OwnedCandidateEvidenceBinding>> {
    const BINDING_SEPARATOR: char = '@';
    values
        .into_iter()
        .map(|value| {
            let Some((artifact_ref, source_ref)) = value.split_once(BINDING_SEPARATOR) else {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "production readiness {label} binding must use ARTIFACT_REF@SOURCE_REF"
                )));
            };
            if artifact_ref.trim().is_empty() || source_ref.trim().is_empty() {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "production readiness {label} binding members must not be empty"
                )));
            }
            Ok(OwnedCandidateEvidenceBinding {
                artifact_ref: artifact_ref.to_string(),
                source_ref: source_ref.to_string(),
            })
        })
        .collect()
}

fn borrow_candidate_evidence_bindings(
    bindings: &[OwnedCandidateEvidenceBinding],
) -> Vec<molten::prod_readiness::CandidateEvidenceBinding<'_>> {
    bindings
        .iter()
        .map(|binding| molten::prod_readiness::CandidateEvidenceBinding {
            artifact_ref: &binding.artifact_ref,
            source_ref: &binding.source_ref,
        })
        .collect()
}

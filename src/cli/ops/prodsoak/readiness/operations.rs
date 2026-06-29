pub(super) fn deployment_profile(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::DeploymentProfile {
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
    } = command
    else {
        return Err(super::super::wrong_handler("deployment-profile"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::deployment_profile_value(&molten::prod_readiness::DeploymentProfileInput {
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
    } = command
    else {
        return Err(super::super::wrong_handler("release-candidate-gate"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::release_candidate_gate_value(
            &molten::prod_readiness::ReleaseCandidateGateInput {
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
            },
        )?,
        out,
        kind: "release-candidate-gate",
        subject: candidate,
        decision,
    })
}

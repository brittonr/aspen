pub(super) fn threat_model(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::ThreatModel {
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
    } = command
    else {
        return Err(super::super::wrong_handler("threat-model"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::threat_model_value(&molten::prod_readiness::ThreatModelInput {
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
        kind: "threat-model",
        subject: model_name,
        decision,
    })
}

pub(super) fn drill(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::SecurityDrill {
        decision,
        drill_kind,
        scenario,
        pass_evidence_refs,
        denial_refs,
        cleanup_refs,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("security-drill"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::security_drill_value(&molten::prod_readiness::SecurityDrillInput {
            decision: &decision,
            drill_kind: &drill_kind,
            scenario: &scenario,
            pass_evidence_refs: &pass_evidence_refs,
            denial_refs: &denial_refs,
            cleanup_refs: &cleanup_refs,
            diagnostics: &diagnostics,
        })?,
        out,
        kind: "security-drill",
        subject: drill_kind,
        decision,
    })
}

pub(super) fn redaction_audit(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::RedactionAudit {
        decision,
        audit_name,
        surface_refs,
        redaction_refs,
        reveal_gate_refs,
        plaintext_denial_refs,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("redaction-audit"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::redaction_audit_value(&molten::prod_readiness::RedactionAuditInput {
            decision: &decision,
            audit_name: &audit_name,
            surface_refs: &surface_refs,
            redaction_refs: &redaction_refs,
            reveal_gate_refs: &reveal_gate_refs,
            plaintext_denial_refs: &plaintext_denial_refs,
            diagnostics: &diagnostics,
        })?,
        out,
        kind: "redaction-audit",
        subject: audit_name,
        decision,
    })
}

pub(super) fn supply_chain_review(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::SupplyChainReview {
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
    } = command
    else {
        return Err(super::super::wrong_handler("supply-chain-review"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::supply_chain_review_value(&molten::prod_readiness::SupplyChainReviewInput {
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
        kind: "supply-chain-review",
        subject: review_name,
        decision,
    })
}

pub(super) fn boundary_negative_suite(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::BoundaryNegativeSuite {
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
    } = command
    else {
        return Err(super::super::wrong_handler("boundary-negative-suite"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::boundary_negative_suite_value(
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
        kind: "boundary-negative-suite",
        subject: suite_name,
        decision,
    })
}

pub(super) fn incident_response_drill(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::IncidentResponseDrill {
        decision,
        incident_kind,
        scenario,
        detection_refs,
        containment_refs,
        recovery_refs,
        next_step_refs,
        diagnostics,
        out,
    } = command
    else {
        return Err(super::super::wrong_handler("incident-response-drill"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::incident_response_drill_value(
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
        kind: "incident-response-drill",
        subject: incident_kind,
        decision,
    })
}

pub(super) fn report(command: super::super::Command) -> super::super::Outcome<super::Emission> {
    let super::super::Command::SecurityReadinessReport {
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
    } = command
    else {
        return Err(super::super::wrong_handler("security-readiness-report"));
    };
    Ok(super::Emission {
        value: molten::prod_readiness::security_readiness_report_value(
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
        kind: "security-readiness-report",
        subject: report_name,
        decision,
    })
}

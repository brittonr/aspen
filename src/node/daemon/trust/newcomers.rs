//! Live peer bootstrap checks and transport-evidence caveats.

pub(super) fn evaluate_live_peer_bootstrap(
    state_root: &crate::node_state::DirectoryRoot,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.peer_bootstrap_refs.len().saturating_add(1));
    let mut admitted_peer_ref = None;
    for peer_ref in envelope.peer_bootstrap_refs.iter() {
        match super::storage::read_ledger_artifact(state_root, peer_ref) {
            Ok(value) => match super::peering::parse_control_live_peer_admission(&value) {
                Ok(admission) => {
                    let admission_diagnostics = live_peer_admission_diagnostics(state_root, envelope, &admission)?;
                    if admission_diagnostics.is_empty() {
                        admitted_peer_ref = Some(admission.admission_ref);
                        break;
                    }
                    diagnostics.extend(admission_diagnostics);
                }
                Err(error) => {
                    if let Some(diagnostic) = transport_evidence_not_authority_diagnostic(
                        &value,
                        peer_ref,
                        "node control live peer bootstrap ref",
                        "bootstrap authority",
                    ) {
                        diagnostics.push(diagnostic);
                    } else {
                        diagnostics.push(format!(
                            "node control live peer bootstrap ref {peer_ref} is not an admission: {error}"
                        ));
                    }
                }
            },
            Err(error) => diagnostics.push(format!("node control live peer bootstrap {peer_ref} not found: {error}")),
        }
    }
    if admitted_peer_ref.is_none() {
        diagnostics.push("node control live peer bootstrap missing admitted ticket".to_string());
    }
    Ok(diagnostics)
}

fn live_peer_admission_diagnostics(
    state_root: &crate::node_state::DirectoryRoot,
    envelope: &super::transits::ControlIngressEnvelope,
    admission: &super::entitlements::ControlLivePeerAdmission,
) -> crate::error::Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.peer_id != envelope.from_peer {
        diagnostics.push(format!(
            "node control live peer admission {} peer {} does not match {}",
            admission.admission_ref, admission.peer_id, envelope.from_peer
        ));
    }
    if admission.node_id != envelope.to_node {
        diagnostics.push(format!(
            "node control live peer admission {} node {} does not match {}",
            admission.admission_ref, admission.node_id, envelope.to_node
        ));
    }
    if admission.topic != envelope.topic {
        diagnostics.push(format!(
            "node control live peer admission {} topic {} does not match {}",
            admission.admission_ref, admission.topic, envelope.topic
        ));
    }
    match super::storage::read_ledger_artifact(state_root, &admission.ticket_ref) {
        Ok(value) => match super::issuance::parse_control_live_ticket(&value) {
            Ok(ticket) => {
                if ticket.node_id != admission.node_id || ticket.topic != admission.topic {
                    diagnostics.push(format!(
                        "node control live peer admission {} ticket binding mismatch",
                        admission.admission_ref
                    ));
                }
            }
            Err(error) => diagnostics.push(format!(
                "node control live peer admission {} ticket is not a live ticket: {error}",
                admission.admission_ref
            )),
        },
        Err(error) => diagnostics.push(format!(
            "node control live peer admission {} ticket {} not found: {error}",
            admission.admission_ref, admission.ticket_ref
        )),
    }
    if admission.sequence > envelope.sequence {
        diagnostics.push(format!(
            "node control live peer admission {} is not valid until sequence {}",
            admission.admission_ref, admission.sequence
        ));
    }
    if let Some(expires_at) = admission.expires_at
        && expires_at < envelope.sequence
    {
        diagnostics.push(format!(
            "node control live peer admission {} expired at sequence {expires_at}",
            admission.admission_ref
        ));
    }
    Ok(diagnostics)
}

pub(super) fn transport_evidence_not_authority_diagnostic(
    value: &preserves::IOValue,
    reference: &str,
    reference_label: &str,
    authority_label: &str,
) -> Option<String> {
    let kind = crate::ledger::artifact_kind(value);
    is_transport_observation_kind(kind)
        .then(|| format!("{reference_label} {reference} is transport evidence, not {authority_label}"))
}

fn is_transport_observation_kind(kind: &str) -> bool {
    matches!(
        kind,
        "node-control-live-transport-receipt"
            | "node-control-live-listener-receipt"
            | "node-control-live-send-receipt"
            | "node-control-live-send-retry-receipt"
            | "node-control-live-send-duplicate-receipt"
    )
}

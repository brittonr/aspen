
fn evaluate_live_peer_bootstrap(
    state_root: &crate::node_state::NodeStateRoot,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.peer_bootstrap_refs.len().saturating_add(1));
    let mut admitted_peer_ref = None;
    for peer_ref in envelope.peer_bootstrap_refs.iter() {
        match read_ledger_artifact(state_root, peer_ref) {
            Ok(value) => match parse_control_live_peer_admission(&value) {
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

// r[impl molten.federated_pull_runtime.status_evidence]
pub fn session_value(session: &molten_core::federation::Session) -> crate::error::Result<preserves::IOValue> {
    let lengths = [
        session.plan.missing.len(),
        session.plan.conflicts.len(),
        session.verified.len(),
        session.admitted.len(),
        session.committed.len(),
        session.evidence_refs.len(),
    ];
    if lengths.iter().any(|length| *length > molten_core::federation::MAX_FEDERATION_ENTRIES) {
        return Err(crate::error::Failure::invalid_harness("federation status exceeds retained bounds"));
    }
    let phase = match session.phase {
        molten_core::federation::SessionPhase::Planned => "planned",
        molten_core::federation::SessionPhase::Fetching => "fetching",
        molten_core::federation::SessionPhase::Verified => "verified",
        molten_core::federation::SessionPhase::Admitted => "admitted",
        molten_core::federation::SessionPhase::Committed => "committed",
        molten_core::federation::SessionPhase::Denied => "denied",
    };
    Ok(crate::preserves_rail::record("federation-status-v1", vec![
        crate::preserves_rail::string(&session.session_ref),
        crate::preserves_rail::u64_value(session.generation),
        crate::preserves_rail::string(&session.plan.peer_ref),
        crate::preserves_rail::string(phase),
        crate::preserves_rail::u64_value(session.plan.deadline),
        crate::preserves_rail::string(&session.plan.signer_ref),
        crate::preserves_rail::u64_value(session.plan.inventory_issued_at),
        crate::preserves_rail::u64_value(session.plan.inventory_expires_at),
        crate::preserves_rail::sequence(
            session.plan.missing.iter().map(|entry| crate::preserves_rail::string(&entry.head_ref)).collect(),
        ),
        refs(&session.verified),
        refs(&session.admitted),
        refs(&session.committed),
        crate::preserves_rail::sequence(
            session
                .plan
                .conflicts
                .iter()
                .map(|conflict| {
                    crate::preserves_rail::record("federation-conflict-v1", vec![
                        crate::preserves_rail::string(&conflict.local_ref),
                        crate::preserves_rail::string(&conflict.remote.head_ref),
                        crate::preserves_rail::string(&conflict.remote.ancestry_ref),
                        crate::preserves_rail::string(&conflict.remote.policy_ref),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::u64_value(session.plan.bytes),
        crate::preserves_rail::u64_value(session.plan.dag_nodes),
        crate::preserves_rail::u64_value(session.plan.chunks),
        refs(&session.evidence_refs),
        crate::preserves_rail::string("local bounded observation; no permanent convergence or remote correctness"),
    ]))
}

// r[impl molten.federated_pull_runtime.status_evidence]
pub fn peers_value(peers: &[molten_core::federation::PeerHint], now: u64) -> crate::error::Result<preserves::IOValue> {
    if peers.len() > molten_core::federation::MAX_FEDERATION_PEERS {
        return Err(crate::error::Failure::invalid_harness("federation peer status exceeds bound"));
    }
    Ok(crate::preserves_rail::record("federation-peers-v1", vec![crate::preserves_rail::sequence(
        peers
            .iter()
            .map(|peer| {
                crate::preserves_rail::record("candidate-peer", vec![
                    crate::preserves_rail::string(&peer.peer_ref),
                    crate::preserves_rail::string(&peer.signer_ref),
                    crate::preserves_rail::string(if peer.observed_at <= now && now <= peer.expires_at {
                        "fresh"
                    } else {
                        "stale"
                    }),
                    crate::preserves_rail::u64_value(peer.observed_at),
                    crate::preserves_rail::u64_value(peer.expires_at),
                    crate::preserves_rail::string(if peer.available {
                        "observed-available"
                    } else {
                        "unavailable"
                    }),
                    crate::preserves_rail::string("hint-only; no import, merge, install, pin, or execution authority"),
                ])
            })
            .collect(),
    )]))
}
fn refs(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

/// Closed diagnostic vocabulary: never persist adapter errors, payload bytes, endpoints or secrets.
pub fn terminal_denial(
    subject: &str,
    generation: u64,
    reason: &str,
    now: u64,
) -> crate::error::Result<preserves::IOValue> {
    crate::preserves_rail::validate_content_ref(subject)?;
    if !matches!(
        reason,
        "deadline"
            | "inventory-rejected"
            | "generation-fenced"
            | "resource-exhausted"
            | "corrupt-content"
            | "signature-rejected"
            | "unsolicited-push"
            | "conflict"
            | "partition"
    ) {
        return Err(crate::error::Failure::invalid_harness("unknown federation denial class"));
    }
    Ok(crate::preserves_rail::record("federation-terminal-denial-v1", vec![
        crate::preserves_rail::string(subject),
        crate::preserves_rail::u64_value(generation),
        crate::preserves_rail::string(reason),
        crate::preserves_rail::u64_value(now),
        crate::preserves_rail::string("local terminal observation; no authority or remote correctness claim"),
    ]))
}

/// Local dataspace assertion only; applying it grants no remote or import authority.
pub fn session_assertion(
    manifest: &crate::mesh::admission::Manifest,
    session: &molten_core::federation::Session,
) -> crate::error::Result<crate::runtime::RuntimeAssertion> {
    if manifest.generation() != session.generation {
        return Err(crate::error::Failure::invalid_harness("federation status assertion generation is fenced"));
    }
    Ok(crate::runtime::RuntimeAssertion {
        actor: manifest.extension().manifest().service_id.clone(),
        value: crate::runtime::RuntimeValue::new(session_value(session)?)?,
        facet: None,
    })
}

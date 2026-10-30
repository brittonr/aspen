pub mod rules;

pub struct ManifestInput {
    pub extension: crate::system_extension::AdmittedManifest,
    pub generation: u64,
    pub peers: Vec<molten_core::federation::PeerHint>,
    pub bounds: molten_core::federation::PullBounds,
    pub dag_binding_ref: String,
    pub peer_configuration_ref: String,
    pub local_admission_policy_ref: String,
    pub resource_profile_ref: String,
    pub rate_profile_ref: String,
    pub evidence_profile_ref: String,
}

pub struct Manifest {
    input: ManifestInput,
    manifest_ref: String,
    value: preserves::IOValue,
}
impl Manifest {
    pub fn extension(&self) -> &crate::system_extension::AdmittedManifest {
        &self.input.extension
    }
    pub fn generation(&self) -> u64 {
        self.input.generation
    }
    pub fn peers(&self) -> &[molten_core::federation::PeerHint] {
        &self.input.peers
    }
    pub fn bounds(&self) -> &molten_core::federation::PullBounds {
        &self.input.bounds
    }
    pub fn manifest_ref(&self) -> &str {
        &self.manifest_ref
    }
    pub fn value(&self) -> &preserves::IOValue {
        &self.value
    }
    pub fn dag_binding_ref(&self) -> &str {
        &self.input.dag_binding_ref
    }
    pub fn local_policy_ref(&self) -> &str {
        &self.input.local_admission_policy_ref
    }
    pub fn resource_profile_ref(&self) -> &str {
        &self.input.resource_profile_ref
    }
}

// r[impl molten.federated_pull_runtime.manifest]
pub fn admit_manifest(input: ManifestInput) -> crate::error::Result<Manifest> {
    if input.generation == 0
        || input.peers.len() > molten_core::federation::MAX_FEDERATION_PEERS
        || input.bounds.inventory_entries == 0
        || input.bounds.inventory_entries > molten_core::federation::MAX_FEDERATION_ENTRIES
        || input.bounds.retained_status == 0
        || input.bounds.retained_status > molten_core::federation::MAX_FEDERATION_ENTRIES
        || input.bounds.concurrent_sessions == 0
        || input.bounds.concurrent_sessions > molten_core::federation::MAX_FEDERATION_PEERS
        || input.bounds.freshness == 0
        || input.bounds.query_interval == 0
        || input.bounds.session_ticks == 0
    {
        return Err(invalid("federation manifest has invalid generation or bounds"));
    }
    validate_bindings(&input)?;
    for reference in [
        &input.peer_configuration_ref,
        &input.local_admission_policy_ref,
        &input.resource_profile_ref,
        &input.rate_profile_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
        if !input.extension.manifest().policy_refs.contains(reference) {
            return Err(invalid("federation profile is not bound by the admitted extension policy"));
        }
    }
    if input.evidence_profile_ref != input.extension.manifest().evidence_profile_ref {
        return Err(invalid("federation evidence profile differs from admitted extension evidence profile"));
    }
    for (index, peer) in input.peers.iter().enumerate() {
        crate::preserves_rail::validate_content_ref(&peer.peer_ref)?;
        crate::preserves_rail::validate_content_ref(&peer.signer_ref)?;
        if peer.source != molten_core::federation::LocatorSource::Static
            || peer.observed_at > peer.expires_at
            || input.peers[..index].iter().any(|prior| prior.peer_ref == peer.peer_ref)
        {
            return Err(invalid("static peer configuration malformed or duplicated"));
        }
    }
    if input.peer_configuration_ref != peer_configuration_ref(&input.peers)?
        || input.rate_profile_ref != rate_profile_ref(&input.bounds)?
    {
        return Err(invalid("federation peer or bounded-rate inputs differ from their reviewed canonical profiles"));
    }
    let value = manifest_value(&input);
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Manifest {
        input,
        manifest_ref,
        value,
    })
}

/// Requires every fabric port and the DAG binding the federation core consumes to be
/// owner-admitted.
fn validate_bindings(input: &ManifestInput) -> crate::error::Result<()> {
    let required = [
        crate::fabric_crypto_identity::FABRIC_CRYPTO_SIGNATURE_PORT_ID,
        crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID,
        "content-store",
        crate::fabric_time::FABRIC_CLOCK_PORT_ID,
        crate::fabric_time::FABRIC_TIMER_PORT_ID,
        crate::fabric_observability::FABRIC_OBSERVATION_PORT_ID,
    ];
    for port in required {
        if !input.extension.required_port_bindings().iter().any(|binding| binding.binding.key.port_id == port) {
            return Err(invalid("federation manifest missing a required admitted fabric binding"));
        }
    }
    if !input.extension.required_port_bindings().iter().any(|binding| {
        binding.binding_ref == input.dag_binding_ref
            && binding.binding.key.port_id == crate::dag_sync::DAG_SYNC_PORT_ID
            && binding.binding.key.version == crate::dag_sync::DAG_SYNC_PORT_VERSION
    }) {
        return Err(invalid("federation requires an owner-admitted DAG binding"));
    }
    Ok(())
}

fn manifest_value(input: &ManifestInput) -> preserves::IOValue {
    crate::preserves_rail::record("federated-pull-manifest-v1", vec![
        crate::preserves_rail::string(input.extension.manifest_ref()),
        crate::preserves_rail::string(&input.extension.manifest().service_id),
        crate::preserves_rail::u64_value(input.generation),
        crate::preserves_rail::string("federation-pull-v1"),
        crate::preserves_rail::sequence(
            input.extension.all_binding_refs().map(crate::preserves_rail::string).collect(),
        ),
        crate::preserves_rail::sequence(
            [
                &input.dag_binding_ref,
                &input.peer_configuration_ref,
                &input.local_admission_policy_ref,
                &input.resource_profile_ref,
                &input.rate_profile_ref,
                &input.evidence_profile_ref,
            ]
            .iter()
            .map(crate::preserves_rail::string)
            .collect(),
        ),
        crate::preserves_rail::sequence(
            input
                .peers
                .iter()
                .map(|peer| {
                    crate::preserves_rail::record("static-peer", vec![
                        crate::preserves_rail::string(&peer.peer_ref),
                        crate::preserves_rail::string(&peer.signer_ref),
                        crate::preserves_rail::u64_value(peer.observed_at),
                        crate::preserves_rail::u64_value(peer.expires_at),
                        crate::preserves_rail::bool_value(peer.available),
                    ])
                })
                .collect(),
        ),
        bounds_value(&input.bounds),
        crate::preserves_rail::sequence(
            [
                "hint-only discovery",
                "no unsolicited import",
                "no automatic merge",
                "no permanent convergence",
                "no remote correctness",
            ]
            .iter()
            .map(crate::preserves_rail::string)
            .collect(),
        ),
    ])
}
fn bounds_value(bounds: &molten_core::federation::PullBounds) -> preserves::IOValue {
    crate::preserves_rail::record("federation-bounds-v1", vec![
        crate::preserves_rail::string(bounds.inventory_entries.to_string()),
        crate::preserves_rail::u64_value(bounds.inventory_bytes),
        crate::preserves_rail::u64_value(bounds.fetch_bytes),
        crate::preserves_rail::u64_value(bounds.dag_nodes),
        crate::preserves_rail::u64_value(bounds.content_chunks),
        crate::preserves_rail::string(bounds.concurrent_sessions.to_string()),
        crate::preserves_rail::u64_value(bounds.query_interval),
        crate::preserves_rail::u64_value(bounds.freshness),
        crate::preserves_rail::u64_value(bounds.session_ticks),
        crate::preserves_rail::u64_value(u64::from(bounds.retries)),
        crate::preserves_rail::string(bounds.retained_status.to_string()),
    ])
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

pub fn peer_configuration_ref(peers: &[molten_core::federation::PeerHint]) -> crate::error::Result<String> {
    if peers.len() > molten_core::federation::MAX_FEDERATION_PEERS {
        return Err(invalid("static peer configuration exceeds bound"));
    }
    for peer in peers {
        crate::preserves_rail::validate_content_ref(&peer.peer_ref)?;
        crate::preserves_rail::validate_content_ref(&peer.signer_ref)?;
        if peer.source != molten_core::federation::LocatorSource::Static {
            return Err(invalid("configured peer must be static"));
        }
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("federation-static-peer-config-v1", vec![
        crate::preserves_rail::sequence(
            peers
                .iter()
                .map(|peer| {
                    crate::preserves_rail::record("static-peer", vec![
                        crate::preserves_rail::string(&peer.peer_ref),
                        crate::preserves_rail::string(&peer.signer_ref),
                    ])
                })
                .collect(),
        ),
    ]))
}

pub fn rate_profile_ref(bounds: &molten_core::federation::PullBounds) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&bounds_value(bounds))
}

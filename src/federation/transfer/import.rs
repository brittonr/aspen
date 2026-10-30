#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Schema,
    Policy,
    Capability,
    Provenance,
    Resource,
    Retention,
    Import,
}
const GATES: [Gate; 7] = [
    Gate::Schema,
    Gate::Policy,
    Gate::Capability,
    Gate::Provenance,
    Gate::Resource,
    Gate::Retention,
    Gate::Import,
];

pub struct LocalSubject<'a> {
    pub session_ref: &'a str,
    pub generation: u64,
    pub now: u64,
    pub head_ref: &'a str,
    pub artifact: &'a preserves::IOValue,
}

pub struct LocalGateObservation {
    pub gate: Gate,
    pub session_ref: String,
    pub generation: u64,
    pub head_ref: String,
    pub admitted: bool,
    pub evidence_ref: String,
}

/// Implemented by the owning local artifact/application extension, never the peer.
pub trait LocalImportAdmissionPort {
    fn admit(&mut self, gate: Gate, subject: &LocalSubject<'_>) -> crate::error::Result<LocalGateObservation>;
}

pub struct Input<'a> {
    pub session: &'a molten_core::federation::Session,
    pub bounds: &'a molten_core::federation::PullBounds,
    pub generation: u64,
    pub now: u64,
    pub head_ref: &'a str,
    pub manifest_ref: &'a str,
    pub dag: &'a crate::dag_sync::DagSyncOutcome,
}

pub struct Ports<'a> {
    pub admission: &'a mut dyn LocalImportAdmissionPort,
    pub quarantine: &'a crate::chunk_store::CapabilityChunkRoot,
    pub registry: &'a crate::ledger::CapabilityLedgerRoot,
    pub durable: &'a crate::node_state::Scope,
}

pub struct Outcome {
    pub session: molten_core::federation::Session,
    pub receipt_ref: String,
    pub receipt: preserves::IOValue,
}

/// Checks receiver-plan correlation and reads the quarantined artifact matching the planned head.
fn verified_artifact(
    input: &Input<'_>,
    quarantine: &crate::chunk_store::CapabilityChunkRoot,
) -> crate::error::Result<preserves::IOValue> {
    if input.dag.receipt.decision != molten_core::dag_sync::DagSyncDecision::Complete
        || input.dag.plan.generation != input.generation
        || input.dag.plan.epoch_ref.as_str() != input.session.session_ref
        || !input.dag.progress.verified.iter().any(|object| object.as_str() == input.head_ref)
        || !input.session.plan.missing.iter().any(|entry| entry.head_ref == input.head_ref)
    {
        return Err(invalid("federation import requires a complete correlated receiver DAG plan"));
    }
    let selected = input
        .session
        .plan
        .missing
        .iter()
        .find(|entry| entry.head_ref == input.head_ref)
        .ok_or_else(|| invalid("unplanned local import"))?;
    let manifest = crate::chunk_store::read_manifest_with_root(quarantine, input.manifest_ref)?;
    if manifest.total_len != selected.bytes || manifest.total_len > input.bounds.fetch_bytes {
        return Err(invalid("quarantined object exceeds receiver-declared import byte budget"));
    }
    let read = crate::chunk_store::read_object_with_root(quarantine, input.manifest_ref)?;
    let artifact = crate::preserves_rail::parse_canonical_bytes(&read.bytes)?;
    if crate::preserves_rail::canonical_hash(&artifact)? != input.head_ref {
        return Err(invalid("federation import identity differs from receiver plan"));
    }
    Ok(artifact)
}

// r[impl molten.federated_pull_runtime.receiver_pull]
pub fn commit_verified(input: &Input<'_>, ports: Ports<'_>) -> crate::error::Result<Outcome> {
    let artifact = verified_artifact(input, ports.quarantine)?;
    let verified = molten_core::federation::advance_session(
        input.session,
        &molten_core::federation::SessionEvent::Verified {
            head_ref: input.head_ref.to_string(),
            evidence_ref: input.dag.canonical_receipt.record_ref.clone(),
        },
        input.generation,
        input.now,
        input.bounds,
    )
    .map_err(session_error)?;
    let subject = LocalSubject {
        session_ref: &verified.session_ref,
        generation: input.generation,
        now: input.now,
        head_ref: input.head_ref,
        artifact: &artifact,
    };
    let prerequisites = run_gates(ports.admission, &subject)?;
    let admitted = molten_core::federation::advance_session(
        &verified,
        &molten_core::federation::SessionEvent::Admitted {
            head_ref: input.head_ref.to_string(),
            prerequisites,
        },
        input.generation,
        input.now,
        input.bounds,
    )
    .map_err(session_error)?;
    let receipt = phase_record("federation-import-complete-v1", input, &admitted.evidence_refs);
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt)?;
    let session = molten_core::federation::advance_session(
        &admitted,
        &molten_core::federation::SessionEvent::Committed {
            head_ref: input.head_ref.to_string(),
            evidence_ref: receipt_ref.clone(),
        },
        input.generation,
        input.now,
        input.bounds,
    )
    .map_err(session_error)?;
    let intent = phase_record("federation-import-intent-v1", input, &admitted.evidence_refs);
    persist(ports.durable, input.head_ref, "intent", &intent)?;
    crate::ledger::import_artifact_with_root(ports.registry, &artifact).map_err(|error| {
        invalid(&format!("federation import effect failed after durable intent; outcome requires readback: {error}"))
    })?;
    persist(ports.durable, input.head_ref, "complete", &receipt).map_err(|error| {
        invalid(&format!("federation import committed but completion persistence is uncertain: {error}"))
    })?;
    Ok(Outcome {
        session,
        receipt_ref,
        receipt,
    })
}

fn run_gates(
    port: &mut dyn LocalImportAdmissionPort,
    subject: &LocalSubject<'_>,
) -> crate::error::Result<molten_core::federation::ImportPrerequisites> {
    let mut refs = Vec::with_capacity(GATES.len());
    for gate in GATES {
        let observation = port.admit(gate, subject)?;
        if !observation.admitted
            || observation.gate != gate
            || observation.session_ref != subject.session_ref
            || observation.generation != subject.generation
            || observation.head_ref != subject.head_ref
        {
            return Err(invalid("local federation admission gate denied or context drifted"));
        }
        crate::preserves_rail::validate_content_ref(&observation.evidence_ref)?;
        refs.push(observation.evidence_ref);
    }
    Ok(molten_core::federation::ImportPrerequisites {
        schema_ref: refs[0].clone(),
        policy_ref: refs[1].clone(),
        capability_ref: refs[2].clone(),
        provenance_ref: refs[3].clone(),
        resource_ref: refs[4].clone(),
        retention_ref: refs[5].clone(),
        import_ref: refs[6].clone(),
    })
}

fn phase_record(label: &'static str, input: &Input<'_>, evidence: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![
        crate::preserves_rail::string(&input.session.session_ref),
        crate::preserves_rail::u64_value(input.generation),
        crate::preserves_rail::string(input.head_ref),
        crate::preserves_rail::string(&input.dag.canonical_receipt.record_ref),
        crate::preserves_rail::sequence(evidence.iter().map(crate::preserves_rail::string).collect()),
    ])
}
fn persist(
    scope: &crate::node_state::Scope,
    head: &str,
    phase: &str,
    value: &preserves::IOValue,
) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(head)?;
    let path = crate::node_state::RelativePath::parse(&format!("federation-{}-{phase}.preserves", &head[7..]))?;
    scope.write(&path, &crate::preserves_rail::canonical_bytes(value)?)
}
fn session_error(issue: molten_core::federation::SessionIssue) -> crate::error::Failure {
    invalid(&format!("federation session transition denied: {issue:?}"))
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
